use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Playwright-compatible cookie object.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
    pub name: String,
    pub value: String,
    pub domain: String,
    #[serde(default = "default_path")]
    pub path: String,
    #[serde(default = "default_expires")]
    pub expires: f64,
    #[serde(default)]
    pub http_only: bool,
    #[serde(default)]
    pub secure: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub same_site: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub partition_key: Option<serde_json::Value>,
}

fn default_path() -> String {
    "/".into()
}

fn default_expires() -> f64 {
    -1.0
}
