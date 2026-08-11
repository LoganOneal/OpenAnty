# Changelog

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
