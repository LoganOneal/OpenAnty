//! Minimal MCP stdio server (JSON-RPC 2.0) for OpenAnty tools.

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
