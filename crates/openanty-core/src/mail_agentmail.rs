//! AgentMail provider — one-click agent inboxes (hands-off email for agents).
//!
//! API: https://docs.agentmail.to
//! Base: https://api.agentmail.to/v0/
//!
//! User once: API key from console.agentmail.to (or agent sign-up flow).
//! Then: create_inbox → address; wait_otp polls messages on that inbox.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

use crate::crypto::MasterKey;
use crate::mail::{extract_otp, WaitOtpRequest, WaitOtpResult};

const CRED_FILE: &str = "mail.agentmail.bin";
const BASE: &str = "https://api.agentmail.to/v0";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMailConfig {
    pub api_key: String,
    /// Default inbox used for wait_otp if not specified
    #[serde(default)]
    pub default_inbox_id: Option<String>,
    #[serde(default)]
    pub default_email: Option<String>,
    pub saved_at: DateTime<Utc>,
}

impl AgentMailConfig {
    pub fn public_status(&self) -> serde_json::Value {
        let preview = if self.api_key.len() > 10 {
            format!("{}…{}", &self.api_key[..4], &self.api_key[self.api_key.len() - 4..])
        } else {
            "(set)".into()
        };
        serde_json::json!({
            "configured": true,
            "provider": "agentmail",
            "api_key_preview": preview,
            "default_inbox_id": self.default_inbox_id,
            "default_email": self.default_email,
            "saved_at": self.saved_at.to_rfc3339(),
            "hands_off": true,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInbox {
    pub inbox_id: String,
    pub email: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub pod_id: Option<String>,
}

pub fn save_config(data_dir: &Path, key: &MasterKey, cfg: &AgentMailConfig) -> Result<(), String> {
    crate::paths::ensure_dir(data_dir).map_err(|e| e.to_string())?;
    let blob = key.encrypt_json(cfg)?;
    std::fs::write(data_dir.join(CRED_FILE), blob).map_err(|e| e.to_string())
}

pub fn load_config(data_dir: &Path, key: &MasterKey) -> Result<Option<AgentMailConfig>, String> {
    if let Ok(env_key) = std::env::var("AGENTMAIL_API_KEY")
        .or_else(|_| std::env::var("OPENANTY_AGENTMAIL_API_KEY"))
    {
        if !env_key.trim().is_empty() {
            return Ok(Some(AgentMailConfig {
                api_key: env_key.trim().to_string(),
                default_inbox_id: std::env::var("OPENANTY_AGENTMAIL_INBOX_ID").ok(),
                default_email: std::env::var("OPENANTY_AGENTMAIL_EMAIL").ok(),
                saved_at: Utc::now(),
            }));
        }
    }
    let path = data_dir.join(CRED_FILE);
    if !path.exists() {
        return Ok(None);
    }
    let blob = std::fs::read(path).map_err(|e| e.to_string())?;
    key.decrypt_json(&blob).map(Some)
}

pub fn clear_config(data_dir: &Path) -> Result<(), String> {
    let path = data_dir.join(CRED_FILE);
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(45))
        .user_agent("OpenAnty/0.1 (AgentMail provider)")
        .build()
        .map_err(|e| e.to_string())
}

/// Create a new AgentMail inbox (one click / one command).
pub async fn create_inbox(
    api_key: &str,
    username: Option<&str>,
    display_name: Option<&str>,
    client_id: Option<&str>,
) -> Result<AgentInbox, String> {
    let mut body = serde_json::Map::new();
    if let Some(u) = username {
        if !u.is_empty() {
            body.insert("username".into(), serde_json::json!(u));
        }
    }
    if let Some(d) = display_name {
        body.insert("display_name".into(), serde_json::json!(d));
    }
    if let Some(c) = client_id {
        body.insert("client_id".into(), serde_json::json!(c));
    } else {
        body.insert(
            "client_id".into(),
            serde_json::json!(format!("openanty-{}", uuid::Uuid::new_v4().simple())),
        );
    }

    let http = client()?;
    let resp = http
        .post(format!("{BASE}/inboxes"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("AgentMail create inbox request failed: {e}"))?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("AgentMail create inbox HTTP {status}: {text}"));
    }
    let v: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("parse: {e} body={text}"))?;

    let inbox_id = v
        .get("inbox_id")
        .or_else(|| v.get("inboxId"))
        .and_then(|x| x.as_str())
        .ok_or_else(|| format!("missing inbox_id in {text}"))?
        .to_string();
    let email = v
        .get("email")
        .and_then(|x| x.as_str())
        .ok_or_else(|| format!("missing email in {text}"))?
        .to_string();

    Ok(AgentInbox {
        inbox_id,
        email,
        display_name: v
            .get("display_name")
            .or_else(|| v.get("displayName"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
        pod_id: v
            .get("pod_id")
            .or_else(|| v.get("podId"))
            .and_then(|x| x.as_str())
            .map(|s| s.to_string()),
    })
}

/// List recent messages (summaries).
pub async fn list_messages(
    api_key: &str,
    inbox_id: &str,
    limit: u32,
    from_contains: Option<&str>,
) -> Result<Vec<serde_json::Value>, String> {
    let http = client()?;
    let mut url = format!(
        "{BASE}/inboxes/{}/messages?limit={}",
        urlencoding_simple(inbox_id),
        limit.clamp(1, 50)
    );
    if let Some(f) = from_contains {
        url.push_str(&format!("&from={}", urlencoding_simple(f)));
    }
    let resp = http
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("AgentMail list messages HTTP {status}: {text}"));
    }
    let v: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::json!({}));
    let msgs = v
        .get("messages")
        .and_then(|m| m.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(msgs)
}

pub async fn get_message(
    api_key: &str,
    inbox_id: &str,
    message_id: &str,
) -> Result<serde_json::Value, String> {
    let http = client()?;
    let url = format!(
        "{BASE}/inboxes/{}/messages/{}",
        urlencoding_simple(inbox_id),
        urlencoding_simple(message_id)
    );
    let resp = http
        .get(&url)
        .header("Authorization", format!("Bearer {api_key}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("AgentMail get message HTTP {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// Poll AgentMail inbox for OTP.
pub async fn wait_for_otp(
    api_key: &str,
    inbox_id: &str,
    req: WaitOtpRequest,
) -> WaitOtpResult {
    let timeout = Duration::from_secs(req.timeout_seconds.clamp(5, 600));
    let poll = Duration::from_secs(req.poll_seconds.clamp(2, 60));
    let start = std::time::Instant::now();
    let mut polled = 0u32;

    loop {
        polled += 1;
        match poll_once(api_key, inbox_id, &req).await {
            Ok(Some((otp, summary))) => {
                return WaitOtpResult {
                    ok: true,
                    found: true,
                    otp: Some(otp),
                    message: Some(summary),
                    polled,
                    elapsed_seconds: start.elapsed().as_secs(),
                    error: None,
                    hint: None,
                };
            }
            Ok(None) => {}
            Err(e) => {
                return WaitOtpResult {
                    ok: false,
                    found: false,
                    otp: None,
                    message: None,
                    polled,
                    elapsed_seconds: start.elapsed().as_secs(),
                    error: Some(e),
                    hint: Some("Check AGENTMAIL_API_KEY and inbox_id".into()),
                };
            }
        }
        if start.elapsed() >= timeout {
            return WaitOtpResult {
                ok: true,
                found: false,
                otp: None,
                message: None,
                polled,
                elapsed_seconds: start.elapsed().as_secs(),
                error: None,
                hint: Some("No OTP yet in AgentMail inbox — check spam filters / from_contains".into()),
            };
        }
        tokio::time::sleep(poll).await;
    }
}

async fn poll_once(
    api_key: &str,
    inbox_id: &str,
    req: &WaitOtpRequest,
) -> Result<Option<(String, crate::mail::MailMessageSummary)>, String> {
    let msgs = list_messages(
        api_key,
        inbox_id,
        req.scan_limit.clamp(1, 30),
        req.from_contains.as_deref(),
    )
    .await?;

    for m in msgs {
        let mid = m
            .get("message_id")
            .or_else(|| m.get("messageId"))
            .and_then(|x| x.as_str())
            .unwrap_or("");
        let subject = m
            .get("subject")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();
        let from = m.get("from").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let preview = m
            .get("preview")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .to_string();

        if let Some(s) = &req.subject_contains {
            if !subject.to_lowercase().contains(&s.to_lowercase()) {
                continue;
            }
        }
        if let Some(f) = &req.from_contains {
            if !from.to_lowercase().contains(&f.to_lowercase()) {
                continue;
            }
        }

        let mut full = format!("{subject}\n{from}\n{preview}");
        if !mid.is_empty() {
            if let Ok(detail) = get_message(api_key, inbox_id, mid).await {
                for key in ["extracted_text", "extractedText", "text", "html", "preview", "subject"] {
                    if let Some(t) = detail.get(key).and_then(|x| x.as_str()) {
                        full.push('\n');
                        full.push_str(t);
                    }
                }
            }
        }
        if let Some(b) = &req.body_contains {
            if !full.to_lowercase().contains(&b.to_lowercase()) {
                continue;
            }
        }
        if let Some(otp) = extract_otp(&full, req.otp_regex.as_deref()) {
            let summary = crate::mail::MailMessageSummary {
                uid: 0,
                from,
                to: String::new(),
                subject,
                date: m
                    .get("timestamp")
                    .and_then(|x| x.as_str())
                    .unwrap_or("")
                    .to_string(),
                snippet: preview.chars().take(240).collect(),
                has_otp: true,
                otp: Some(otp.clone()),
            };
            return Ok(Some((otp, summary)));
        }
    }
    Ok(None)
}

/// Agent self sign-up (sends OTP to human_email).
pub async fn agent_sign_up(human_email: &str, username: &str) -> Result<serde_json::Value, String> {
    let http = client()?;
    let resp = http
        .post(format!("{BASE}/agent/sign-up"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "human_email": human_email,
            "username": username,
            "source": "openanty",
            "referrer": "openanty"
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("AgentMail sign-up HTTP {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("parse: {e} {text}"))
}

pub async fn agent_verify(api_key: &str, otp_code: &str) -> Result<serde_json::Value, String> {
    let http = client()?;
    let resp = http
        .post(format!("{BASE}/agent/verify"))
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({ "otp_code": otp_code }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!("AgentMail verify HTTP {status}: {text}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("parse: {e} {text}"))
}

fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '@' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}
