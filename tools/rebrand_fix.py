from pathlib import Path

root = Path(__file__).resolve().parents[1]
skip = {"target", ".git", "tools"}
repls = [
    ("OpenAntry_fp", "openantry_fp"),
    ("OpenAntry_proto", "openantry_proto"),
    ("OpenAntry_core", "openantry_core"),
    ("OpenAntry_DATA_DIR", "OPENANTRY_DATA_DIR"),
    ("OpenAntry_BROWSER_PATH", "OPENANTRY_BROWSER_PATH"),
    ("OpenAntry_MASTER_KEY", "OPENANTRY_MASTER_KEY"),
    ("OpenAntry_BIND", "OPENANTRY_BIND"),
    ("OpenAntry_MAX_SESSIONS", "OPENANTRY_MAX_SESSIONS"),
    ("OpenAntry_ALLOW_LAN", "OPENANTRY_ALLOW_LAN"),
    ("OpenAntry_EXPERIMENTAL_JS_STEALTH", "OPENANTRY_EXPERIMENTAL_JS_STEALTH"),
    ("OpenAntry_INSECURE_NO_TOKEN", "OPENANTRY_INSECURE_NO_TOKEN"),
    ("OpenAntry_API_BASE", "OPENANTRY_API_BASE"),
    ("OpenAntry_ADSPOWER_SHIM", "OPENANTRY_ADSPOWER_SHIM"),
    ("OpenAntry_ORPHAN_POLICY", "OPENANTRY_ORPHAN_POLICY"),
    ("OpenAntry_CDP_PORT_RANGE", "OPENANTRY_CDP_PORT_RANGE"),
    ("OpenAntry_STRICT_FP", "OPENANTRY_STRICT_FP"),
    ("OpenAntry_STOCK_UA_FLAG", "OPENANTRY_STOCK_UA_FLAG"),
    ("x-OpenAntry-token", "x-openantry-token"),
    ('name = "OpenAntry"', 'name = "openantry"'),
    ('name = "OpenAntryd"', 'name = "openantryd"'),
    ('default_value = "OpenAntryd"', 'default_value = "openantryd"'),
    ('"OpenAntry"', '"openantry"'),
    ("OpenAntryd", "openantryd"),
    ("OpenAntry-cli", "openantry-cli"),
    ("OpenAntry-proto", "openantry-proto"),
    ("OpenAntry-core", "openantry-core"),
    ("OpenAntry-fp", "openantry-fp"),
    ("OpenAntry.exe", "openantry.exe"),
    ("github.com/OpenAntry/openantry", "github.com/openantry/openantry"),
    ("github.com/OpenAntry/OpenAntry", "github.com/openantry/openantry"),
    ("crates/OpenAntry", "crates/openantry"),
]
exts = {".rs", ".toml", ".md", ".yml", ".ps1", ".iss", ".json", ".js"}
count = 0
for p in root.rglob("*"):
    if any(s in p.parts for s in skip):
        continue
    if p.suffix.lower() not in exts or not p.is_file():
        continue
    t = p.read_text(encoding="utf-8")
    n = t
    for a, b in repls:
        n = n.replace(a, b)
    if n != t:
        p.write_text(n, encoding="utf-8")
        count += 1
        print(p.relative_to(root))
print("files", count)
