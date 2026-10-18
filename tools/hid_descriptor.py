# ESPNANYS-REZAR — open-source Razer BlackShark V2 HyperSpeed control app
# Author: SAIL
# Copyright (c) 2026 SAIL
# SPDX-License-Identifier: MIT
import hid

VID, PID = 0x1532, 0x0565

def parse_descriptor(data):
    out = []
    i = 0
    report_id = 0
    report_size = 0
    report_count = 0
    usage_page = 0
    usage = 0
    while i < len(data):
        prefix = data[i]
        if prefix == 0xFE:  # long item
            size = data[i + 1]
            tag = data[i + 2]
            out.append(("long", tag, data[i + 3:i + 3 + size].hex()))
            i += 3 + size
            continue
        size = [0, 1, 2, 4][prefix & 0x03]
        typ = (prefix >> 2) & 0x03
        tag = prefix >> 4
        val = int.from_bytes(data[i + 1:i + 1 + size], "little") if size else 0
        if typ == 1:  # global
            if tag == 0x0: usage_page = val
            elif tag == 0x7: report_size = val
            elif tag == 0x8: report_id = val
            elif tag == 0x9: report_count = val
        elif typ == 2:  # local
            if tag == 0x0: usage = val
        elif typ == 0:  # main
            if tag == 0x8:  # input
                out.append(("input", report_id, usage_page, usage, report_size, report_count, val))
            elif tag == 0x9:  # output
                out.append(("output", report_id, usage_page, usage, report_size, report_count, val))
            elif tag == 0xB:  # feature
                out.append(("feature", report_id, usage_page, usage, report_size, report_count, val))
            elif tag == 0xA:  # collection
                out.append(("collection", report_id, usage_page, usage, val))
        i += 1 + size
    return out

for d in hid.enumerate(VID, PID):
    if d["interface_number"] != 3:
        continue
    print("\n===", hex(d["usage_page"]), d["usage"], "===")
    dev = hid.device()
    dev.open_path(d["path"])
    desc = dev.get_report_descriptor()
    print("descriptor", len(desc), "bytes")
    for item in parse_descriptor(desc):
        if item[0] in ("input", "output", "feature"):
            kind, rid, up, us, rsize, rcount, flags = item
            print(f"  {kind:7} report_id={rid} usage_page=0x{up:04x} usage=0x{us:04x} size={rsize} count={rcount} bytes={rsize*rcount//8} flags=0x{flags:02x}")
    dev.close()
