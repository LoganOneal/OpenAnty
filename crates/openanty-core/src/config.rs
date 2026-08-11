use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bind: String,
    pub sessions_cap: u32,
    pub cdp_port_start: u16,
    pub cdp_port_end: u16,
    pub browser_path: Option<String>,
    pub max_session_ttl_seconds: u64,
    pub allow_lan: bool,
    pub experimental_js_stealth: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: std::env::var("OPENANTY_BIND").unwrap_or_else(|_| "127.0.0.1:3847".into()),
            sessions_cap: std::env::var("OPENANTY_MAX_SESSIONS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            cdp_port_start: 9222,
            cdp_port_end: 9321,
            browser_path: std::env::var("OPENANTY_BROWSER_PATH").ok().filter(|s| !s.is_empty()),
            max_session_ttl_seconds: 86400,
            allow_lan: std::env::var("OPENANTY_ALLOW_LAN").ok().as_deref() == Some("1"),
            experimental_js_stealth: std::env::var("OPENANTY_EXPERIMENTAL_JS_STEALTH")
                .ok()
                .as_deref()
                == Some("1"),
        }
    }
}

impl Config {
    pub fn load(data_dir: &Path) -> Self {
        let path = data_dir.join("config.toml");
        if path.exists() {
            if let Ok(text) = std::fs::read_to_string(&path) {
                if let Ok(cfg) = toml_ish(&text) {
                    return cfg;
                }
            }
        }
        let cfg = Self::default();
        let _ = cfg.save(data_dir);
        cfg
    }

    pub fn save(&self, data_dir: &Path) -> std::io::Result<()> {
        paths::ensure_dir(data_dir)?;
        let path = data_dir.join("config.toml");
        let text = format!(
            r#"# Open Anty daemon configuration
bind = "{}"
sessions_cap = {}
cdp_port_start = {}
cdp_port_end = {}
max_session_ttl_seconds = {}
allow_lan = {}
experimental_js_stealth = {}
{}
"#,
            self.bind,
            self.sessions_cap,
            self.cdp_port_start,
            self.cdp_port_end,
            self.max_session_ttl_seconds,
            self.allow_lan,
            self.experimental_js_stealth,
            self.browser_path
                .as_ref()
                .map(|p| format!("browser_path = \"{}\"", p.replace('\\', "\\\\")))
                .unwrap_or_default()
        );
        std::fs::write(path, text)
    }

    pub fn api_base(&self) -> String {
        format!("http://{}", self.bind)
    }
}

/// Minimal TOML subset parser for our config (avoid extra dep complexity).
fn toml_ish(text: &str) -> Result<Config, ()> {
    let mut cfg = Config::default();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        let v = v.trim().trim_matches('"');
        match k {
            "bind" => cfg.bind = v.to_string(),
            "sessions_cap" => cfg.sessions_cap = v.parse().unwrap_or(cfg.sessions_cap),
            "cdp_port_start" => cfg.cdp_port_start = v.parse().unwrap_or(cfg.cdp_port_start),
            "cdp_port_end" => cfg.cdp_port_end = v.parse().unwrap_or(cfg.cdp_port_end),
            "browser_path" => {
                if !v.is_empty() {
                    cfg.browser_path = Some(v.to_string());
                }
            }
            "max_session_ttl_seconds" => {
                cfg.max_session_ttl_seconds = v.parse().unwrap_or(cfg.max_session_ttl_seconds)
            }
            "allow_lan" => cfg.allow_lan = v == "true",
            "experimental_js_stealth" => cfg.experimental_js_stealth = v == "true",
            _ => {}
        }
    }
    Ok(cfg)
}

pub fn token_path(data_dir: &Path) -> PathBuf {
    data_dir.join("api.token")
}

pub fn read_or_create_token(data_dir: &Path) -> std::io::Result<String> {
    paths::ensure_dir(data_dir)?;
    let path = token_path(data_dir);
    if path.exists() {
        return std::fs::read_to_string(path).map(|s| s.trim().to_string());
    }
    let token = format!("oa_{}", uuid::Uuid::new_v4().simple());
    std::fs::write(&path, &token)?;
    Ok(token)
}
