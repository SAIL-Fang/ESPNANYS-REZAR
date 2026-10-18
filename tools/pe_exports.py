"""Minimal PE export-table reader (no external dependencies)."""
import struct
import sys


def u16(b, o): return struct.unpack_from("<H", b, o)[0]
def u32(b, o): return struct.unpack_from("<I", b, o)[0]
def u64(b, o): return struct.unpack_from("<Q", b, o)[0]


def exports(path):
    data = open(path, "rb").read()
    if data[:2] != b"MZ":
        raise ValueError("not a PE file")
    pe = u32(data, 0x3C)
    if data[pe:pe + 4] != b"PE\0\0":
        raise ValueError("missing PE signature")
    coff = pe + 4
    nsections = u16(data, coff + 2)
    opt_size = u16(data, coff + 16)
    opt = coff + 20
    magic = u16(data, opt)
    is64 = magic == 0x20B
    dd = opt + (112 if is64 else 96)
    export_rva = u32(data, dd + 0)
    export_size = u32(data, dd + 4)
    if export_rva == 0:
        return []
    sections = []
    sec = opt + opt_size
    for i in range(nsections):
        off = sec + i * 40
        va = u32(data, off + 12)
        vsize = u32(data, off + 8)
        raw = u32(data, off + 20)
        rawsize = u32(data, off + 16)
        sections.append((va, max(vsize, rawsize), raw))

    def rva_to_off(rva):
        for va, size, raw in sections:
            if va <= rva < va + size:
                return raw + (rva - va)
        raise ValueError(f"RVA 0x{rva:x} not mapped")

    ed = rva_to_off(export_rva)
    nnames = u32(data, ed + 24)
    names_rva = u32(data, ed + 32)
    ords_rva = u32(data, ed + 36)
    funcs_rva = u32(data, ed + 28)
    names_off = rva_to_off(names_rva)
    ords_off = rva_to_off(ords_rva)
    funcs_off = rva_to_off(funcs_rva)
    out = []
    for i in range(nnames):
        name_rva = u32(data, names_off + i * 4)
        name_off = rva_to_off(name_rva)
        end = data.index(b"\0", name_off)
        name = data[name_off:end].decode("ascii", "replace")
        ordinal = u16(data, ords_off + i * 2)
        func_rva = u32(data, funcs_off + ordinal * 4)
        out.append((name, ordinal, func_rva))
    return sorted(out)


if __name__ == "__main__":
    path = sys.argv[1]
    rows = exports(path)
    print(f"{len(rows)} exports")
    for name, ordinal, rva in rows:
        print(f"{ordinal:4d}  0x{rva:08x}  {name}")
