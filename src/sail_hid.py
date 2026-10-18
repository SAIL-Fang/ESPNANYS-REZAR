"""Minimal HID client for the Razer BlackShark V2 HyperSpeed headset.

Tested against:
  - Razer BlackShark V2 HS 2.4 (PID 0x0565, 2.4 GHz dongle)
  - Razer Synapse 4.0.827 / RazerAppEngine 4.0.827

The headset exposes a vendor-defined HID collection on interface 3:
  usage_page = 0xFF14, usage = 1
  input  report id 2, 63-byte payload
  output report id 2, 63-byte payload
  feature report id 2, 8-byte payload (not used by the 64-byte protocol)

All 64-byte frames include the 0x02 report id as byte 0.  The device echoes
the sequence byte, so responses can be correlated.

This module only uses documented observations and never requires Razer Synapse.
It is read/write capable; callers are responsible for restoring device state.
"""

from __future__ import annotations

import json
import time
from typing import Any

import hid

VID = 0x1532
PID_WIRELESS = 0x0565  # 1381
PID_WIRED = 0x056E     # 1390
USAGE_PAGE = 0xFF14
REPORT_ID = 0x02
FRAME_SIZE = 64

# class byte at offset 6/7 in a request
CLASS_GET = 0x04
CLASS_SET1 = 0x05
CLASS_SET10 = 0x0E

COMMANDS: dict[str, dict[str, Any]] = {
    "serial":            {"cmd": 0x00, "sub": 0x00, "kind": "get"},
    "edition":           {"cmd": 0x01, "sub": 0x00, "kind": "get"},
    "firmware":          {"cmd": 0x02, "sub": 0x00, "kind": "get"},
    "speaker_preset_eq": {"cmd": 0x1E, "sub": 0x00, "kind": "get"},
    "battery":           {"cmd": 0x21, "sub": 0x00, "kind": "get"},
    "charging":          {"cmd": 0x2A, "sub": 0x00, "kind": "get"},
    "auto_power_off":    {"cmd": 0x2C, "sub": 0x00, "kind": "get"},
    "dongle_led":        {"cmd": 0x66, "sub": 0x00, "kind": "get", "flags9": 0x00},
    "preset_eq_index":   {"cmd": 0x93, "sub": 0x00, "kind": "set1"},
    "eq_gains":          {"cmd": 0x95, "sub": 0x00, "kind": "set10"},
    "mic_preset_eq":     {"cmd": 0x96, "sub": 0x00, "kind": "set1"},
    "sidetone":          {"cmd": 0x98, "sub": 0x00, "kind": "set1"},
    "sidetone_volume":   {"cmd": 0x99, "sub": 0x00, "kind": "set1"},
    "speaker_eq_status": {"cmd": 0x9E, "sub": 0x00, "kind": "set1"},
}

EVENTS: dict[int, str] = {
    0x20: "wireless_connection_status",
    0x21: "battery_status",
    0x27: "do_not_disturb",
    0x55: "mic_status",
}

# Speaker EQ presets observed in Synapse 4.0.827 (0x93 payload -> curve for 0x95).
SPEAKER_EQ_PRESETS: dict[str, int] = {
    "game": 7,
    "music": 8,
    "cinema": 9,
    "custom": 255,
}

SPEAKER_EQ_GAINS: dict[str, list[int]] = {
    "game": [1, -5, -3, 0, 1, 1, 1, 1, 1, -3],
    "music": [3, 3, 2, 0, 1, 1, 1, 1, 1, -3],
    "cinema": [4, 4, 3, -4, -6, -3, 1, 3, 1, -1],
    "custom": [3, 3, 2, 0, 1, 1, 1, 1, 1, -1],
}


def _checksum(frame: list[int]) -> int:
    x = 0
    for b in frame[2:62]:
        x ^= b
    return (x ^ 0x02) & 0xFF


class RazerHeadset:
    def __init__(self, pid: int = PID_WIRELESS, timeout: float = 1.5) -> None:
        self.pid = pid
        self.timeout = timeout
        self.dev: hid.device | None = None
        self.seq = 0x60
        self.info: dict[str, Any] = {}
        self._open()

    # ------------------------------------------------------------------ open
    def _open(self) -> None:
        candidates = []
        for d in hid.enumerate(VID, self.pid):
            if d["interface_number"] == 3 and d["usage_page"] == USAGE_PAGE:
                candidates.append(d)
        if not candidates:
            raise RuntimeError(
                f"no {USAGE_PAGE:#06x} HID collection for VID {VID:#06x} PID {self.pid:#06x}"
            )
        d = candidates[0]
        dev = hid.device()
        dev.open_path(d["path"])
        dev.set_nonblocking(1)
        self.dev = dev
        self.info = {
            "product": d.get("product_string"),
            "manufacturer": d.get("manufacturer_string"),
            "serial": d.get("serial_number"),
            "path": d["path"].decode(errors="replace"),
            "usage_page": d["usage_page"],
            "usage": d["usage"],
            "interface_number": d["interface_number"],
            "release_number": d.get("release_number"),
        }
        self._drain()

    def _drain(self) -> None:
        assert self.dev is not None
        for _ in range(64):
            try:
                if not self.dev.read(FRAME_SIZE):
                    break
            except Exception:
                break

    def close(self) -> None:
        if self.dev is not None:
            try:
                self.dev.close()
            finally:
                self.dev = None

    def __enter__(self) -> "RazerHeadset":
        return self

    def __exit__(self, *exc: object) -> None:
        self.close()

    # ----------------------------------------------------------------- frame
    def _next_seq(self) -> int:
        self.seq = (self.seq + 1) & 0xFF
        if self.seq == 0:
            self.seq = 1
        return self.seq

    def _frame(
        self,
        cls: int,
        target: int,
        sub: int,
        payload: list[int],
        seq: int,
        flags9: int = 0x80,
    ) -> list[int]:
        f = [0] * FRAME_SIZE
        f[0] = REPORT_ID
        f[1] = 0x00
        f[2] = seq
        f[6] = cls
        f[7] = 0x00
        f[8] = 0x00
        f[9] = flags9
        f[10] = target
        f[11] = sub
        f[12] = len(payload)
        for i, b in enumerate(payload):
            f[13 + i] = b & 0xFF
        f[62] = _checksum(f)
        f[63] = 0x00
        return f

    def _transact(self, frame: list[int], expect_cmd: int) -> dict[str, Any]:
        assert self.dev is not None
        self.dev.write(frame)
        deadline = time.time() + self.timeout
        while time.time() < deadline:
            try:
                resp = self.dev.read(FRAME_SIZE, 100)
            except Exception as exc:
                return {"ok": False, "error": f"read: {exc}"}
            if not resp:
                continue
            resp = list(resp)
            if len(resp) >= 14 and resp[0] == REPORT_ID and resp[1] == 0x02 and resp[10] == expect_cmd:
                length = resp[12]
                return {
                    "ok": resp[11] == 1,
                    "status": resp[11],
                    "seq": resp[2],
                    "cmd": resp[10],
                    "length": length,
                    "data": resp[13:13 + length],
                    "raw": resp,
                }
        return {"ok": False, "error": "timeout", "request": frame}

    # ---------------------------------------------------------------- public
    def get(self, name: str) -> dict[str, Any]:
        spec = COMMANDS[name]
        if spec["kind"] != "get":
            raise ValueError(f"{name} is not a get command")
        frame = self._frame(
            CLASS_GET,
            spec["cmd"],
            spec["sub"],
            [],
            self._next_seq(),
            spec.get("flags9", 0x80),
        )
        result = self._transact(frame, spec["cmd"])
        result["name"] = name
        return result

    def set1(self, name: str, value: int) -> dict[str, Any]:
        spec = COMMANDS[name]
        if spec["kind"] != "set1":
            raise ValueError(f"{name} is not a set1 command")
        frame = self._frame(
            CLASS_SET1, spec["cmd"], spec["sub"], [value & 0xFF], self._next_seq(),
            spec.get("flags9", 0x80),
        )
        result = self._transact(frame, spec["cmd"])
        result["name"] = name
        return result

    def set10(self, name: str, values: list[int]) -> dict[str, Any]:
        spec = COMMANDS[name]
        if spec["kind"] != "set10":
            raise ValueError(f"{name} is not a set10 command")
        if len(values) != 10:
            raise ValueError("expected exactly 10 EQ values")
        frame = self._frame(
            CLASS_SET10, spec["cmd"], spec["sub"], values, self._next_seq(),
            spec.get("flags9", 0x80),
        )
        result = self._transact(frame, spec["cmd"])
        result["name"] = name
        return result

    def status(self) -> dict[str, Any]:
        out: dict[str, Any] = {"device": self.info, "identity": {}, "values": {}}
        for name in ("firmware", "edition", "serial", "dongle_led"):
            r = self.get(name)
            out["identity"][name] = r.get("data") if r.get("ok") else {"error": r.get("error", "failed")}
        for name in ("battery", "charging", "auto_power_off", "speaker_preset_eq"):
            r = self.get(name)
            out["values"][name] = r.get("data") if r.get("ok") else {"error": r.get("error", "failed")}
        fw = out["identity"].get("firmware")
        if isinstance(fw, list) and fw:
            out["firmware_version"] = ".".join(str(x) for x in fw)
        serial = out["identity"].get("serial")
        if isinstance(serial, list) and serial:
            out["serial_ascii"] = bytes(serial).decode("ascii", "replace")
        battery = out["values"].get("battery")
        out["battery_percent"] = battery[0] if isinstance(battery, list) and battery else None
        return out

    def set_sidetone(self, enabled: bool) -> dict[str, Any]:
        return self.set1("sidetone", 1 if enabled else 0)

    def set_preset_eq_index(self, index: int) -> dict[str, Any]:
        return self.set1("preset_eq_index", index)

    def set_eq_gains(self, gains: list[int]) -> dict[str, Any]:
        return self.set10("eq_gains", gains)

    def set_speaker_eq_preset(self, preset: str) -> dict[str, Any]:
        key = preset.lower()
        if key not in SPEAKER_EQ_PRESETS:
            raise ValueError(f"unknown speaker EQ preset: {preset}")
        return self.set1("preset_eq_index", SPEAKER_EQ_PRESETS[key])

    def set_speaker_eq_gains(self, gains: list[int]) -> dict[str, Any]:
        return self.set10("eq_gains", gains)

    def read_event(self, timeout: float = 1.0) -> dict[str, Any] | None:
        assert self.dev is not None
        deadline = time.time() + timeout
        while time.time() < deadline:
            try:
                resp = self.dev.read(FRAME_SIZE, 100)
            except Exception:
                return None
            if not resp:
                continue
            resp = list(resp)
            if len(resp) >= 14 and resp[0] == REPORT_ID and resp[1] in (0x0A, 0x09):
                rid = resp[10]
                length = resp[12]
                return {
                    "kind": "event" if resp[1] == 0x0A else "notification",
                    "record_id": rid,
                    "record": EVENTS.get(rid, f"0x{rid:02x}"),
                    "length": length,
                    "data": resp[13:13 + length],
                    "raw": resp,
                }
        return None

    def watch(self, count: int = 0) -> list[dict[str, Any]]:
        seen: list[dict[str, Any]] = []
        while count <= 0 or len(seen) < count:
            ev = self.read_event(timeout=5.0)
            if ev:
                seen.append(ev)
                print(json.dumps(ev, ensure_ascii=False), flush=True)
        return seen


if __name__ == "__main__":
    with RazerHeadset() as hs:
        print(json.dumps(hs.status(), ensure_ascii=False, indent=2))
