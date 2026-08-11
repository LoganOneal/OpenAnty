//! High-level page control over an existing CDP WebSocket (no Playwright required).

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message};

type Ws = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
type WriteHalf = futures_util::stream::SplitSink<Ws, Message>;
type ReadHalf = futures_util::stream::SplitStream<Ws>;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

pub struct PageResult {
    pub url: String,
    pub title: String,
    pub content: String,
    pub content_type: String,
}

async fn open_page_target(cdp_ws: &str) -> Result<(WriteHalf, ReadHalf, String), String> {
    let (ws, _) = tokio::time::timeout(Duration::from_secs(5), connect_async(cdp_ws))
        .await
        .map_err(|_| "cdp connect timeout".to_string())?
        .map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();

    let targets = session_call(&mut write, &mut read, 1, "", "Target.getTargets", json!({})).await?;
    let list = targets
        .get("targetInfos")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();

    let page = list
        .iter()
        .find(|t| t.get("type").and_then(|x| x.as_str()) == Some("page"))
        .cloned();

    let target_id = if let Some(p) = page {
        p.get("targetId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    } else {
        let created = session_call(
            &mut write,
            &mut read,
            2,
            "",
            "Target.createTarget",
            json!({ "url": "about:blank" }),
        )
        .await?;
        created
            .get("targetId")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string()
    };
    if target_id.is_empty() {
        return Err("no page target available".into());
    }

    let attached = session_call(
        &mut write,
        &mut read,
        3,
        "",
        "Target.attachToTarget",
        json!({ "targetId": target_id, "flatten": true }),
    )
    .await?;
    let session_id = attached
        .get("sessionId")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();

    Ok((write, read, session_id))
}

async fn session_call(
    write: &mut WriteHalf,
    read: &mut ReadHalf,
    id: i64,
    session_id: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let mut msg = json!({ "id": id, "method": method, "params": params });
    if !session_id.is_empty() {
        msg["sessionId"] = json!(session_id);
    }
    write
        .send(Message::Text(msg.to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    let deadline = tokio::time::Instant::now() + DEFAULT_TIMEOUT;
    while tokio::time::Instant::now() < deadline {
        match tokio::time::timeout(Duration::from_secs(5), read.next()).await {
            Ok(Some(Ok(Message::Text(t)))) => {
                let v: Value = serde_json::from_str(&t).map_err(|e| e.to_string())?;
                if v.get("id").and_then(|x| x.as_i64()) == Some(id) {
                    if let Some(err) = v.get("error") {
                        return Err(err.to_string());
                    }
                    return Ok(v.get("result").cloned().unwrap_or(json!({})));
                }
            }
            Ok(Some(Ok(_))) => continue,
            Ok(Some(Err(e))) => return Err(e.to_string()),
            Ok(None) => return Err("cdp closed".into()),
            Err(_) => continue,
        }
    }
    Err(format!("timeout on {method}"))
}

async fn page_snapshot(
    write: &mut WriteHalf,
    read: &mut ReadHalf,
    sid: &str,
    mode: &str,
) -> Result<PageResult, String> {
    let expr = match mode {
        "text" => {
            "({url: location.href, title: document.title, content: document.body ? document.body.innerText : '', content_type: 'text'})"
        }
        _ => {
            "({url: location.href, title: document.title, content: document.documentElement ? document.documentElement.outerHTML : '', content_type: 'html'})"
        }
    };
    let result = session_call(
        write,
        read,
        50,
        sid,
        "Runtime.evaluate",
        json!({
            "expression": expr,
            "returnByValue": true,
            "awaitPromise": true
        }),
    )
    .await?;
    let val = result
        .pointer("/result/value")
        .cloned()
        .unwrap_or(json!({}));
    Ok(PageResult {
        url: val.get("url").and_then(|x| x.as_str()).unwrap_or("").into(),
        title: val.get("title").and_then(|x| x.as_str()).unwrap_or("").into(),
        content: val
            .get("content")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .into(),
        content_type: val
            .get("content_type")
            .and_then(|x| x.as_str())
            .unwrap_or(mode)
            .into(),
    })
}

pub async fn page_navigate(cdp_ws: &str, url: &str) -> Result<PageResult, String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 10, &sid, "Page.enable", json!({})).await;
    let _ = session_call(&mut write, &mut read, 11, &sid, "Runtime.enable", json!({})).await;
    let _ = session_call(
        &mut write,
        &mut read,
        12,
        &sid,
        "Page.navigate",
        json!({ "url": url }),
    )
    .await?;
    for i in 0..40i64 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        let state = session_call(
            &mut write,
            &mut read,
            100 + i,
            &sid,
            "Runtime.evaluate",
            json!({ "expression": "document.readyState", "returnByValue": true }),
        )
        .await
        .ok()
        .and_then(|r| {
            r.pointer("/result/value")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
        });
        if matches!(state.as_deref(), Some("complete") | Some("interactive")) {
            break;
        }
    }
    page_snapshot(&mut write, &mut read, &sid, "html").await
}

pub async fn page_content(cdp_ws: &str, mode: &str) -> Result<PageResult, String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 20, &sid, "Runtime.enable", json!({})).await;
    page_snapshot(&mut write, &mut read, &sid, mode).await
}

pub async fn page_links(cdp_ws: &str, same_host_only: bool) -> Result<Vec<String>, String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 30, &sid, "Runtime.enable", json!({})).await;
    let expr = if same_host_only {
        r#"[...document.querySelectorAll('a[href]')].map(a => a.href).filter(h => { try { return new URL(h).host === location.host } catch { return false } })"#
    } else {
        r#"[...document.querySelectorAll('a[href]')].map(a => a.href)"#
    };
    let result = session_call(
        &mut write,
        &mut read,
        31,
        &sid,
        "Runtime.evaluate",
        json!({ "expression": expr, "returnByValue": true }),
    )
    .await?;
    let arr = result
        .pointer("/result/value")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut out = Vec::new();
    for v in arr {
        if let Some(s) = v.as_str() {
            if !s.is_empty() && !out.iter().any(|x| x == s) {
                out.push(s.to_string());
            }
        }
    }
    Ok(out)
}

pub async fn page_click(cdp_ws: &str, selector: &str) -> Result<(), String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 40, &sid, "Runtime.enable", json!({})).await;
    let expr = format!(
        r#"(() => {{ const el = document.querySelector({sel}); if (!el) throw new Error('not found'); el.click(); return true; }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into())
    );
    let _ = session_call(
        &mut write,
        &mut read,
        41,
        &sid,
        "Runtime.evaluate",
        json!({ "expression": expr, "returnByValue": true }),
    )
    .await?;
    Ok(())
}

pub async fn page_type(cdp_ws: &str, selector: &str, text: &str) -> Result<(), String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 42, &sid, "Runtime.enable", json!({})).await;
    let expr = format!(
        r#"(() => {{ const el = document.querySelector({sel}); if (!el) throw new Error('not found'); el.focus(); el.value = {val}; el.dispatchEvent(new Event('input', {{ bubbles: true }})); return true; }})()"#,
        sel = serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".into()),
        val = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into())
    );
    let _ = session_call(
        &mut write,
        &mut read,
        43,
        &sid,
        "Runtime.evaluate",
        json!({ "expression": expr, "returnByValue": true }),
    )
    .await?;
    Ok(())
}

/// Apply CDP mobile device emulation from fingerprint-like dimensions / UA.
pub async fn apply_mobile_emulation(
    cdp_ws: &str,
    width: u32,
    height: u32,
    device_scale_factor: f64,
    user_agent: &str,
    platform: &str,
    languages: &[String],
    max_touch_points: u32,
    mobile: bool,
    model: &str,
    ua_full_version: &str,
) -> Result<(), String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 60, &sid, "Page.enable", json!({})).await;
    let _ = session_call(&mut write, &mut read, 61, &sid, "Network.enable", json!({})).await;
    let _ = session_call(
        &mut write,
        &mut read,
        62,
        &sid,
        "Emulation.setDeviceMetricsOverride",
        json!({
            "width": width,
            "height": height,
            "deviceScaleFactor": device_scale_factor,
            "mobile": mobile,
            "screenWidth": width,
            "screenHeight": height,
        }),
    )
    .await?;
    let _ = session_call(
        &mut write,
        &mut read,
        63,
        &sid,
        "Emulation.setTouchEmulationEnabled",
        json!({
            "enabled": mobile || max_touch_points > 0,
            "maxTouchPoints": max_touch_points.max(1),
        }),
    )
    .await;
    let accept_lang = if languages.is_empty() {
        "en-US,en;q=0.9".to_string()
    } else {
        let mut parts = Vec::new();
        for (i, l) in languages.iter().enumerate() {
            if i == 0 {
                parts.push(l.clone());
            } else {
                let q = (10 - i.min(9)) as f64 / 10.0;
                parts.push(format!("{l};q={q}"));
            }
        }
        parts.join(",")
    };
    let ua = user_agent.to_string();
    let major = ua_full_version
        .split('.')
        .next()
        .unwrap_or("130")
        .to_string();
    let plat = if mobile { "Linux armv8l" } else { platform };
    let ua_params = json!({
        "userAgent": ua,
        "acceptLanguage": accept_lang,
        "platform": plat,
    });
    // Network + Emulation overrides (both for max compatibility)
    let _ = session_call(
        &mut write,
        &mut read,
        64,
        &sid,
        "Network.setUserAgentOverride",
        ua_params.clone(),
    )
    .await;
    let meta = json!({
        "brands": [
            { "brand": "Chromium", "version": major },
            { "brand": "Google Chrome", "version": major },
            { "brand": "Not_A Brand", "version": "24" }
        ],
        "fullVersionList": [
            { "brand": "Chromium", "version": ua_full_version },
            { "brand": "Google Chrome", "version": ua_full_version },
            { "brand": "Not_A Brand", "version": "10.0.0.24" }
        ],
        "fullVersion": ua_full_version,
        "platform": if mobile { "Android" } else { "Windows" },
        "platformVersion": if mobile { "14.0.0" } else { "15.0.0" },
        "architecture": if mobile { "" } else { "x86" },
        "model": model,
        "mobile": mobile,
        "bitness": "64",
        "wow64": false
    });
    let mut emu = ua_params;
    emu["userAgentMetadata"] = meta;
    let _ = session_call(
        &mut write,
        &mut read,
        66,
        &sid,
        "Emulation.setUserAgentOverride",
        emu,
    )
    .await;

    // Persist UA/touch/platform for navigations via init script (hardens flaky CDP UA).
    let touch = max_touch_points.max(if mobile { 5 } else { 0 });
    let init = format!(
        r#"(function(){{
  try {{
    Object.defineProperty(navigator, 'userAgent', {{ get: () => {ua} }});
    Object.defineProperty(navigator, 'platform', {{ get: () => {plat} }});
    Object.defineProperty(navigator, 'maxTouchPoints', {{ get: () => {touch} }});
    Object.defineProperty(navigator, 'webdriver', {{ get: () => undefined }});
  }} catch(e) {{}}
}})();"#,
        ua = serde_json::to_string(&ua).unwrap_or_else(|_| "\"\"".into()),
        plat = serde_json::to_string(plat).unwrap_or_else(|_| "\"\"".into()),
        touch = touch,
    );
    let _ = session_call(
        &mut write,
        &mut read,
        67,
        &sid,
        "Page.addScriptToEvaluateOnNewDocument",
        json!({ "source": init }),
    )
    .await;
    let _ = session_call(
        &mut write,
        &mut read,
        68,
        &sid,
        "Runtime.evaluate",
        json!({ "expression": init, "returnByValue": true }),
    )
    .await;
    Ok(())
}

/// Run a JavaScript expression in the page context and return the value as JSON.
pub async fn page_evaluate(cdp_ws: &str, expression: &str) -> Result<Value, String> {
    let (mut write, mut read, sid) = open_page_target(cdp_ws).await?;
    let _ = session_call(&mut write, &mut read, 44, &sid, "Runtime.enable", json!({})).await;
    let result = session_call(
        &mut write,
        &mut read,
        45,
        &sid,
        "Runtime.evaluate",
        json!({
            "expression": expression,
            "returnByValue": true,
            "awaitPromise": true
        }),
    )
    .await?;
    if let Some(exc) = result.pointer("/exceptionDetails") {
        return Err(format!("page_evaluate exception: {exc}"));
    }
    Ok(result
        .pointer("/result/value")
        .cloned()
        .unwrap_or(Value::Null))
}
