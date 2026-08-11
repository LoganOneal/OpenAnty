use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// OS family used for coherent fingerprint generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum OsFamily {
    Windows,
    Macos,
    Linux,
    Android,
}

impl OsFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::Macos => "macos",
            Self::Linux => "linux",
            Self::Android => "android",
        }
    }

    /// True for phone/tablet-style profiles that should get CDP mobile emulation.
    pub fn is_mobile(self) -> bool {
        matches!(self, Self::Android)
    }
}

/// Named coherent templates (design F-037).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FingerprintTemplate {
    Win11ChromeMid,
    Win11ChromeHigh,
    MacosChromeMSeries,
    LinuxChromeGeneric,
    /// Android Chrome on Pixel-class handset (CDP mobile emulation).
    AndroidChromePixel,
    RandomCoherent,
}

impl FingerprintTemplate {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Win11ChromeMid => "win11_chrome_mid",
            Self::Win11ChromeHigh => "win11_chrome_high",
            Self::MacosChromeMSeries => "macos_chrome_m_series",
            Self::LinuxChromeGeneric => "linux_chrome_generic",
            Self::AndroidChromePixel => "android_chrome_pixel",
            Self::RandomCoherent => "random_coherent",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "win11_chrome_mid" => Some(Self::Win11ChromeMid),
            "win11_chrome_high" => Some(Self::Win11ChromeHigh),
            "macos_chrome_m_series" => Some(Self::MacosChromeMSeries),
            "linux_chrome_generic" => Some(Self::LinuxChromeGeneric),
            "android_chrome_pixel" | "android_pixel" | "mobile_android" => {
                Some(Self::AndroidChromePixel)
            }
            "random_coherent" => Some(Self::RandomCoherent),
            _ => None,
        }
    }

    pub fn default_os(self) -> OsFamily {
        match self {
            Self::Win11ChromeMid | Self::Win11ChromeHigh => OsFamily::Windows,
            Self::MacosChromeMSeries => OsFamily::Macos,
            Self::LinuxChromeGeneric => OsFamily::Linux,
            Self::AndroidChromePixel => OsFamily::Android,
            Self::RandomCoherent => OsFamily::Windows,
        }
    }

    pub fn is_mobile(self) -> bool {
        matches!(self, Self::AndroidChromePixel)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScreenInfo {
    pub width: u32,
    pub height: u32,
    pub avail_width: u32,
    pub avail_height: u32,
    pub color_depth: u8,
    pub device_pixel_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebGlInfo {
    pub vendor: String,
    pub renderer: String,
    #[serde(default)]
    pub unmasked_vendor: Option<String>,
    #[serde(default)]
    pub unmasked_renderer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClientHints {
    pub brands: Vec<ClientHintBrand>,
    pub mobile: bool,
    pub platform: String,
    pub platform_version: String,
    pub architecture: String,
    pub bitness: String,
    pub ua_full_version: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ClientHintBrand {
    pub brand: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct NoiseSeeds {
    /// Deterministic seed for canvas noise (stable per profile).
    pub canvas: u64,
    /// Deterministic seed for audio noise.
    pub audio: u64,
}

/// Versioned fingerprint document stored per profile.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FingerprintDocument {
    pub schema_version: u32,
    pub template: String,
    pub os: OsFamily,
    pub os_version: String,
    pub platform: String,
    pub user_agent: String,
    pub binary_major_required: u32,
    pub languages: Vec<String>,
    pub timezone: String,
    pub screen: ScreenInfo,
    pub hardware_concurrency: u32,
    pub device_memory: f64,
    pub max_touch_points: u32,
    pub webgl: WebGlInfo,
    pub fonts_set_id: String,
    pub webrtc_policy: String,
    pub do_not_track: Option<bool>,
    pub client_hints: ClientHints,
    pub noise: NoiseSeeds,
    #[serde(default)]
    pub geo_country: Option<String>,
}

impl FingerprintDocument {
    pub const SCHEMA_VERSION: u32 = 1;
}

/// Partial overrides (typed; no free-form maps).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FingerprintDocumentPartial {
    #[serde(default)]
    pub timezone: Option<String>,
    #[serde(default)]
    pub languages: Option<Vec<String>>,
    #[serde(default)]
    pub user_agent: Option<String>,
    #[serde(default)]
    pub hardware_concurrency: Option<u32>,
    #[serde(default)]
    pub device_memory: Option<f64>,
    #[serde(default)]
    pub screen_width: Option<u32>,
    #[serde(default)]
    pub screen_height: Option<u32>,
    #[serde(default)]
    pub webrtc_policy: Option<String>,
    #[serde(default)]
    pub geo_country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsistencyReport {
    pub ok: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FingerprintSummary {
    pub os: String,
    pub browser: String,
    pub template: String,
    pub fingerprint_hash: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enforcement: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    pub consistency: ConsistencyReport,
}
