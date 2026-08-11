# Changelog

## Unreleased

### Added

- **BYO Gmail / IMAP OTP:** `mail_connect`, `mail_wait_otp`, `mail_list`, `mail_status`, `mail_handoff` (MCP + REST `/v1/mail/*` + CLI `openanty mail`). Gmail App Passwords; encrypted local credentials. See [docs/skills/gmail-otp.md](docs/skills/gmail-otp.md)
- **Operator UI:** Dolphin{anty}-inspired control panel at `http://127.0.0.1:3847/` (`openantyd serve` + `openanty ui`)
- **Phase B/C/D features:** proxy pool, extensions, bulk profiles, fingerprint health, cookie robot, scenario runner, synchronizer navigate, local team users, AdsPower API shim (`/browser/*`)
- **Agent-first install:** npm package `packages/npm/openanty` — `npx -y openanty@latest mcp` downloads GitHub Release binaries
- **Native MCP page tools** (no Playwright): `page_navigate`, `page_content`, `page_links`, `page_evaluate`, `page_click`, `page_type`
- **Intent tools:** `ensure_ready`, `setup_scrape_profile` (proxy URL + cookies in one call)
- Docs: [FEATURE_GAP_AND_AGENT_FIRST.md](docs/FEATURE_GAP_AND_AGENT_FIRST.md), [skills/scrape-domain.md](docs/skills/scrape-domain.md)
- CLI: `openanty mcp-config --npx`, `openanty ui`

## 0.1.0 — 2026-08-10

### Added

- Initial monorepo: `openanty-proto`, `openanty-fp`, `openanty-core`, `openantyd`, `openanty-cli`
- Tag-based GitHub Release workflow (binaries only when publishing `v*` tags)
- Constraint-based fingerprint generation and validation
- Encrypted-at-rest profile secrets (XChaCha20-Poly1305)
- Profile CRUD, cookie import/export, proxy check
- Session launch against stock Chrome/Chromium with CDP URL
- REST API on `127.0.0.1:3847` with bearer token + Host allowlist
- MCP stdio server (`openantyd mcp`) with core agent tools
- CLI: `init`, `doctor`, `mcp-config`, profile/session commands
- Windows portable packaging script + Inno Setup installer stub
- CI workflow, docs, responsible use policy
