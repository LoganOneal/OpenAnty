# Skill: Scrape a domain with Open Anty

Use this when a user asks to scrape pages on a domain **using Open Anty** (or “openanty”).

## Install / connect (one shot)

Agents should **not** ask the user to build from Rust. Prefer:

```bash
npx -y openanty@latest mcp
```

Or MCP host config:

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

Requires: **Node 18+**, **Chrome/Chromium**, outbound network for first binary download.

## Tool sequence (canonical)

```
1. ensure_ready
   → if ready=false, tell user to install Chrome or set OPENANTY_BROWSER_PATH

2. setup_scrape_profile
   name: "scrape-<domain>"
   proxy: (optional) "http://user:pass@host:port" or "socks5://host:port"
   cookies: (optional) array of {name, value, domain, ...}

3. launch_session
   profile_id: from step 2
   start_url: "https://domain.com"
   headed: false  (unless user wants to watch)

4. BFS same-host scrape:
   page_content(session_id) → save title/url/text or html
   page_links(session_id, same_host_only=true)
   for each unvisited link (cap e.g. 50–200 unless user says otherwise):
     page_navigate(session_id, url)
     page_content(...)
     page_links(...) again if needed

5. stop_session(session_id)

6. Return structured JSON: [{url, title, content}, ...]
```

## Do **not**

- Install Playwright for this path (native `page_*` tools are enough).
- Scrape out of scope / paywalled content the user is not allowed to access.
- Expose `export_cookies` secrets in chat unless the user asks.

## Proxy & cookies

- **Proxy:** pass a single URL string to `setup_scrape_profile.proxy`.
- **Cookies:** Netscape/JSON cookie objects with at least `name`, `value`, `domain`.
- Reuse profiles: `list_profiles` then `launch_session` if a scrape profile already exists.

## Example user prompt

> Scrape every page on example.com using Open Anty, text only, max 30 pages.

Agent checklist: ensure_ready → setup_scrape_profile → launch_session → loop page_links/page_navigate/page_content → stop_session.
