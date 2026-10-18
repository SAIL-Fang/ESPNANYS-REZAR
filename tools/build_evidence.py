"""Build concise evidence files from the captured logs."""
import os
import pathlib
import re
import json

ROOT = pathlib.Path(__file__).resolve().parents[1]
CAP = ROOT / "captures"
LOCAL_APP_DATA = pathlib.Path(
    os.environ.get("LOCALAPPDATA", pathlib.Path.home() / "AppData" / "Local")
)
LOG = LOCAL_APP_DATA / "Razer" / "RazerAppEngine" / "User Data" / "Logs"

# --- THX normalization evidence -------------------------------------------------
out = []
out.append("# THX '声音标准化' (DRC) evidence\n")
out.append("Registry HKCU\\Software\\THX\\SpatialAudio\\UserState:\n")
try:
    import winreg
    key = winreg.OpenKey(winreg.HKEY_CURRENT_USER, r"Software\THX\SpatialAudio\UserState")
    for name in ("UserScope", "Game Mode-1-Headphones-usb\\1532\\0565"):
        try:
            value, _ = winreg.QueryValueEx(key, name)
            m1 = re.search(r'"drc_enabled"\s*:\s*(true|false)', value)
            m2 = re.search(r'"drc_level"\s*:\s*(\d+)', value)
            out.append(f"{name}: drc_enabled={m1.group(1) if m1 else '?'} drc_level={m2.group(1) if m2 else '?'}\n")
        except FileNotFoundError:
            pass
except Exception as exc:
    out.append(f"(registry read failed: {exc})\n")

for p in sorted(LOG.glob("THXDLL_V3CommonSubProcess*.log"), key=lambda x: x.stat().st_mtime):
    lines = p.read_text(encoding="utf-8", errors="ignore").splitlines()
    for needle in ("action: 'SetRenderNormalization'", "action: 'SetRenderNormalizationLevel'"):
        idx = [i for i, l in enumerate(lines) if needle in l]
        if not idx:
            continue
        out.append(f"\n## {p.name}: {needle} ({len(idx)} occurrences)\n")
        for i in idx[:2] + idx[-2:]:
            block = "\n".join(lines[max(0, i - 1):i + 14])
            out.append(block + "\n---\n")

(CAP / "thx-normalization-evidence.txt").write_text("\n".join(out), encoding="utf-8")

# --- EQ preset evidence ---------------------------------------------------------
pairs = {}
mic_fail = None
for p in LOG.glob("products_1390*.log"):
    text = p.read_text(encoding="utf-8", errors="ignore")
    for m in re.finditer(r'"eqPreset":(\d+)[^}]{0,80}?"eqPresetEnum":"([^"]+)"', text):
        pairs[(m.group(1), m.group(2))] = pairs.get((m.group(1), m.group(2)), 0) + 1
    if mic_fail is None:
        m = re.search(r'"command":\[150,0,1\][^\n]{0,250}?"data":\[1\][^\n]{0,250}?"error":"([^"]+)"', text)
        if m:
            mic_fail = m.group(1)
eq_out = ["# EQ preset evidence\n", "Observed eqPreset -> eqPresetEnum:\n"]
for k, v in sorted(pairs.items()):
    eq_out.append(f"  {k[0]} -> {k[1]} (x{v})\n")
eq_out.append("\nSet Mic Preset EQ Index (0x96) on 0x0565:\n")
eq_out.append(f"  status data [1], error: {mic_fail or 'not found'}\n")
eq_out.append("\nCommand descriptions from renderer logs:\n")
for desc in ("Set Preset EQ Index", "Set Mic Preset EQ Index", "Set Customer EQ Band", "Set Speaker Preset EQ Status"):
    eq_out.append(f"  {desc}\n")
(CAP / "eq-preset-evidence.txt").write_text("".join(eq_out), encoding="utf-8")

# --- sidetone HID evidence ------------------------------------------------------
p = CAP / "protocol.log"
rows = []
for line in p.read_text(encoding="utf-8", errors="ignore").splitlines():
    m = re.search(r"(\d{4}-\d\d-\d\d \d\d:\d\d:\d\d\.\d+).*? 1381 .*?\s([wr])\s+\[([0-9a-fA-F,]+)\]\s+64", line)
    if not m:
        continue
    arr = [int(x, 16) for x in m.group(3).split(",")]
    if len(arr) != 64:
        continue
    cmd = (arr[10] << 8) | arr[11] if arr[10] in (0x98, 0x99) else None
    if cmd:
        rows.append((m.group(1), m.group(2), arr[2], arr[10], arr[12], arr[13], arr[62]))
st = ["# Sidetone HID evidence (protocol.log)\n",
      "time                     dir seq  cmd  len value chk\n"]
for r in rows:
    st.append(f"{r[0]}  {r[1]}   {r[2]:02x}   {r[3]:02x}   {r[4]}   {r[5]}    {r[6]:02x}\n")
(CAP / "sidetone-evidence.txt").write_text("".join(st), encoding="utf-8")

print("written:", [p.name for p in CAP.glob("*evidence.txt")])
