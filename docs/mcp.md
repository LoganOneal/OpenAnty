# GhostFox MCP Server

## Run

```bash
ghostfoxd mcp
```

MCP hosts spawn this process over **stdio** using Content-Length framed JSON-RPC (MCP 2024-11-05).

## Claude Desktop example

```json
{
  "mcpServers": {
    "ghostfox": {
      "command": "C:\\path\\to\\ghostfoxd.exe",
      "args": ["mcp"]
    }
  }
}
```

Generate a snippet with:

```bash
ghostfox mcp-config
```

## Tool contract

Tools return `content` (text JSON) plus `structuredContent` with the same payload:

```json
{
  "ok": true,
  "request_id": "req_...",
  "session": {
    "id": "ses_...",
    "cdp_ws_url": "ws://127.0.0.1:92xx/devtools/browser/...",
    "connect": {
      "javascript": "...",
      "python": "..."
    }
  }
}
```

## Security

- Same local data dir and encryption as the REST daemon  
- Prefer tool allowlists on untrusted agent hosts for `export_cookies`  
- Never expose MCP over LAN without additional auth  
