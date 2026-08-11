# Open Anty MCP Server

## Agent install (recommended)

Zero local binary management — Node downloads release assets:

```bash
npx -y openanty@latest mcp
```

Claude Desktop / Cursor / Grok MCP config:

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

Generate a snippet:

```bash
openanty mcp-config --npx
# or local binary:
openanty mcp-config
```

## Run from a local binary

```bash
openantyd mcp
```

MCP hosts spawn this process over **stdio** using Content-Length framed JSON-RPC (MCP 2024-11-05).

## Tools

### Lifecycle / setup

| Tool | Purpose |
| --- | --- |
| `ensure_ready` | Doctor + data dir; call first |
| `setup_scrape_profile` | Create profile + optional proxy URL + cookies |
| `create_profile` / `list_profiles` / `get_profile` / `delete_profile` | Profile CRUD |
| `apply_proxy` | Attach proxy object |
| `import_cookies` / `export_cookies` | Cookie blob |
| `launch_session` / `stop_session` / `list_sessions` | Browser lifecycle |
| `get_session_cdp_url` / `heartbeat_session` | CDP attach / TTL |
| `doctor` | Environment checks |

### Native page control (no Playwright)

| Tool | Purpose |
| --- | --- |
| `page_navigate` | Goto URL; returns html snapshot |
| `page_content` | Current page `html` or `text` |
| `page_links` | Extract links (`same_host_only` default true) |
| `page_evaluate` | Run JS, return JSON value |
| `page_click` | Click CSS selector |
| `page_type` | Type into input selector |

Optional: still use Playwright via `cdp_ws_url` from `launch_session` if you prefer.

## Tool contract

Tools return `content` (text JSON) plus `structuredContent` with the same payload:

```json
{
  "ok": true,
  "request_id": "req_...",
  "session_id": "ses_...",
  "url": "https://example.com/",
  "title": "Example Domain",
  "content": "..."
}
```

## Scrape-domain recipe

See [skills/scrape-domain.md](skills/scrape-domain.md).

## Security

- Same local data dir and encryption as the REST daemon  
- Prefer tool allowlists on untrusted agent hosts for `export_cookies`  
- Never expose MCP over LAN without additional auth  
