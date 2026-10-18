"""Collect a full read-only snapshot of the headset into captures/device-state.json."""
import json
import pathlib
import sys
import time

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from razer_hid import RazerHeadset  # noqa: E402


def main() -> None:
    out = {"collected_at": time.strftime("%Y-%m-%dT%H:%M:%S%z")}
    with RazerHeadset(timeout=3.0) as hs:
        out["device"] = hs.info
        out["identity"] = {}
        for name in ("firmware", "edition", "serial", "dongle_led"):
            r = hs.get(name)
            out["identity"][name] = {
                "ok": r.get("ok"),
                "status": r.get("status"),
                "data": r.get("data"),
                "raw": r.get("raw"),
            }
            time.sleep(0.2)
        out["status"] = {}
        for name in ("battery", "charging", "auto_power_off", "speaker_preset_eq"):
            r = hs.get(name)
            out["status"][name] = {"ok": r.get("ok"), "data": r.get("data"), "raw": r.get("raw")}
            time.sleep(0.2)
        # decode ASCII serial
        serial = out["identity"].get("serial", {}).get("data")
        if serial:
            out["serial_ascii"] = bytes(serial).decode("ascii", "replace")
        fw = out["identity"].get("firmware", {}).get("data")
        if fw:
            out["firmware_version"] = ".".join(str(x) for x in fw)
    path = ROOT / "captures" / "device-state.json"
    path.write_text(json.dumps(out, ensure_ascii=False, indent=2), encoding="utf-8")
    print(json.dumps(out, ensure_ascii=False, indent=2))


if __name__ == "__main__":
    main()
