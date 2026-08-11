use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{FingerprintSummary, ProxyStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Starting,
    Running,
    Stopping,
    Stopped,
    Crashed,
    Expired,
}

impl SessionStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Starting => "starting",
            Self::Running => "running",
            Self::Stopping => "stopping",
            Self::Stopped => "stopped",
            Self::Crashed => "crashed",
            Self::Expired => "expired",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "starting" => Some(Self::Starting),
            "running" => Some(Self::Running),
            "stopping" => Some(Self::Stopping),
            "stopped" => Some(Self::Stopped),
            "crashed" => Some(Self::Crashed),
            "expired" => Some(Self::Expired),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectSnippets {
    pub javascript: String,
    pub python: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CookiesApplied {
    pub attempted: u32,
    pub applied: u32,
    pub failed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub profile_id: String,
    pub status: SessionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cdp_ws_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_port: Option<u16>,
    pub headed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
    pub connect: ConnectSnippets,
    pub proxy_status: ProxyStatus,
    pub fingerprint_summary: FingerprintSummary,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cookies_applied: Option<CookiesApplied>,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct LaunchSessionRequest {
    pub profile_id: String,
    #[serde(default = "default_headed")]
    pub headed: bool,
    #[serde(default)]
    pub start_url: Option<String>,
    #[serde(default = "default_ttl")]
    pub ttl_seconds: u64,
    #[serde(default)]
    pub force: bool,
    #[serde(default = "default_true")]
    pub locale_from_proxy: bool,
}

fn default_headed() -> bool {
    true
}

fn default_ttl() -> u64 {
    3600
}

fn default_true() -> bool {
    true
}

impl Session {
    pub fn connect_snippets(cdp: &str) -> ConnectSnippets {
        ConnectSnippets {
            javascript: format!(
                "const {{ chromium }} = require('playwright');\nconst browser = await chromium.connectOverCDP('{cdp}');"
            ),
            python: format!(
                "from playwright.async_api import async_playwright\nasync with async_playwright() as p:\n    browser = await p.chromium.connect_over_cdp('{cdp}')"
            ),
        }
    }
}
