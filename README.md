# Open Antry

**Local-first, agent-first open-source antidetect browser platform.**

**Open Antry** lets you create isolated browser profiles with coherent fingerprints, bind proxies, import cookies, and hand AI agents (Claude, Grok, Cursor) a CDP WebSocket URL for Playwright/Puppeteer — primarily via **MCP**, with REST and CLI as peers.

> Not affiliated with Google. Uses stock Chrome/Chromium (or your binary). Patched Chromium is planned; stealth claims are gated on that work. See [RESPONSIBLE_USE.md](RESPONSIBLE_USE.md).

## Features (v0.1)

- Isolated multi-profiles (encrypted secrets at rest)
- Constraint-based fingerprint generation + validation
- Session launch with CDP URL for Playwright
- Local REST API (`127.0.0.1:3847`) + token auth
- **MCP server** (`openantryd mcp`) for agents
- CLI (`openantry`) for humans and scripts
- Cookie import/export (applied via CDP on launch)
- Proxy attach + health check
- `openantry doctor` environment checks
- Easy installer packaging (Windows script + Inno Setup stubs)

## Quick start (developers)

```bash
# Requirements: Rust 1.75+, Chrome/Chromium installed
cargo build --release
export PATH="$PWD/target/release:$PATH"   # or Windows: $env:PATH = "...\target\release;$env:PATH"

openantry init
openantry doctor
openantry profile create "demo"
openantry session launch <profile_id> --headless --start-url https://example.com
```

### MCP (Claude Desktop / Cursor / Grok)

```bash
openantry mcp-config
```

Paste into your MCP host config:

```json
{
  "mcpServers": {
    "openantry": {
      "command": "openantryd",
      "args": ["mcp"],
      "env": {
        "OPENANTRY_DATA_DIR": "C:\\\\Users\\\\you\\\\AppData\\\\Roaming\\\\OpenAntry"
      }
    }
  }
}
```

### REST API

```bash
openantryd serve
# Authorization: Bearer $(cat %APPDATA%\OpenAntry\api.token)
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:3847/v1/system/status
```

## Easy installer (end users)

Primary distribution is an **easy installer**, not `cargo build`:

| Platform | Artifact |
| --- | --- |
| Windows | `OpenAntry-Setup-*.exe` (see `packaging/windows/`) |
| macOS | DMG (planned) |
| Linux | AppImage (planned) |

Build a Windows portable bundle:

```powershell
.\packaging\windows\build-portable.ps1
```

See [docs/quick-start.md](docs/quick-start.md) and [DESIGN.md](DESIGN.md).

## Agent workflow

1. `create_profile` — coherent fingerprint  
2. `apply_proxy` (optional)  
3. `import_cookies` (optional)  
4. `launch_session` → **`cdp_ws_url`**  
5. Playwright `connectOverCDP` / Puppeteer connect  
6. `stop_session`

## Project layout

```
crates/
  openantry-proto/   # shared types & error codes
  openantry-fp/      # fingerprint engine
  openantry-core/    # storage, sessions, proxy, cookies
  openantryd/        # daemon: REST + MCP
  openantry-cli/     # `OpenAntry` CLI
packaging/          # installers
docs/               # guides
DESIGN.md           # architecture & product design
```

## License

Apache-2.0 — see [LICENSE](LICENSE).

## Security

- Binds `127.0.0.1` by default  
- API token required even on loopback  
- CDP is local-only (any local process can attach — inherent CDP limit)  
- Recovery key printed once at init — save offline  
