use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{FingerprintSummary, ProxyConfig, FingerprintDocumentPartial, FingerprintTemplate, OsFamily};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    pub fingerprint_hash: String,
    pub fingerprint_summary: FingerprintSummary,
    pub proxy_configured: bool,
    pub cookies_pending_apply: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Full fingerprint only when include_secrets / create detail requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<crate::FingerprintDocument>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateProfileRequest {
    pub name: String,
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    #[serde(default)]
    pub proxy: Option<ProxyConfig>,
    #[serde(default)]
    pub fingerprint_overrides: Option<FingerprintDocumentPartial>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub notes: Option<String>,
}

impl CreateProfileRequest {
    pub fn template_enum(&self) -> FingerprintTemplate {
        self.template
            .as_deref()
            .and_then(FingerprintTemplate::parse)
            .unwrap_or(FingerprintTemplate::Win11ChromeMid)
    }

    pub fn os_enum(&self) -> Option<OsFamily> {
        self.os.as_deref().and_then(|s| match s.to_lowercase().as_str() {
            "windows" | "win" => Some(OsFamily::Windows),
            "macos" | "mac" | "darwin" => Some(OsFamily::Macos),
            "linux" => Some(OsFamily::Linux),
            "android" => Some(OsFamily::Android),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateProfileRequest {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub notes: Option<String>,
    #[serde(default)]
    pub fingerprint_overrides: Option<FingerprintDocumentPartial>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ApplyProxyRequest {
    pub proxy: ProxyConfig,
    #[serde(default = "default_true")]
    pub align_geo: bool,
    #[serde(default = "default_true")]
    pub check: bool,
}

fn default_true() -> bool {
    true
}
