# Open Anty

**Local-first, agent-first open-source antidetect browser platform.**

**Open Anty** lets you create isolated browser profiles with coherent fingerprints, bind proxies, import cookies, and hand AI agents (Claude, Grok, Cursor) a CDP WebSocket URL for Playwright/Puppeteer — primarily via **MCP**, with REST and CLI as peers.

> Not affiliated with Google. Uses stock Chrome/Chromium (or your binary). Patched Chromium is planned; stealth claims are gated on that work. See [RESPONSIBLE_USE.md](RESPONSIBLE_USE.md).

## Features (v0.1)

- Isolated multi-profiles (encrypted secrets at rest)
- Constraint-based fingerprint generation + validation
- Session launch with CDP URL for Playwright **or native MCP page tools**
- Local REST API (`127.0.0.1:3847`) + token auth
- **MCP server** with agent install via **`npx openanty`**
- Native page control: `page_navigate`, `page_content`, `page_links`, `page_click`, … (no Playwright required)
- High-level agent tools: `ensure_ready`, `setup_scrape_profile`
- CLI (`openanty`) for humans and scripts
- Cookie import/export (applied via CDP on launch)
- Proxy attach + health check
- `openanty doctor` environment checks
- Easy installer packaging (Windows script + Inno Setup stubs)

## Open the UI (Dolphin-style control panel)

```bash
# Terminal 1 — start API + embedded UI
openantyd serve

# Terminal 2 — open browser to the panel
openanty ui
# → http://127.0.0.1:3847/
```

The control panel mirrors **Dolphin{anty}-style** layout: dark theme, left sidebar (Browser Profiles, Proxies, Extensions, Automation, Synchronizer, Cookie Robot, Team, Settings), profile table with bulk actions, and a Create Profile modal (general / proxy / fingerprint / cookies).

> Branding is **Open Anty** (not Dolphin). Layout and workflows are intentionally similar for operator familiarity.

## For agents (Claude / Cursor / Grok) — easiest path

**Goal:** User says *“scrape every page on domain.com using Open Anty”* and the agent can install, configure, and control the browser without Playwright.

### 1. Install / start MCP (one line)

```bash
npx -y openanty@latest mcp
```

MCP host config (copy-paste):

```json
{
  "mcpServers": {
    "openanty": {
      "command": "npx",
      "args": ["-y", "openanty@latest", "mcp"]
    }
  }
}
```

Requires **Node 18+** and **Chrome/Chromium**. First run downloads platform binaries from [GitHub Releases](https://github.com/LoganOneal/OpenAnty/releases).

### 2. Tool flow for a domain scrape

1. `ensure_ready`  
2. `setup_scrape_profile` — name + optional `proxy` URL + `cookies`  
3. `launch_session` — `start_url`  
4. Loop: `page_links` (same host) → `page_navigate` → `page_content`  
5. `stop_session`  

### 3. Signup OTP (Gmail)

```bash
# One-time: Google App Password (not your Gmail password)
openanty mail connect you@gmail.com --password "xxxx xxxx xxxx xxxx"
# Agent: after form submit
# mail_wait_otp { "from_contains": "reddit.com", "timeout_seconds": 180 }
```

Recipe: [docs/skills/gmail-otp.md](docs/skills/gmail-otp.md) · scrape: [docs/skills/scrape-domain.md](docs/skills/scrape-domain.md) · gaps: [docs/FEATURE_GAP_AND_AGENT_FIRST.md](docs/FEATURE_GAP_AND_AGENT_FIRST.md)

## Quick start (developers)

```bash
# Requirements: Rust 1.75+, Chrome/Chromium installed
cargo build --release
export PATH="$PWD/target/release:$PATH"   # or Windows: $env:PATH = "...\target\release;$env:PATH"

openanty init
openanty doctor
openanty profile create "demo"
openanty session launch <profile_id> --headless --start-url https://example.com
```

### MCP from a local binary

```bash
openanty mcp-config --npx   # agent-friendly
openanty mcp-config         # local openantyd on PATH
```

```json
{
  "mcpServers": {
    "openanty": {
      "command": "openantyd",
      "args": ["mcp"],
      "env": {
        "OPENANTY_DATA_DIR": "C:\\\\Users\\\\you\\\\AppData\\\\Roaming\\\\OpenAnty"
      }
    }
  }
}
```

### REST API

```bash
openantyd serve
# Authorization: Bearer $(cat %APPDATA%\OpenAnty\api.token)
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:3847/v1/system/status
```

## Easy installer (end users)

Primary distribution is an **easy installer**, not `cargo build`:

Prebuilt binaries are on **[GitHub Releases](https://github.com/LoganOneal/OpenAnty/releases)** — published only when we cut a version tag (not on every CI build).

1. Download **`openanty-windows-x64.exe`** and **`openantyd-windows-x64.exe`** from the release  
   (or the full `OpenAnty-windows-x64.zip` if you want docs + install script)
2. Put them on your `PATH` (optional rename to `openanty.exe` / `openantyd.exe`)
3. `openanty init` then `openanty doctor`

Maintainers: [docs/releasing.md](docs/releasing.md) (`git tag v0.1.0 && git push origin v0.1.0`)

Local portable build (no release):

```powershell
.\packaging\windows\build-portable.ps1
```

See [docs/quick-start.md](docs/quick-start.md) and [DESIGN.md](DESIGN.md).

## Agent workflow

**Native (preferred):**

1. `ensure_ready`  
2. `setup_scrape_profile` (or `create_profile` + `apply_proxy` + `import_cookies`)  
3. `launch_session`  
4. `page_navigate` / `page_content` / `page_links` / …  
5. `stop_session`  

**Playwright attach (optional):** use `cdp_ws_url` from `launch_session` with `connectOverCDP`.

## Project layout

```
crates/
  openanty-proto/   # shared types & error codes
  openanty-fp/      # fingerprint engine
  openanty-core/    # storage, sessions, proxy, cookies, cdp_page
  openantyd/        # daemon: REST + MCP
  openanty-cli/     # `openanty` CLI
packages/npm/openanty/  # npx openanty — agent install
packaging/          # portable / installer helpers
docs/               # guides, skills, releasing
DESIGN.md           # architecture & product design
.github/workflows/
  ci.yml            # test on every PR/push
  release.yml       # binaries only on v* tags
```

## License

Apache-2.0 — see [LICENSE](LICENSE).

## Security

- Binds `127.0.0.1` by default  
- API token required even on loopback  
- CDP is local-only (any local process can attach — inherent CDP limit)  
- Recovery key printed once at init — save offline  
