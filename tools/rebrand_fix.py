from pathlib import Path

root = Path(__file__).resolve().parents[1]
skip = {"target", ".git", "tools"}
repls = [
    ("OpenAnty_fp", "openanty_fp"),
    ("OpenAnty_proto", "openanty_proto"),
    ("OpenAnty_core", "openanty_core"),
    ("OpenAnty_DATA_DIR", "OPENANTY_DATA_DIR"),
    ("OpenAnty_BROWSER_PATH", "OPENANTY_BROWSER_PATH"),
    ("OpenAnty_MASTER_KEY", "OPENANTY_MASTER_KEY"),
    ("OpenAnty_BIND", "OPENANTY_BIND"),
    ("OpenAnty_MAX_SESSIONS", "OPENANTY_MAX_SESSIONS"),
    ("OpenAnty_ALLOW_LAN", "OPENANTY_ALLOW_LAN"),
    ("OpenAnty_EXPERIMENTAL_JS_STEALTH", "OPENANTY_EXPERIMENTAL_JS_STEALTH"),
    ("OpenAnty_INSECURE_NO_TOKEN", "OPENANTY_INSECURE_NO_TOKEN"),
    ("OpenAnty_API_BASE", "OPENANTY_API_BASE"),
    ("OpenAnty_ADSPOWER_SHIM", "OPENANTY_ADSPOWER_SHIM"),
    ("OpenAnty_ORPHAN_POLICY", "OPENANTY_ORPHAN_POLICY"),
    ("OpenAnty_CDP_PORT_RANGE", "OPENANTY_CDP_PORT_RANGE"),
    ("OpenAnty_STRICT_FP", "OPENANTY_STRICT_FP"),
    ("OpenAnty_STOCK_UA_FLAG", "OPENANTY_STOCK_UA_FLAG"),
    ("x-OpenAnty-token", "x-openanty-token"),
    ('name = "OpenAnty"', 'name = "openanty"'),
    ('name = "OpenAntyd"', 'name = "openantyd"'),
    ('default_value = "OpenAntyd"', 'default_value = "openantyd"'),
    ('"OpenAnty"', '"openanty"'),
    ("OpenAntyd", "openantyd"),
    ("OpenAnty-cli", "openanty-cli"),
    ("OpenAnty-proto", "openanty-proto"),
    ("OpenAnty-core", "openanty-core"),
    ("OpenAnty-fp", "openanty-fp"),
    ("OpenAnty.exe", "openanty.exe"),
    ("github.com/OpenAnty/openanty", "github.com/openanty/openanty"),
    ("github.com/OpenAnty/OpenAnty", "github.com/openanty/openanty"),
    ("crates/OpenAnty", "crates/openanty"),
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
