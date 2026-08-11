//! DuckDuckGo Email Protection — private `@duck.com` address generation.
//!
//! **Unofficial** internal API (same approach as Bitwarden / browser extension):
//! `POST https://quack.duckduckgo.com/api/email/addresses`
//! with `Authorization: Bearer <token>`.
//!
//! There is no official public DuckDuckGo developer API. Tokens are obtained from a
//! logged-in session on https://duckduckgo.com/email (see docs/skills/duckduckgo-email.md).
//! This may break without notice and is subject to daily rate limits / ToS.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::crypto::MasterKey;

const TOKEN_FILE: &str = "mail.duck.token.bin";
const API_URL: &str = "https://quack.duckduckgo.com/api/email/addresses";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckToken {
    /// Bearer token (never returned by status APIs in full)
    pub token: String,
    /// Optional personal address e.g. name@duck.com (for docs only)
    #[serde(default)]
    pub personal_address: Option<String>,
    pub saved_at: DateTime<Utc>,
}

impl DuckToken {
    pub fn public_status(&self) -> serde_json::Value {
        let preview = if self.token.len() > 12 {
            format!("{}…{}", &self.token[..6], &self.token[self.token.len() - 4..])
        } else {
            "(set)".into()
        };
        serde_json::json!({
            "configured": true,
            "provider": "duckduckgo",
            "token_preview": preview,
            "personal_address": self.personal_address,
            "saved_at": self.saved_at.to_rfc3339(),
            "experimental": true,
            "api": API_URL,
            "note": "Unofficial API — may break; obtain token from DDG Email Autofill Network tab"
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuckAliasResult {
    pub ok: bool,
    /// Full address e.g. random-words@duck.com
    pub address: Option<String>,
    /// Local part only (as returned by API)
    pub local_part: Option<String>,
    pub error: Option<String>,
    pub raw: Option<serde_json::Value>,
    pub hint: Option<String>,
}

pub fn token_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(TOKEN_FILE)
}

pub fn save_token(data_dir: &Path, key: &MasterKey, duck: &DuckToken) -> Result<(), String> {
    crate::paths::ensure_dir(data_dir).map_err(|e| e.to_string())?;
    let blob = key.encrypt_json(duck)?;
    std::fs::write(token_path(data_dir), blob).map_err(|e| e.to_string())
}

pub fn load_token(data_dir: &Path, key: &MasterKey) -> Result<Option<DuckToken>, String> {
    if let Ok(env_tok) = std::env::var("OPENANTY_DDG_TOKEN") {
        if !env_tok.trim().is_empty() {
            return Ok(Some(DuckToken {
                token: env_tok.trim().trim_start_matches("Bearer ").trim().to_string(),
                personal_address: std::env::var("OPENANTY_DDG_PERSONAL").ok(),
                saved_at: Utc::now(),
            }));
        }
    }
    let path = token_path(data_dir);
    if !path.exists() {
        return Ok(None);
    }
    let blob = std::fs::read(path).map_err(|e| e.to_string())?;
    key.decrypt_json(&blob).map(Some)
}

pub fn clear_token(data_dir: &Path) -> Result<(), String> {
    let path = token_path(data_dir);
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Normalize user paste of token (strip "Bearer " prefix / whitespace).
pub fn normalize_token(raw: &str) -> String {
    raw.trim()
        .trim_start_matches("Bearer ")
        .trim_start_matches("bearer ")
        .trim()
        .to_string()
}

/// Generate one private Duck Address via unofficial quack API.
pub async fn generate_private_address(token: &str) -> DuckAliasResult {
    let token = normalize_token(token);
    if token.is_empty() {
        return DuckAliasResult {
            ok: false,
            address: None,
            local_part: None,
            error: Some("empty token".into()),
            raw: None,
            hint: Some("Paste Bearer token from DDG Email Autofill → Generate Private Duck Address Network tab".into()),
        };
    }

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("OpenAnty/0.1 (DuckDuckGo email alias; experimental)")
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return DuckAliasResult {
                ok: false,
                address: None,
                local_part: None,
                error: Some(e.to_string()),
                raw: None,
                hint: None,
            };
        }
    };

    let resp = client
        .post(API_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/json")
        // Empty JSON body is typical for this endpoint
        .json(&serde_json::json!({}))
        .send()
        .await;

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            return DuckAliasResult {
                ok: false,
                address: None,
                local_part: None,
                error: Some(format!("request failed: {e}")),
                raw: None,
                hint: Some("Check network; unofficial API may be blocked or changed".into()),
            };
        }
    };

    let status = resp.status();
    let body_text = resp.text().await.unwrap_or_default();
    let raw: Option<serde_json::Value> = serde_json::from_str(&body_text).ok();

    if !status.is_success() {
        return DuckAliasResult {
            ok: false,
            address: None,
            local_part: None,
            error: Some(format!("HTTP {status}: {body_text}")),
            raw,
            hint: Some(
                "Token may be expired — re-copy Bearer from DevTools after Generate Private Duck Address"
                    .into(),
            ),
        };
    }

    // Response shapes seen in the wild:
    // { "address": "word-word-word" }
    // { "address": "word-word-word@duck.com" }
    // { "email": "..." }
    let local = raw
        .as_ref()
        .and_then(|v| {
            v.get("address")
                .or_else(|| v.get("email"))
                .and_then(|x| x.as_str())
                .map(|s| s.to_string())
        })
        .or_else(|| {
            // plain string body
            let t = body_text.trim().trim_matches('"');
            if !t.is_empty() && !t.starts_with('{') {
                Some(t.to_string())
            } else {
                None
            }
        });

    let Some(local) = local else {
        return DuckAliasResult {
            ok: false,
            address: None,
            local_part: None,
            error: Some(format!("unexpected response: {body_text}")),
            raw,
            hint: None,
        };
    };

    let address = if local.contains('@') {
        local.clone()
    } else {
        format!("{local}@duck.com")
    };
    let local_part = address
        .split('@')
        .next()
        .unwrap_or(&local)
        .to_string();

    DuckAliasResult {
        ok: true,
        address: Some(address),
        local_part: Some(local_part),
        error: None,
        raw,
        hint: Some(
            "Mail is forwarded to your DDG-linked inbox — use mail_wait_otp on that Gmail/IMAP"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_bearer() {
        assert_eq!(normalize_token("Bearer abc123"), "abc123");
        assert_eq!(normalize_token("  xyz  "), "xyz");
    }
}
