# GhostFox Quick Start

## Install (easy path — when release assets exist)

1. Download **GhostFox Setup** for your OS from GitHub Releases.  
2. Run the installer (double-click).  
3. Finish the first-run wizard (save the recovery key).  
4. Run **GhostFox Doctor** from the Start Menu (or `ghostfox doctor`).  
5. Copy the MCP snippet into Claude / Cursor / Grok.

Target: **≤ 10 minutes** from download to a green doctor check (excluding browser download).

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
ghostfox init
ghostfox doctor --json
```

## First commands

```bash
ghostfox profile create "shop-us-1" --template win11_chrome_mid
ghostfox profile list
ghostfox session launch prf_... --headless --start-url https://example.com
# note cdp_ws_url in JSON output
ghostfox session stop ses_...
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
| Windows | `%APPDATA%\GhostFox` |
| macOS | `~/Library/Application Support/GhostFox` |
| Linux | `~/.local/share/ghostfox` |

Override with `GHOSTFOX_DATA_DIR`.

## Playwright connect (JavaScript)

```js
const { chromium } = require('playwright');
const browser = await chromium.connectOverCDP(process.env.CDP_URL);
const context = browser.contexts()[0];
const page = context.pages()[0] || await context.newPage();
await page.goto('https://example.com');
```
