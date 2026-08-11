//! Minimal MCP stdio server (JSON-RPC 2.0) for OpenAnty tools.
//! Includes native CDP page control so agents do not need Playwright.

use openanty_core::cdp_page;
use openanty_core::OpenAntyService;
use openanty_proto::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader as TokioBufReader};
use tokio::sync::Mutex;

pub async fn run(service: Arc<OpenAntyService>) -> anyhow::Result<()> {
    // MCP over stdio: Content-Length framing (LSP-style) OR newline JSON.
    // We support newline-delimited JSON-RPC for simplicity + Content-Length.
    let stdin = tokio::io::stdin();
    let mut reader = TokioBufReader::new(stdin);
    let stdout = Arc::new(Mutex::new(tokio::io::stdout()));
    let mut line_buf = String::new();

    loop {
        line_buf.clear();
        // Try Content-Length first by peeking is hard; use dual mode:
        // If line starts with "Content-Length:", read framed message.
        let n = reader.read_line(&mut line_buf).await?;
        if n == 0 {
            break;
        }
        let trimmed = line_buf.trim_end();
        if trimmed.is_empty() {
            continue;
        }

        let msg = if trimmed.to_ascii_lowercase().starts_with("content-length:") {
            let len: usize = trimmed
                .split(':')
                .nth(1)
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            // read remaining headers until blank line
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).await?;
                if h.trim().is_empty() {
                    break;
                }
            }
            let mut body = vec![0u8; len];
            use tokio::io::AsyncReadExt;
            reader.read_exact(&mut body).await?;
            String::from_utf8(body)?
        } else {
            trimmed.to_string()
        };

        let req: Value = match serde_json::from_str(&msg) {
            Ok(v) => v,
            Err(e) => {
                write_json(
                    &stdout,
                    json!({
                        "jsonrpc": "2.0",
                        "id": null,
                        "error": { "code": -32700, "message": e.to_string() }
                    }),
                )
                .await?;
                continue;
            }
        };

        let id = req.get("id").cloned().unwrap_or(Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(json!({}));

        // notifications (no id response for some) — still fine to ignore reply if id null and method starts with notifications/
        if method == "notifications/initialized" || method == "notifications/cancelled" {
            continue;
        }

        let result = match method {
            "initialize" => Ok(json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "openanty",
                    "version": env!("CARGO_PKG_VERSION")
                }
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tool_defs() })),
            "tools/call" => handle_tool_call(service.clone(), params).await,
            _ => Err(format!("method not found: {method}")),
        };

        let response = match result {
            Ok(r) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": r
            }),
            Err(e) => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": e }
            }),
        };
        write_json(&stdout, response).await?;
    }
    Ok(())
}

async fn write_json(
    stdout: &Arc<Mutex<tokio::io::Stdout>>,
    value: Value,
) -> anyhow::Result<()> {
    let body = serde_json::to_string(&value)?;
    let mut out = stdout.lock().await;
    // Content-Length framing for broad MCP host compatibility
    let header = format!("Content-Length: {}\r\n\r\n", body.len());
    out.write_all(header.as_bytes()).await?;
    out.write_all(body.as_bytes()).await?;
    out.flush().await?;
    Ok(())
}

fn tool_defs() -> Vec<Value> {
    vec![
        tool(
            "create_profile",
            "Create an isolated browser profile with a coherent fingerprint",
            json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string" },
                    "template": { "type": "string" },
                    "os": { "type": "string" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "notes": { "type": "string" }
                }
            }),
        ),
        tool(
            "list_profiles",
            "List browser profiles",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer" }
                }
            }),
        ),
        tool(
            "get_profile",
            "Get a profile by id",
            json!({
                "type": "object",
                "required": ["profile_id"],
                "properties": {
                    "profile_id": { "type": "string" },
                    "include_secrets": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "delete_profile",
            "Soft-delete a profile",
            json!({
                "type": "object",
                "required": ["profile_id"],
                "properties": {
                    "profile_id": { "type": "string" },
                    "confirm": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "apply_proxy",
            "Attach and optionally check a proxy on a profile",
            json!({
                "type": "object",
                "required": ["profile_id", "proxy"],
                "properties": {
                    "profile_id": { "type": "string" },
                    "proxy": {
                        "type": "object",
                        "required": ["server"],
                        "properties": {
                            "server": { "type": "string" },
                            "username": { "type": "string" },
                            "password": { "type": "string" }
                        }
                    },
                    "align_geo": { "type": "boolean" },
                    "check": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "launch_session",
            "Launch a browser session and return a CDP WebSocket URL for Playwright",
            json!({
                "type": "object",
                "required": ["profile_id"],
                "properties": {
                    "profile_id": { "type": "string" },
                    "headed": { "type": "boolean" },
                    "start_url": { "type": "string" },
                    "ttl_seconds": { "type": "integer" },
                    "force": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "stop_session",
            "Stop a running browser session",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" }
                }
            }),
        ),
        tool(
            "list_sessions",
            "List sessions",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "get_session_cdp_url",
            "Get CDP WebSocket URL for a session",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" }
                }
            }),
        ),
        tool(
            "heartbeat_session",
            "Extend session TTL",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" }
                }
            }),
        ),
        tool(
            "import_cookies",
            "Import cookies into a profile (applied on next launch via CDP)",
            json!({
                "type": "object",
                "required": ["profile_id", "cookies"],
                "properties": {
                    "profile_id": { "type": "string" },
                    "cookies": { "type": "array" },
                    "merge": { "type": "boolean" }
                }
            }),
        ),
        tool(
            "export_cookies",
            "Export cookies blob for a profile",
            json!({
                "type": "object",
                "required": ["profile_id"],
                "properties": {
                    "profile_id": { "type": "string" }
                }
            }),
        ),
        tool(
            "doctor",
            "Run local environment health checks",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "ensure_ready",
            "Idempotent setup check for agents: data dir, API token, Chrome/Chromium, doctor. Call first before scraping.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "setup_scrape_profile",
            "One-shot: create a scrape profile with optional proxy (http(s)|socks5://user:pass@host:port) and cookies. Returns profile_id ready for launch_session.",
            json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string", "description": "Profile name, e.g. scrape-domain.com" },
                    "proxy": {
                        "type": "string",
                        "description": "Proxy URL: http://host:port or http://user:pass@host:port or socks5://..."
                    },
                    "cookies": {
                        "type": "array",
                        "description": "Cookie objects {name,value,domain,...} applied on next launch"
                    },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "os": { "type": "string" },
                    "template": { "type": "string" }
                }
            }),
        ),
        tool(
            "setup_mobile_profile",
            "Create an Android/mobile phone profile (Pixel-class). Launch applies CDP mobile viewport, UA, and touch. Use for mobile web flows (e.g. Reddit register).",
            json!({
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string" },
                    "device": {
                        "type": "string",
                        "description": "pixel_7 | pixel_8 (default pixel_7). Currently all map to android_chrome_pixel template."
                    },
                    "proxy": { "type": "string" },
                    "cookies": { "type": "array" },
                    "tags": { "type": "array", "items": { "type": "string" } }
                }
            }),
        ),
        tool(
            "page_navigate",
            "Navigate the live session browser to a URL (native CDP, no Playwright). Returns page url/title/html snapshot.",
            json!({
                "type": "object",
                "required": ["session_id", "url"],
                "properties": {
                    "session_id": { "type": "string" },
                    "url": { "type": "string" }
                }
            }),
        ),
        tool(
            "page_content",
            "Get current page content from a live session (html or text). Native CDP — no Playwright.",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" },
                    "mode": { "type": "string", "enum": ["html", "text"], "description": "Default html" }
                }
            }),
        ),
        tool(
            "page_links",
            "Extract links from the current page. Use same_host_only=true for domain-wide scrapes.",
            json!({
                "type": "object",
                "required": ["session_id"],
                "properties": {
                    "session_id": { "type": "string" },
                    "same_host_only": { "type": "boolean", "description": "Default true" }
                }
            }),
        ),
        tool(
            "page_evaluate",
            "Run JavaScript in the page and return the JSON value. Native CDP.",
            json!({
                "type": "object",
                "required": ["session_id", "expression"],
                "properties": {
                    "session_id": { "type": "string" },
                    "expression": { "type": "string", "description": "JS expression, e.g. document.title" }
                }
            }),
        ),
        tool(
            "page_click",
            "Click an element by CSS selector on the live session page.",
            json!({
                "type": "object",
                "required": ["session_id", "selector"],
                "properties": {
                    "session_id": { "type": "string" },
                    "selector": { "type": "string" }
                }
            }),
        ),
        tool(
            "page_type",
            "Type text into an input matching a CSS selector (sets value + input event).",
            json!({
                "type": "object",
                "required": ["session_id", "selector", "text"],
                "properties": {
                    "session_id": { "type": "string" },
                    "selector": { "type": "string" },
                    "text": { "type": "string" }
                }
            }),
        ),
        // —— Mail / OTP (BYO Gmail IMAP) ——
        tool(
            "mail_status",
            "Show whether BYO Gmail/IMAP is configured (password never returned).",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "mail_connect",
            "Connect BYO Gmail (or generic IMAP). Prefer Gmail App Password (not account password). Saves encrypted credentials locally.",
            json!({
                "type": "object",
                "required": ["username", "password"],
                "properties": {
                    "provider": { "type": "string", "description": "gmail (default) or imap" },
                    "username": { "type": "string", "description": "e.g. you@gmail.com" },
                    "password": { "type": "string", "description": "Gmail App Password (16 chars)" },
                    "host": { "type": "string", "description": "IMAP host if not gmail" },
                    "port": { "type": "integer", "description": "Default 993" },
                    "folder": { "type": "string", "description": "Default INBOX" },
                    "name": { "type": "string" },
                    "test": { "type": "boolean", "description": "Test login (default true)" }
                }
            }),
        ),
        tool(
            "mail_disconnect",
            "Remove saved mail credentials from this machine.",
            json!({ "type": "object", "properties": {} }),
        ),
        tool(
            "mail_list",
            "List recent inbox messages (snippets + any extracted OTPs). Does not mark read.",
            json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Default 10, max 50" }
                }
            }),
        ),
        tool(
            "mail_wait_otp",
            "Poll inbox until a verification code arrives. Use after signup forms. Filters: from_contains (e.g. reddit.com), subject_contains, body_contains.",
            json!({
                "type": "object",
                "properties": {
                    "timeout_seconds": { "type": "integer", "description": "Default 120, max 600" },
                    "poll_seconds": { "type": "integer", "description": "Default 5" },
                    "from_contains": { "type": "string", "description": "e.g. reddit.com or noreply" },
                    "subject_contains": { "type": "string" },
                    "body_contains": { "type": "string" },
                    "to_contains": { "type": "string", "description": "Filter +alias recipient" },
                    "otp_regex": { "type": "string", "description": "Optional custom capture regex" },
                    "max_age_minutes": { "type": "integer" },
                    "scan_limit": { "type": "integer" }
                }
            }),
        ),
        tool(
            "mail_handoff",
            "Human-in-the-loop: return instructions asking the user to paste an OTP (when BYO mail unavailable).",
            json!({
                "type": "object",
                "properties": {
                    "context": { "type": "string", "description": "e.g. Reddit email verification" },
                    "email_hint": { "type": "string" }
                }
            }),
        ),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

async fn handle_tool_call(service: Arc<OpenAntyService>, params: Value) -> Result<Value, String> {
    let name = params
        .get("name")
        .and_then(|n| n.as_str())
        .ok_or_else(|| "missing tool name".to_string())?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let rid = OpenAntyService::request_id();

    let payload = match name {
        "create_profile" => {
            let req: CreateProfileRequest =
                serde_json::from_value(args).map_err(|e| e.to_string())?;
            let profile = service.create_profile(req).map_err(|e| e.to_string())?;
            serde_json::to_value(ProfileResult::success(rid, profile)).unwrap()
        }
        "list_profiles" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as u32;
            let items = service.list_profiles(limit).map_err(|e| e.to_string())?;
            serde_json::to_value(ProfileListResult {
                ok: true,
                error: None,
                request_id: rid,
                items,
                next_cursor: None,
            })
            .unwrap()
        }
        "get_profile" => {
            let id = args
                .get("profile_id")
                .and_then(|v| v.as_str())
                .ok_or("profile_id required")?;
            let secrets = args
                .get("include_secrets")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let profile = service
                .get_profile(id, secrets)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(ProfileResult::success(rid, profile)).unwrap()
        }
        "delete_profile" => {
            let id = args
                .get("profile_id")
                .and_then(|v| v.as_str())
                .ok_or("profile_id required")?;
            service.delete_profile(id).map_err(|e| e.to_string())?;
            serde_json::to_value(OkResult::success(rid, Some("deleted".into()))).unwrap()
        }
        "apply_proxy" => {
            let profile_id = args
                .get("profile_id")
                .and_then(|v| v.as_str())
                .ok_or("profile_id required")?
                .to_string();
            let req = ApplyProxyRequest {
                proxy: serde_json::from_value(
                    args.get("proxy").cloned().unwrap_or(json!({})),
                )
                .map_err(|e| e.to_string())?,
                align_geo: args
                    .get("align_geo")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                check: args.get("check").and_then(|v| v.as_bool()).unwrap_or(true),
            };
            let (profile, proxy_status, regenerated) = service
                .apply_proxy(&profile_id, req)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(ProxyApplyResult {
                ok: true,
                error: None,
                request_id: rid,
                profile_id: Some(profile.id),
                proxy_status: Some(proxy_status),
                fingerprint_hash: Some(profile.fingerprint_hash),
                fingerprint_regenerated: regenerated,
            })
            .unwrap()
        }
        "launch_session" => {
            let req: LaunchSessionRequest =
                serde_json::from_value(args).map_err(|e| e.to_string())?;
            let session = service
                .launch_session(req)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(SessionResult::success(rid, session)).unwrap()
        }
        "stop_session" => {
            let id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or("session_id required")?;
            service
                .stop_session(id)
                .await
                .map_err(|e| e.to_string())?;
            serde_json::to_value(OkResult::success(rid, Some("stopped".into()))).unwrap()
        }
        "list_sessions" => {
            let items = service.list_sessions().map_err(|e| e.to_string())?;
            serde_json::to_value(SessionListResult {
                ok: true,
                error: None,
                request_id: rid,
                items,
                next_cursor: None,
            })
            .unwrap()
        }
        "get_session_cdp_url" | "heartbeat_session" => {
            let id = args
                .get("session_id")
                .and_then(|v| v.as_str())
                .ok_or("session_id required")?;
            let session = if name == "heartbeat_session" {
                service.heartbeat(id).map_err(|e| e.to_string())?
            } else {
                service.get_session(id).map_err(|e| e.to_string())?
            };
            serde_json::to_value(SessionResult::success(rid, session)).unwrap()
        }
        "import_cookies" => {
            let profile_id = args
                .get("profile_id")
                .and_then(|v| v.as_str())
                .ok_or("profile_id required")?
                .to_string();
            let cookies: Vec<Cookie> = serde_json::from_value(
                args.get("cookies").cloned().unwrap_or(json!([])),
            )
            .map_err(|e| e.to_string())?;
            let merge = args.get("merge").and_then(|v| v.as_bool()).unwrap_or(true);
            let (imported, skipped, pending) = service
                .import_cookies(&profile_id, cookies, merge)
                .map_err(|e| e.to_string())?;
            serde_json::to_value(CookieImportResult {
                ok: true,
                error: None,
                request_id: rid,
                profile_id: Some(profile_id),
                imported,
                skipped_expired: skipped,
                merged: merge,
                applied_live: false,
                cookies_pending_apply: pending,
                failed: vec![],
            })
            .unwrap()
        }
        "export_cookies" => {
            let profile_id = args
                .get("profile_id")
                .and_then(|v| v.as_str())
                .ok_or("profile_id required")?
                .to_string();
            let cookies = service
                .export_cookies(&profile_id)
                .map_err(|e| e.to_string())?;
            let count = cookies.len() as u32;
            serde_json::to_value(CookieExportResult {
                ok: true,
                error: None,
                request_id: rid,
                profile_id: Some(profile_id),
                source: "blob".into(),
                cookies,
                count,
            })
            .unwrap()
        }
        "doctor" => service.doctor(),
        "ensure_ready" => {
            let doctor = service.doctor();
            let ready = doctor.get("ok") == Some(&json!(true));
            json!({
                "ok": ready,
                "request_id": rid,
                "ready": ready,
                "data_dir": service.data_dir.display().to_string(),
                "version": env!("CARGO_PKG_VERSION"),
                "doctor": doctor,
                "next_steps": if ready {
                    json!([
                        "setup_scrape_profile or create_profile",
                        "optional apply_proxy / import_cookies",
                        "launch_session",
                        "page_navigate / page_links / page_content (no Playwright needed)",
                        "stop_session"
                    ])
                } else {
                    json!([
                        "Install Google Chrome or Chromium",
                        "Or set OPENANTY_BROWSER_PATH to a chromium binary",
                        "Re-run ensure_ready / doctor"
                    ])
                },
                "hint": "For agents: npx -y openanty@latest mcp"
            })
        }
        "setup_mobile_profile" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("name required")?
                .to_string();
            let mut tags: Vec<String> = args
                .get("tags")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            for t in ["mobile", "android"] {
                if !tags.iter().any(|x| x == t) {
                    tags.push(t.into());
                }
            }
            let device = args
                .get("device")
                .and_then(|v| v.as_str())
                .unwrap_or("pixel_7");
            let req = CreateProfileRequest {
                name: name.clone(),
                template: Some("android_chrome_pixel".into()),
                os: Some("android".into()),
                proxy: None,
                fingerprint_overrides: None,
                tags: Some(tags),
                notes: Some(format!("mobile profile device={device}")),
            };
            let profile = service.create_profile(req).map_err(|e| e.to_string())?;
            let mut proxy_status = None;
            if let Some(proxy_str) = args.get("proxy").and_then(|v| v.as_str()) {
                if !proxy_str.is_empty() {
                    let proxy = parse_proxy_url(proxy_str).map_err(|e| e.to_string())?;
                    let (_p, status, _regen) = service
                        .apply_proxy(
                            &profile.id,
                            ApplyProxyRequest {
                                proxy,
                                align_geo: true,
                                check: true,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    proxy_status = Some(status);
                }
            }
            if let Some(cookies_val) = args.get("cookies") {
                if !cookies_val.is_null() {
                    let cookies: Vec<Cookie> =
                        serde_json::from_value(cookies_val.clone()).map_err(|e| e.to_string())?;
                    if !cookies.is_empty() {
                        let _ = service
                            .import_cookies(&profile.id, cookies, true)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
            let profile = service
                .get_profile(&profile.id, true)
                .map_err(|e| e.to_string())?;
            json!({
                "ok": true,
                "request_id": rid,
                "profile_id": profile.id,
                "mobile": true,
                "template": "android_chrome_pixel",
                "device": device,
                "user_agent": profile.fingerprint.as_ref().map(|f| f.user_agent.clone()),
                "screen": profile.fingerprint.as_ref().map(|f| json!({
                    "width": f.screen.width,
                    "height": f.screen.height,
                    "dpr": f.screen.device_pixel_ratio
                })),
                "proxy_status": proxy_status,
                "next": "launch_session with headed=true; mobile CDP metrics apply automatically"
            })
        }
        "setup_scrape_profile" => {
            let name = args
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or("name required")?
                .to_string();
            let mut tags: Vec<String> = args
                .get("tags")
                .and_then(|v| serde_json::from_value(v.clone()).ok())
                .unwrap_or_default();
            if !tags.iter().any(|t| t == "scrape") {
                tags.push("scrape".into());
            }
            let req = CreateProfileRequest {
                name: name.clone(),
                template: args
                    .get("template")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                os: args
                    .get("os")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                proxy: None,
                fingerprint_overrides: None,
                tags: Some(tags),
                notes: Some(format!("agent scrape profile: {name}")),
            };
            let profile = service.create_profile(req).map_err(|e| e.to_string())?;
            let mut proxy_status = None;
            let mut fingerprint_regenerated = false;
            if let Some(proxy_str) = args.get("proxy").and_then(|v| v.as_str()) {
                if !proxy_str.is_empty() {
                    let proxy = parse_proxy_url(proxy_str).map_err(|e| e.to_string())?;
                    let (p, status, regen) = service
                        .apply_proxy(
                            &profile.id,
                            ApplyProxyRequest {
                                proxy,
                                align_geo: true,
                                check: true,
                            },
                        )
                        .await
                        .map_err(|e| e.to_string())?;
                    proxy_status = Some(status);
                    fingerprint_regenerated = regen;
                    let _ = p;
                }
            }
            let mut cookies_imported = 0u32;
            let mut cookies_pending = false;
            if let Some(cookies_val) = args.get("cookies") {
                if !cookies_val.is_null() {
                    let cookies: Vec<Cookie> =
                        serde_json::from_value(cookies_val.clone()).map_err(|e| e.to_string())?;
                    if !cookies.is_empty() {
                        let (imported, _skipped, pending) = service
                            .import_cookies(&profile.id, cookies, true)
                            .map_err(|e| e.to_string())?;
                        cookies_imported = imported;
                        cookies_pending = pending;
                    }
                }
            }
            let profile = service
                .get_profile(&profile.id, false)
                .map_err(|e| e.to_string())?;
            json!({
                "ok": true,
                "request_id": rid,
                "profile_id": profile.id,
                "profile": profile,
                "proxy_status": proxy_status,
                "fingerprint_regenerated": fingerprint_regenerated,
                "cookies_imported": cookies_imported,
                "cookies_pending_apply": cookies_pending,
                "next": "launch_session with this profile_id, then page_navigate / page_links / page_content"
            })
        }
        "page_navigate" => {
            let session_id = arg_str(&args, "session_id")?;
            let url = arg_str(&args, "url")?;
            let cdp = session_cdp(&service, &session_id)?;
            let page = cdp_page::page_navigate(&cdp, &url)
                .await
                .map_err(|e| e.to_string())?;
            page_payload(rid, &session_id, page, None)
        }
        "page_content" => {
            let session_id = arg_str(&args, "session_id")?;
            let mode = args
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("html");
            let cdp = session_cdp(&service, &session_id)?;
            let page = cdp_page::page_content(&cdp, mode)
                .await
                .map_err(|e| e.to_string())?;
            page_payload(rid, &session_id, page, None)
        }
        "page_links" => {
            let session_id = arg_str(&args, "session_id")?;
            let same_host = args
                .get("same_host_only")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);
            let cdp = session_cdp(&service, &session_id)?;
            let links = cdp_page::page_links(&cdp, same_host)
                .await
                .map_err(|e| e.to_string())?;
            let count = links.len();
            json!({
                "ok": true,
                "request_id": rid,
                "session_id": session_id,
                "same_host_only": same_host,
                "count": count,
                "links": links
            })
        }
        "page_evaluate" => {
            let session_id = arg_str(&args, "session_id")?;
            let expression = arg_str(&args, "expression")?;
            let cdp = session_cdp(&service, &session_id)?;
            let value = cdp_page::page_evaluate(&cdp, &expression)
                .await
                .map_err(|e| e.to_string())?;
            json!({
                "ok": true,
                "request_id": rid,
                "session_id": session_id,
                "value": value
            })
        }
        "page_click" => {
            let session_id = arg_str(&args, "session_id")?;
            let selector = arg_str(&args, "selector")?;
            let cdp = session_cdp(&service, &session_id)?;
            cdp_page::page_click(&cdp, &selector)
                .await
                .map_err(|e| e.to_string())?;
            json!({
                "ok": true,
                "request_id": rid,
                "session_id": session_id,
                "clicked": selector
            })
        }
        "page_type" => {
            let session_id = arg_str(&args, "session_id")?;
            let selector = arg_str(&args, "selector")?;
            let text = arg_str(&args, "text")?;
            let cdp = session_cdp(&service, &session_id)?;
            cdp_page::page_type(&cdp, &selector, &text)
                .await
                .map_err(|e| e.to_string())?;
            json!({
                "ok": true,
                "request_id": rid,
                "session_id": session_id,
                "typed": true,
                "selector": selector
            })
        }
        "mail_status" => service.mail_status().map_err(|e| e.to_string())?,
        "mail_connect" => {
            let username = arg_str(&args, "username")?;
            let password = arg_str(&args, "password")?;
            let provider = args
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("gmail");
            let test = args.get("test").and_then(|v| v.as_bool()).unwrap_or(true);
            service
                .mail_connect(
                    provider,
                    &username,
                    &password,
                    args.get("host").and_then(|v| v.as_str()),
                    args.get("port").and_then(|v| v.as_u64()).map(|p| p as u16),
                    args.get("folder").and_then(|v| v.as_str()),
                    args.get("name").and_then(|v| v.as_str()),
                    test,
                )
                .map_err(|e| e.to_string())?
        }
        "mail_disconnect" => service.mail_disconnect().map_err(|e| e.to_string())?,
        "mail_list" => {
            let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as u32;
            service.mail_list(limit).map_err(|e| e.to_string())?
        }
        "mail_wait_otp" => {
            let req: openanty_core::mail::WaitOtpRequest =
                serde_json::from_value(args).map_err(|e| e.to_string())?;
            let result = service.mail_wait_otp(req).await.map_err(|e| e.to_string())?;
            serde_json::to_value(result).unwrap_or(json!({ "ok": false }))
        }
        "mail_handoff" => {
            let context = args
                .get("context")
                .and_then(|v| v.as_str())
                .unwrap_or("email verification");
            let email_hint = args
                .get("email_hint")
                .and_then(|v| v.as_str())
                .unwrap_or("(the signup email)");
            json!({
                "ok": true,
                "request_id": rid,
                "needs_human": true,
                "message": format!(
                    "Please open the inbox for {email_hint}, find the {context} message, and paste the verification code here. Then the agent will call page tools to enter it."
                ),
                "instructions": [
                    "Open your Gmail (or the inbox used at signup)",
                    "Find the latest verification email",
                    "Copy the 6-digit (or 4–8 digit) code",
                    "Paste the code in chat for the agent"
                ],
                "alternative": "Or configure BYO Gmail: mail_connect with an App Password, then mail_wait_otp"
            })
        }
        other => return Err(format!("unknown tool: {other}")),
    };

    Ok(json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string())
        }],
        "structuredContent": payload,
        "isError": payload.get("ok") == Some(&json!(false))
    }))
}

fn arg_str(args: &Value, key: &str) -> Result<String, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("{key} required"))
}

fn session_cdp(service: &OpenAntyService, session_id: &str) -> Result<String, String> {
    let ses = service.get_session(session_id).map_err(|e| e.to_string())?;
    ses.cdp_ws_url
        .filter(|u| !u.is_empty())
        .ok_or_else(|| "session has no cdp_ws_url (not running or already stopped)".into())
}

fn page_payload(
    rid: String,
    session_id: &str,
    page: cdp_page::PageResult,
    extra: Option<Value>,
) -> Value {
    let mut out = json!({
        "ok": true,
        "request_id": rid,
        "session_id": session_id,
        "url": page.url,
        "title": page.title,
        "content_type": page.content_type,
        "content": page.content,
        "content_length": page.content.len()
    });
    if let Some(Value::Object(map)) = extra {
        if let Some(obj) = out.as_object_mut() {
            for (k, v) in map {
                obj.insert(k, v);
            }
        }
    }
    out
}

/// Parse proxy URLs used by agents: `http://user:pass@host:port`, `socks5://host:port`, or bare `host:port`.
fn parse_proxy_url(raw: &str) -> Result<ProxyConfig, String> {
    let s = raw.trim();
    if s.is_empty() {
        return Err("empty proxy string".into());
    }
    // Already structured JSON-ish path not used; URL form only.
    let (scheme, rest) = if let Some(idx) = s.find("://") {
        (&s[..idx], &s[idx + 3..])
    } else {
        ("http", s)
    };
    let scheme = scheme.to_ascii_lowercase();
    if !matches!(
        scheme.as_str(),
        "http" | "https" | "socks5" | "socks5h" | "socks4"
    ) {
        return Err(format!("unsupported proxy scheme: {scheme}"));
    }
    let (userinfo, hostport) = if let Some(at) = rest.rfind('@') {
        (Some(&rest[..at]), &rest[at + 1..])
    } else {
        (None, rest)
    };
    let hostport = hostport.trim_end_matches('/');
    if hostport.is_empty() {
        return Err("proxy host:port missing".into());
    }
    let (username, password) = if let Some(ui) = userinfo {
        if let Some(colon) = ui.find(':') {
            (
                Some(ui[..colon].to_string()),
                Some(ui[colon + 1..].to_string()),
            )
        } else {
            (Some(ui.to_string()), None)
        }
    } else {
        (None, None)
    };
    let server = format!("{scheme}://{hostport}");
    Ok(ProxyConfig {
        server,
        username,
        password,
        check_timeout_ms: 8000,
        check_url: None,
    })
}
