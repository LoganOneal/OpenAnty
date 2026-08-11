//! BYO mail (Gmail IMAP / generic IMAP) for agent OTP extraction.
//!
//! Credentials are stored encrypted under the data dir (`mail.credentials.bin`).
//! Prefer Gmail App Passwords: https://myaccount.google.com/apppasswords

use chrono::{DateTime, Utc};
use imap::types::Fetch;
use mailparse::MailHeaderMap;
use native_tls::TlsConnector;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::crypto::MasterKey;

const CRED_FILE: &str = "mail.credentials.bin";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAccount {
    /// Logical name, e.g. "gmail-primary"
    pub name: String,
    /// IMAP host, default gmail
    pub host: String,
    pub port: u16,
    pub username: String,
    /// App password or IMAP password (never returned by status APIs)
    pub password: String,
    /// gmail | outlook | imap
    pub provider: String,
    #[serde(default = "default_folder")]
    pub folder: String,
    pub saved_at: DateTime<Utc>,
}

fn default_folder() -> String {
    "INBOX".into()
}

impl MailAccount {
    pub fn gmail(username: &str, app_password: &str) -> Self {
        Self {
            name: "gmail".into(),
            host: "imap.gmail.com".into(),
            port: 993,
            username: username.trim().into(),
            password: app_password.replace(' ', ""), // app passwords often shown with spaces
            provider: "gmail".into(),
            folder: "INBOX".into(),
            saved_at: Utc::now(),
        }
    }

    pub fn public_status(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "provider": self.provider,
            "host": self.host,
            "port": self.port,
            "username": self.username,
            "folder": self.folder,
            "saved_at": self.saved_at.to_rfc3339(),
            "password_configured": !self.password.is_empty(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessageSummary {
    pub uid: u32,
    pub from: String,
    pub to: String,
    pub subject: String,
    pub date: String,
    pub snippet: String,
    pub has_otp: bool,
    pub otp: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitOtpRequest {
    /// Seconds to poll (default 120, max 600)
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Poll interval seconds (default 5)
    #[serde(default = "default_poll")]
    pub poll_seconds: u64,
    /// Only messages whose From contains this (case-insensitive), e.g. "reddit.com"
    #[serde(default)]
    pub from_contains: Option<String>,
    /// Only subject contains
    #[serde(default)]
    pub subject_contains: Option<String>,
    /// Only body/subject contain this free text
    #[serde(default)]
    pub body_contains: Option<String>,
    /// Custom OTP regex (default 4–8 digit code)
    #[serde(default)]
    pub otp_regex: Option<String>,
    /// Only consider messages newer than this many minutes (default 30)
    #[serde(default = "default_max_age")]
    pub max_age_minutes: u64,
    /// Max messages to scan per poll (default 20)
    #[serde(default = "default_scan")]
    pub scan_limit: u32,
    /// If set, only messages TO this address (useful with +aliases)
    #[serde(default)]
    pub to_contains: Option<String>,
}

fn default_timeout() -> u64 {
    120
}
fn default_poll() -> u64 {
    5
}
fn default_max_age() -> u64 {
    30
}
fn default_scan() -> u32 {
    20
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitOtpResult {
    pub ok: bool,
    pub found: bool,
    pub otp: Option<String>,
    pub message: Option<MailMessageSummary>,
    pub polled: u32,
    pub elapsed_seconds: u64,
    pub error: Option<String>,
    pub hint: Option<String>,
}

pub fn credentials_path(data_dir: &Path) -> PathBuf {
    data_dir.join(CRED_FILE)
}

pub fn save_account(data_dir: &Path, key: &MasterKey, account: &MailAccount) -> Result<(), String> {
    crate::paths::ensure_dir(data_dir).map_err(|e| e.to_string())?;
    let blob = key.encrypt_json(account)?;
    std::fs::write(credentials_path(data_dir), blob).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_account(data_dir: &Path, key: &MasterKey) -> Result<Option<MailAccount>, String> {
    let path = credentials_path(data_dir);
    if !path.exists() {
        // Env fallback for agents / CI
        if let (Ok(user), Ok(pass)) = (
            std::env::var("OPENANTY_MAIL_USER"),
            std::env::var("OPENANTY_MAIL_PASSWORD"),
        ) {
            let host = std::env::var("OPENANTY_MAIL_HOST")
                .unwrap_or_else(|_| "imap.gmail.com".into());
            let port: u16 = std::env::var("OPENANTY_MAIL_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(993);
            let provider = if host.contains("gmail") {
                "gmail"
            } else {
                "imap"
            };
            return Ok(Some(MailAccount {
                name: provider.into(),
                host,
                port,
                username: user,
                password: pass.replace(' ', ""),
                provider: provider.into(),
                folder: std::env::var("OPENANTY_MAIL_FOLDER").unwrap_or_else(|_| "INBOX".into()),
                saved_at: Utc::now(),
            }));
        }
        return Ok(None);
    }
    let blob = std::fs::read(&path).map_err(|e| e.to_string())?;
    let acc: MailAccount = key.decrypt_json(&blob)?;
    Ok(Some(acc))
}

pub fn clear_account(data_dir: &Path) -> Result<(), String> {
    let path = credentials_path(data_dir);
    if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Default OTP patterns: prefer isolated 6-digit, then 4–8 digit codes.
pub fn extract_otp(text: &str, custom_regex: Option<&str>) -> Option<String> {
    if let Some(pat) = custom_regex {
        if let Ok(re) = Regex::new(pat) {
            if let Some(c) = re.captures(text) {
                if let Some(m) = c.get(1).or_else(|| c.get(0)) {
                    return Some(m.as_str().to_string());
                }
            }
        }
    }
    // Explicit "code is 123456" style
    let labeled = Regex::new(
        r"(?i)(?:code|otp|verification|pin|passcode)[^\d]{0,20}(\d{4,8})\b",
    )
    .ok()?;
    if let Some(c) = labeled.captures(text) {
        return c.get(1).map(|m| m.as_str().to_string());
    }
    // Standalone 6-digit (most common)
    let six = Regex::new(r"\b(\d{6})\b").ok()?;
    if let Some(c) = six.captures(text) {
        let code = c.get(1)?.as_str();
        // skip years / obvious non-otps
        if !code.starts_with("20") && code != "000000" {
            return Some(code.to_string());
        }
    }
    // 4–8 digit fallback
    let any = Regex::new(r"\b(\d{4,8})\b").ok()?;
    any.captures(text)
        .and_then(|c| c.get(1).map(|m| m.as_str().to_string()))
}

fn body_from_fetch(fetch: &Fetch) -> String {
    let mut parts = Vec::new();
    if let Some(envelope) = fetch.envelope() {
        if let Some(subj) = &envelope.subject {
            parts.push(String::from_utf8_lossy(subj).to_string());
        }
    }
    if let Some(bytes) = fetch.body() {
        if let Ok(parsed) = mailparse::parse_mail(bytes) {
            if let Some(subj) = parsed.headers.get_first_value("Subject") {
                parts.push(subj);
            }
            if let Ok(body) = parsed.get_body() {
                parts.push(body);
            }
            // multipart
            for sub in &parsed.subparts {
                if let Ok(b) = sub.get_body() {
                    parts.push(b);
                }
            }
            // headers
            if let Some(from) = parsed.headers.get_first_value("From") {
                parts.push(from);
            }
        } else {
            parts.push(String::from_utf8_lossy(bytes).to_string());
        }
    }
    if let Some(header) = fetch.header() {
        parts.push(String::from_utf8_lossy(header).to_string());
    }
    parts.join("\n")
}

fn summary_from_fetch(fetch: &Fetch, otp_re: Option<&str>) -> MailMessageSummary {
    let uid = fetch.uid.unwrap_or(0);
    let mut from = String::new();
    let mut to = String::new();
    let mut subject = String::new();
    let mut date = String::new();

    if let Some(env) = fetch.envelope() {
        if let Some(subj) = &env.subject {
            subject = String::from_utf8_lossy(subj).to_string();
        }
        if let Some(date_b) = &env.date {
            date = String::from_utf8_lossy(date_b).to_string();
        }
        if let Some(from_list) = &env.from {
            from = from_list
                .iter()
                .map(|a| {
                    let mbox = a
                        .mailbox
                        .as_ref()
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_default();
                    let host = a
                        .host
                        .as_ref()
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_default();
                    if host.is_empty() {
                        mbox
                    } else {
                        format!("{mbox}@{host}")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
        }
        if let Some(to_list) = &env.to {
            to = to_list
                .iter()
                .map(|a| {
                    let mbox = a
                        .mailbox
                        .as_ref()
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_default();
                    let host = a
                        .host
                        .as_ref()
                        .map(|b| String::from_utf8_lossy(b).to_string())
                        .unwrap_or_default();
                    if host.is_empty() {
                        mbox
                    } else {
                        format!("{mbox}@{host}")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
        }
    }

    if let Some(bytes) = fetch.body() {
        if let Ok(parsed) = mailparse::parse_mail(bytes) {
            if subject.is_empty() {
                subject = parsed.headers.get_first_value("Subject").unwrap_or_default();
            }
            if from.is_empty() {
                from = parsed.headers.get_first_value("From").unwrap_or_default();
            }
            if to.is_empty() {
                to = parsed.headers.get_first_value("To").unwrap_or_default();
            }
            if date.is_empty() {
                date = parsed.headers.get_first_value("Date").unwrap_or_default();
            }
        }
    }

    let full = body_from_fetch(fetch);
    let otp = extract_otp(&full, otp_re);
    let snippet: String = full.chars().take(240).collect::<String>().replace('\n', " ");
    MailMessageSummary {
        uid,
        from,
        to,
        subject,
        date,
        snippet,
        has_otp: otp.is_some(),
        otp,
    }
}

fn matches_filters(msg: &MailMessageSummary, full_text: &str, req: &WaitOtpRequest) -> bool {
    if let Some(f) = &req.from_contains {
        if !msg.from.to_lowercase().contains(&f.to_lowercase())
            && !full_text.to_lowercase().contains(&f.to_lowercase())
        {
            return false;
        }
    }
    if let Some(s) = &req.subject_contains {
        if !msg.subject.to_lowercase().contains(&s.to_lowercase()) {
            return false;
        }
    }
    if let Some(b) = &req.body_contains {
        if !full_text.to_lowercase().contains(&b.to_lowercase()) {
            return false;
        }
    }
    if let Some(t) = &req.to_contains {
        if !msg.to.to_lowercase().contains(&t.to_lowercase()) {
            return false;
        }
    }
    true
}

fn with_session<F, T>(account: &MailAccount, f: F) -> Result<T, String>
where
    F: FnOnce(&mut imap::Session<native_tls::TlsStream<std::net::TcpStream>>) -> Result<T, String>,
{
    let tls = TlsConnector::builder()
        .build()
        .map_err(|e| format!("tls: {e}"))?;
    let client = imap::connect((account.host.as_str(), account.port), &account.host, &tls)
        .map_err(|e| format!("imap connect {}:{} — {e}", account.host, account.port))?;
    let mut session = client
        .login(&account.username, &account.password)
        .map_err(|e| {
            format!(
                "imap login failed for {} — check app password / IMAP enabled: {}",
                account.username, e.0
            )
        })?;
    let out = f(&mut session);
    let _ = session.logout();
    out
}

/// Test login only.
pub fn test_connection(account: &MailAccount) -> Result<serde_json::Value, String> {
    with_session(account, |session| {
        let mailbox = session
            .select(&account.folder)
            .map_err(|e| format!("select {}: {e}", account.folder))?;
        Ok(serde_json::json!({
            "ok": true,
            "folder": account.folder,
            "exists": mailbox.exists,
        }))
    })
}

/// List recent messages (UID highest).
pub fn list_recent(
    account: &MailAccount,
    limit: u32,
    otp_regex: Option<&str>,
) -> Result<Vec<MailMessageSummary>, String> {
    with_session(account, |session| {
        session
            .select(&account.folder)
            .map_err(|e| format!("select: {e}"))?;
        // Search all; take last N UIDs via sequence
        let uids = session.uid_search("ALL").map_err(|e| format!("search: {e}"))?;
        let mut uid_list: Vec<u32> = uids.into_iter().collect();
        uid_list.sort_unstable();
        let take = limit.clamp(1, 50) as usize;
        let start = uid_list.len().saturating_sub(take);
        let recent = &uid_list[start..];
        if recent.is_empty() {
            return Ok(vec![]);
        }
        let set = recent
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let fetches = session
            .uid_fetch(&set, "(UID ENVELOPE BODY.PEEK[])")
            .map_err(|e| format!("fetch: {e}"))?;
        let mut out = Vec::new();
        for fetch in fetches.iter() {
            out.push(summary_from_fetch(fetch, otp_regex));
        }
        out.sort_by(|a, b| b.uid.cmp(&a.uid));
        Ok(out)
    })
}

/// Single poll for an OTP matching filters.
pub fn poll_otp(
    account: &MailAccount,
    req: &WaitOtpRequest,
) -> Result<Option<(String, MailMessageSummary)>, String> {
    with_session(account, |session| {
        session
            .select(&account.folder)
            .map_err(|e| format!("select: {e}"))?;
        // Prefer UNSEEN then fall back to recent ALL
        let mut uids: Vec<u32> = session
            .uid_search("UNSEEN")
            .map(|s| s.into_iter().collect())
            .unwrap_or_default();
        if uids.is_empty() {
            uids = session
                .uid_search("ALL")
                .map_err(|e| format!("search: {e}"))?
                .into_iter()
                .collect();
        }
        uids.sort_unstable();
        let take = req.scan_limit.clamp(1, 50) as usize;
        let start = uids.len().saturating_sub(take);
        let recent = &uids[start..];
        if recent.is_empty() {
            return Ok(None);
        }
        let set = recent
            .iter()
            .map(|u| u.to_string())
            .collect::<Vec<_>>()
            .join(",");
        let fetches = session
            .uid_fetch(&set, "(UID ENVELOPE BODY.PEEK[])")
            .map_err(|e| format!("fetch: {e}"))?;

        let mut candidates: Vec<(String, MailMessageSummary, u32)> = Vec::new();
        for fetch in fetches.iter() {
            let full = body_from_fetch(fetch);
            let msg = summary_from_fetch(fetch, req.otp_regex.as_deref());
            if !matches_filters(&msg, &full, req) {
                continue;
            }
            if let Some(otp) = msg.otp.clone().or_else(|| extract_otp(&full, req.otp_regex.as_deref()))
            {
                candidates.push((otp, msg, fetch.uid.unwrap_or(0)));
            }
        }
        // newest UID first
        candidates.sort_by(|a, b| b.2.cmp(&a.2));
        if let Some((otp, msg, _)) = candidates.into_iter().next() {
            return Ok(Some((otp, msg)));
        }
        Ok(None)
    })
}

/// Async wait loop (runs blocking IMAP on spawn_blocking).
pub async fn wait_for_otp(
    account: MailAccount,
    req: WaitOtpRequest,
) -> WaitOtpResult {
    let timeout = Duration::from_secs(req.timeout_seconds.clamp(5, 600));
    let poll = Duration::from_secs(req.poll_seconds.clamp(2, 60));
    let start = std::time::Instant::now();
    let mut polled = 0u32;

    loop {
        polled += 1;
        let acc = account.clone();
        let r = req.clone();
        let result = tokio::task::spawn_blocking(move || poll_otp(&acc, &r)).await;
        match result {
            Ok(Ok(Some((otp, msg)))) => {
                return WaitOtpResult {
                    ok: true,
                    found: true,
                    otp: Some(otp),
                    message: Some(msg),
                    polled,
                    elapsed_seconds: start.elapsed().as_secs(),
                    error: None,
                    hint: None,
                };
            }
            Ok(Ok(None)) => {}
            Ok(Err(e)) => {
                return WaitOtpResult {
                    ok: false,
                    found: false,
                    otp: None,
                    message: None,
                    polled,
                    elapsed_seconds: start.elapsed().as_secs(),
                    error: Some(e),
                    hint: Some(
                        "For Gmail: enable 2FA, create an App Password, use imap.gmail.com:993"
                            .into(),
                    ),
                };
            }
            Err(e) => {
                return WaitOtpResult {
                    ok: false,
                    found: false,
                    otp: None,
                    message: None,
                    polled,
                    elapsed_seconds: start.elapsed().as_secs(),
                    error: Some(e.to_string()),
                    hint: None,
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
                hint: Some(
                    "No OTP yet. Check spam, from_contains filter, or use a real Gmail inbox."
                        .into(),
                ),
            };
        }
        tokio::time::sleep(poll).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_six_digit() {
        let t = "Your Reddit verification code is 847293. Do not share.";
        assert_eq!(extract_otp(t, None).as_deref(), Some("847293"));
    }

    #[test]
    fn extracts_labeled() {
        let t = "OTP: 1234 for login";
        assert_eq!(extract_otp(t, None).as_deref(), Some("1234"));
    }
}
