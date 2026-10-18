"""CLI for the Razer BlackShark V2 HyperSpeed HID protocol.

Examples:
  python tools/razerctl.py status
  python tools/razerctl.py battery
  python tools/razerctl.py sidetone off
  python tools/razerctl.py eq-preset 7
  python tools/razerctl.py watch --count 5
"""

from __future__ import annotations

import argparse
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from razer_hid import RazerHeadset  # noqa: E402


def dump(obj: object) -> None:
    print(json.dumps(obj, ensure_ascii=False, indent=2, default=lambda o: list(o) if isinstance(o, (bytes, bytearray)) else str(o)))


def main() -> int:
    p = argparse.ArgumentParser(description="Razer BlackShark V2 HyperSpeed HID client")
    p.add_argument("--pid", type=lambda v: int(v, 0), default=0x0565,
                   help="0x0565 wireless dongle (default) or 0x056E wired USB")
    sub = p.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status")
    sub.add_parser("battery")
    sub.add_parser("charging")
    sub.add_parser("auto-off")
    sub.add_parser("speaker-eq")

    st = sub.add_parser("sidetone")
    st.add_argument("state", choices=["on", "off"])

    ep = sub.add_parser("eq-preset")
    ep.add_argument("index", help="device EQ preset index (7/8/9/255) or name (game/music/cinema/custom)")

    eg = sub.add_parser("eq-gains")
    eg.add_argument("gains", help="10 comma-separated values, e.g. 1,-5,-3,0,1,1,1,1,1,-3")

    w = sub.add_parser("watch")
    w.add_argument("--count", type=int, default=5)

    args = p.parse_args()
    with RazerHeadset(pid=args.pid) as hs:
        if args.cmd == "status":
            dump(hs.status())
        elif args.cmd == "battery":
            dump(hs.get("battery"))
        elif args.cmd == "charging":
            dump(hs.get("charging"))
        elif args.cmd == "auto-off":
            dump(hs.get("auto_power_off"))
        elif args.cmd == "speaker-eq":
            dump(hs.get("speaker_preset_eq"))
        elif args.cmd == "sidetone":
            dump(hs.set_sidetone(args.state == "on"))
        elif args.cmd == "eq-preset":
            try:
                index = int(args.index, 0)
            except ValueError:
                dump(hs.set_speaker_eq_preset(args.index))
            else:
                dump(hs.set_preset_eq_index(index))
        elif args.cmd == "eq-gains":
            values = [int(x.strip()) for x in args.gains.split(",")]
            dump(hs.set_eq_gains(values))
        elif args.cmd == "watch":
            hs.watch(args.count)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
