# OpenAnty Quick Start

## Install (easy path — GitHub Release)

1. Open [GitHub Releases](https://github.com/LoganOneal/OpenAnty/releases) and download the zip/tarball for your OS.  
2. Unzip the archive.  
3. Windows: run `INSTALL.ps1` (or `bin\openanty.exe init`). macOS/Linux: `chmod +x install.sh bin/* && ./install.sh`.  
4. Save the recovery key when prompted.  
5. Run `openanty doctor`, then `openanty mcp-config` for Claude / Cursor / Grok.

Target: **≤ 10 minutes** from download to a green doctor check.  
Maintainers: see [releasing.md](releasing.md) — releases are tag-only, not every CI build.

## Install (from source)

```bash
git clone <repo>
cd opensource-no-detect-browser-for-agents
cargo build --release
```

Add `target/release` to your `PATH`.

### Windows PowerShell

```powershell
$env:PATH = "$PWD\target\release;$env:PATH"
openanty init
openanty doctor --json
```

## First commands

```bash
openanty profile create "shop-us-1" --template win11_chrome_mid
openanty profile list
openanty session launch prf_... --headless --start-url https://example.com
# note cdp_ws_url in JSON output
openanty session stop ses_...
```

## MCP tools

| Tool | Purpose |
| --- | --- |
| `create_profile` | New isolated identity |
| `launch_session` | Start browser → CDP URL |
| `stop_session` | Tear down |
| `import_cookies` / `export_cookies` | Cookie blobs |
| `apply_proxy` | Per-profile proxy |
| `doctor` | Environment checks |

## Data directory

| OS | Path |
| --- | --- |
| Windows | `%APPDATA%\OpenAnty` |
| macOS | `~/Library/Application Support/OpenAnty` |
| Linux | `~/.local/share/OpenAnty` |

Override with `OPENANTY_DATA_DIR`.

## Playwright connect (JavaScript)

```js
const { chromium } = require('playwright');
const browser = await chromium.connectOverCDP(process.env.CDP_URL);
const context = browser.contexts()[0];
const page = context.pages()[0] || await context.newPage();
await page.goto('https://example.com');
```
