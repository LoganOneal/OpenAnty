# Feature gap analysis & agent-first roadmap

**Product:** Open Anty  
**Date:** 2026-08-11  
**Goal:** Beat commercial antidetect tools on **agent UX**, match them on core isolation, and make  
“Claude: scrape every page on domain.com with Open Anty” a one-shot workflow.

---

## 1. Competitive feature matrix (what others have)

Legend: **Y** = we have · **P** = partial · **N** = missing · **A** = agent-native advantage we should own

| Feature | AdsPower | Multilogin | Dolphin | GoLogin | Agent-browser / Playwright MCP | **Open Anty now** | Priority |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Isolated multi-profiles | Y | Y | Y | Y | N | **Y** | — |
| Fingerprint coherence | Y | Y | Y | Y | N | **P** (doc + stock) | P0 patches |
| Proxy per profile | Y | Y | Y | Y | P | **Y** | — |
| Cookie import/export | Y | Y | Y | Y | P | **Y** | — |
| Cookie robot / warm-up | Y | Y | Y | P | N | **N** | P2 |
| Local API | Y | Y | Y | Y | Y | **Y** | — |
| Selenium/Puppeteer/Playwright attach | Y | Y | Y | Y | Y | **Y** (CDP) | — |
| **MCP agent server** | N | N | N | P | Y | **Y** | Own this |
| **Native page control in MCP** (no Playwright install) | N | N | N | N | Y | **N → Y (this plan)** | **P0** |
| **npm / npx install** | N | N | N | N | Y | **N → Y (this plan)** | **P0** |
| Visual RPA / scenario builder | Y | P | Y | P | N | **N** | P2 |
| Action synchronizer (1→N windows) | Y | P | Y | P | N | **N** | P2 |
| Team RBAC / cloud sync | Y | Y | Y | Y | N | **N** | P3 |
| Built-in proxy marketplace | P | Y | N | Y | N | **N** (BYO) | Non-goal |
| Mobile / cloud phones | P | Y | P | P | N | **N** | Non-goal |
| Patched Chromium stealth engine | Y | Y | Y | Y | N | **P** | P1 |
| Fingerprint health harness | N | N | N | N | N | **P** | P1 |
| AdsPower API compatibility shim | — | — | — | — | N | **N** | P2 |
| GUI desktop app | Y | Y | Y | Y | N | **N** | P2 |
| Extension management | Y | Y | Y | Y | N | **N** | P1 |
| Bulk profile ops | Y | Y | Y | Y | N | **P** | P1 |
| Human typing / mouse curves | P | Y | P | P | P | **N** | P2 |
| Captcha solvers | P | P | P | P | P | **N** (out of scope) | — |

### Where Open Anty wins (by design)

1. **Agent-first MCP** — commercial tools bolt APIs onto GUIs; we invert that.  
2. **Local-first + OSS** — no cloud profile hostage.  
3. **One-command agent install** (`npx openanty`) — same pattern as BrowserStack MCP / agent-browser.  
4. **Native CDP tools in MCP** — Claude doesn’t need Playwright installed to click/scrape.

### Where we lag (must plan)

1. Engine-level stealth (patched Chromium)  
2. Cookie robot / account warm-up  
3. Synchronizer + no-code RPA  
4. Team / multi-user  
5. Polish GUI  

---

## 2. Best-in-class agent install & control (research summary)

| Pattern | Who | Why it works for agents |
| --- | --- | --- |
| **`npx -y @pkg/mcp`** | BrowserStack MCP, many MCP servers | Claude/Cursor already have Node; zero global install |
| **`npm i -g` + binary postinstall** | Vercel **agent-browser** | Native speed, downloads browser/binary on first use |
| **Desktop Extension `.mcpb`** | Anthropic Claude Desktop | One-click install for non-devs |
| **CDP tools inside MCP** | Playwright MCP, agent-browser | Agent never shells out to Python/JS scrape scripts |
| **High-level “workflow” tools** | Browserless agent tools | One tool ≈ one user intent (“scrape site”) |

**Canonical agent flow we target:**

```
User → Claude: "Scrape every page on domain.com with Open Anty"
Claude → npx openanty (or MCP already configured)
Claude → ensure_ready / create_profile / apply_proxy / import_cookies
Claude → launch_session
Claude → page_goto / page_links / page_content  (native MCP, no Playwright)
Claude → stop_session / export_cookies
```

---

## 3. Agent-first product principles

1. **Install in one line** agents already know: `npx -y openanty@latest mcp`  
2. **Zero secondary deps** for basic scrape (no Playwright required for MCP path).  
3. **Intent tools** alongside primitives (`scrape_url`, `discover_links`, `setup_profile`).  
4. **Structured results** always include `cdp_ws_url` *and* extracted data.  
5. **Idempotent setup** — `ensure_ready` creates data dir, binary, doctor if needed.  
6. **Safe defaults** — localhost, token, no LAN, responsible-use in tool descriptions.

---

## 4. Phased delivery plan

### Phase A — Agent install + native control (**now**)

- [x] Feature gap doc (this file)  
- [x] **npm package `openanty`**: download GH release binaries, expose `openanty` / `openantyd` / `mcp`  
- [x] MCP tools: `page_navigate`, `page_content`, `page_links`, `page_evaluate`, `page_click`, `page_type`  
- [x] MCP tools: `ensure_ready`, `setup_scrape_profile`  
- [x] Skill doc: scrape-domain agent recipe  
- [x] README: “For Claude / agents” section with copy-paste MCP JSON using npx  

### Phase B — Stealth depth (next)

- Patched Chromium distribution (Phase 0 gate from DESIGN.md)  
- Extension load per profile  
- Fingerprint health public suites  
- Bulk profile create  

### Phase C — Operator parity

- Cookie robot / warm-up  
- Scenario runner (JSON steps, not full visual RPA)  
- Synchronizer MVP  
- AdsPower local API shim subset  
- Tauri GUI  

### Phase D — Teams (optional)

- Local multi-user RBAC  
- Optional encrypted sync  

---

## 5. “Scrape every page on domain.com” acceptance criteria

An agent with only MCP + Node should:

1. Install/start Open Anty via **`npx -y openanty mcp`** (or preconfigured MCP).  
2. Call **`ensure_ready`** → doctor green.  
3. Call **`setup_scrape_profile`** with optional proxy string.  
4. **`launch_session`** with `start_url`.  
5. Loop: **`page_links`** → filter same-host → **`page_navigate`** → **`page_content`**.  
6. **`stop_session`**.  
7. Return structured pages JSON without the user installing Rust/Playwright.

---

## 6. Non-goals (keep agent surface clean)

- Captcha farms / solver marketplaces  
- Proxy resale  
- Guaranteeing undetectability  
- Cloud multi-tenant SaaS as default  

---

## 7. Success metrics

| Metric | Target |
| --- | --- |
| Time for agent to first `page_content` | &lt; 2 minutes on clean machine with Node + Chrome |
| MCP tools for scrape path | ≤ 8 tools needed for BFS same-host scrape |
| Install commands | 1 (`npx`) or 0 (if MCP preconfigured) |
| Feature parity with AdsPower core isolation | Yes |
| Feature parity with AdsPower RPA/sync | Deferred (Phase C) |
