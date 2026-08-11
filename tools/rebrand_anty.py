"""Rename Open Antry / openantry → Open Anty / openanty (case-sensitive)."""
from pathlib import Path
import shutil

root = Path(__file__).resolve().parents[1]

# 1) Rename crate directories if present
dir_renames = [
    ("crates/openantry-proto", "crates/openanty-proto"),
    ("crates/openantry-fp", "crates/openanty-fp"),
    ("crates/openantry-core", "crates/openanty-core"),
    ("crates/openantryd", "crates/openantyd"),
    ("crates/openantry-cli", "crates/openanty-cli"),
]
for src, dst in dir_renames:
    s, d = root / src, root / dst
    if s.exists() and not d.exists():
        s.rename(d)
        print(f"dir {src} -> {dst}")
    elif d.exists():
        print(f"dir already {dst}")

# 2) Text replacements (order matters: longer tokens first)
repls = [
    ("openantry-proto", "openanty-proto"),
    ("openantry-core", "openanty-core"),
    ("openantry-cli", "openanty-cli"),
    ("openantry-fp", "openanty-fp"),
    ("openantryd", "openantyd"),
    ("openantry_proto", "openanty_proto"),
    ("openantry_core", "openanty_core"),
    ("openantry_fp", "openanty_fp"),
    ("OpenAntryService", "OpenAntyService"),
    ("OpenAntryError", "OpenAntyError"),
    ("OPENANTRY_", "OPENANTY_"),
    ("Open Antry", "Open Anty"),
    ("OpenAntry", "OpenAnty"),
    ("openantry", "openanty"),
    ("OA-RECOVERY-", "OA-RECOVERY-"),  # keep
    ("x-openantry-token", "x-openanty-token"),
    ("github.com/openantry/openantry", "github.com/openanty/openanty"),
]

skip_parts = {"target", ".git"}
exts = {".rs", ".toml", ".md", ".yml", ".yaml", ".ps1", ".iss", ".json", ".js", ".py"}
count = 0
for p in root.rglob("*"):
    if any(s in p.parts for s in skip_parts):
        continue
    if not p.is_file() or p.suffix.lower() not in exts:
        continue
    # skip this script until end pattern already applied
    if p.name == "rebrand_anty.py":
        continue
    try:
        t = p.read_text(encoding="utf-8")
    except Exception:
        continue
    n = t
    for a, b in repls:
        n = n.replace(a, b)
    if n != t:
        p.write_text(n, encoding="utf-8")
        count += 1
        print(p.relative_to(root))

# Rename installer file if needed
iss_old = root / "packaging/windows/OpenAntry.iss"
iss_new = root / "packaging/windows/OpenAnty.iss"
if iss_old.exists():
    if iss_new.exists():
        iss_old.unlink()
    else:
        iss_old.rename(iss_new)
    print("iss -> OpenAnty.iss")

print("files_updated", count)
