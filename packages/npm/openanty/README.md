# openanty (npm)

**Agent-first install** for [Open Anty](https://github.com/LoganOneal/OpenAnty) — antidetect multi-profile browser control with native MCP page tools.

## One-line for agents

```bash
npx -y openanty@latest mcp
```

Or configure Claude Desktop / Cursor / Grok once:

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

> The `openanty` package downloads platform binaries from GitHub Releases on install. Requires **Node 18+** and **Chrome/Chromium**.

## What agents can do without Playwright

After MCP connects:

1. `ensure_ready` — doctor + data dir  
2. `setup_scrape_profile` — profile + optional proxy URL + cookies  
3. `launch_session` — isolated browser, returns `session_id`  
4. `page_navigate` / `page_links` / `page_content` / `page_click` / `page_type` / `page_evaluate`  
5. `stop_session`

Example agent prompt:

> Scrape every same-host page on https://example.com using Open Anty. Use proxies if I provide one.

## Env vars

| Variable | Meaning |
| --- | --- |
| `OPENANTY_DATA_DIR` | Profile/DB directory |
| `OPENANTY_BROWSER_PATH` | Chrome/Chromium binary |
| `OPENANTY_BIN_DIR` | Use local binaries instead of vendor/ |
| `OPENANTY_SKIP_DOWNLOAD` | Skip postinstall download |
| `OPENANTY_VERSION` | Pin release tag (e.g. `0.1.1`) |
| `OPENANTY_GITHUB_REPO` | Override `owner/repo` for downloads |

## License

Apache-2.0
