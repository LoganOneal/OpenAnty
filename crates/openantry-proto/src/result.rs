use serde::Serialize;

use crate::{ErrorBody, Profile, ProxyStatus, Session};

/// Base envelope fields shared by tool/API responses.
#[derive(Debug, Clone, Serialize)]
pub struct BaseResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct OkResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl OkResult {
    pub fn success(request_id: impl Into<String>, message: Option<String>) -> Self {
        Self {
            ok: true,
            error: None,
            request_id: request_id.into(),
            message,
        }
    }

    pub fn fail(request_id: impl Into<String>, error: ErrorBody) -> Self {
        Self {
            ok: false,
            error: Some(error),
            request_id: request_id.into(),
            message: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Profile>,
}

impl ProfileResult {
    pub fn success(request_id: impl Into<String>, profile: Profile) -> Self {
        Self {
            ok: true,
            error: None,
            request_id: request_id.into(),
            profile: Some(profile),
        }
    }

    pub fn fail(request_id: impl Into<String>, error: ErrorBody) -> Self {
        Self {
            ok: false,
            error: Some(error),
            request_id: request_id.into(),
            profile: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProfileListResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    pub items: Vec<Profile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<Session>,
}

impl SessionResult {
    pub fn success(request_id: impl Into<String>, session: Session) -> Self {
        Self {
            ok: true,
            error: None,
            request_id: request_id.into(),
            session: Some(session),
        }
    }

    pub fn fail(request_id: impl Into<String>, error: ErrorBody) -> Self {
        Self {
            ok: false,
            error: Some(error),
            request_id: request_id.into(),
            session: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionListResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    pub items: Vec<Session>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProxyApplyResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub proxy_status: Option<ProxyStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint_hash: Option<String>,
    pub fingerprint_regenerated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CookieImportResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub imported: u32,
    pub skipped_expired: u32,
    pub merged: bool,
    pub applied_live: bool,
    pub cookies_pending_apply: bool,
    pub failed: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CookieExportResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    pub request_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    pub source: String,
    pub cookies: Vec<crate::Cookie>,
    pub count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemStatus {
    pub ok: bool,
    pub version: String,
    pub api_semver: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_path: Option<String>,
    pub browser_major: Option<u32>,
    pub sessions_active: u32,
    pub sessions_cap: u32,
    pub bind: String,
    pub pid: u32,
    pub data_dir: String,
    pub features: SystemFeatures,
}

#[derive(Debug, Clone, Serialize)]
pub struct SystemFeatures {
    pub patched_chromium: bool,
    pub lan_bind: bool,
    pub mcp: bool,
}
