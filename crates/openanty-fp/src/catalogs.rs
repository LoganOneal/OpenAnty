use openanty_proto::{OsFamily, WebGlInfo};

pub struct OsCatalog {
    pub os_version: &'static str,
    pub platform: &'static str,
    pub fonts_set_id: &'static str,
    pub ch_platform: &'static str,
    pub ch_platform_version: &'static str,
    pub ch_architecture: &'static str,
    pub screens: &'static [(u32, u32)],
    pub webgl: &'static [WebGlEntry],
    pub locales: &'static [(&'static str, &'static [&'static str])], // tz, langs
}

pub struct WebGlEntry {
    pub vendor: &'static str,
    pub renderer: &'static str,
}

pub fn catalog(os: OsFamily) -> &'static OsCatalog {
    match os {
        OsFamily::Windows => &WINDOWS,
        OsFamily::Macos => &MACOS,
        OsFamily::Linux => &LINUX,
        OsFamily::Android => &ANDROID,
    }
}

static WINDOWS: OsCatalog = OsCatalog {
    os_version: "15.0.0",
    platform: "Win32",
    fonts_set_id: "win11_default",
    ch_platform: "Windows",
    ch_platform_version: "15.0.0",
    ch_architecture: "x86",
    screens: &[(1920, 1080), (2560, 1440), (1366, 768), (1536, 864)],
    webgl: &[
        WebGlEntry {
            vendor: "Google Inc. (NVIDIA)",
            renderer: "ANGLE (NVIDIA, NVIDIA GeForce GTX 1660 Direct3D11 vs_5_0 ps_5_0, D3D11)",
        },
        WebGlEntry {
            vendor: "Google Inc. (Intel)",
            renderer: "ANGLE (Intel, Intel(R) UHD Graphics 630 Direct3D11 vs_5_0 ps_5_0, D3D11)",
        },
        WebGlEntry {
            vendor: "Google Inc. (AMD)",
            renderer: "ANGLE (AMD, AMD Radeon RX 580 Series Direct3D11 vs_5_0 ps_5_0, D3D11)",
        },
    ],
    locales: &[
        ("America/New_York", &["en-US", "en"]),
        ("America/Los_Angeles", &["en-US", "en"]),
        ("Europe/London", &["en-GB", "en"]),
        ("Europe/Berlin", &["de-DE", "de", "en-US"]),
    ],
};

static MACOS: OsCatalog = OsCatalog {
    os_version: "14.5.0",
    platform: "MacIntel",
    fonts_set_id: "macos_sonoma",
    ch_platform: "macOS",
    ch_platform_version: "14.5.0",
    ch_architecture: "arm",
    screens: &[(1440, 900), (1680, 1050), (2560, 1600), (1512, 982)],
    webgl: &[
        WebGlEntry {
            vendor: "Google Inc. (Apple)",
            renderer: "ANGLE (Apple, ANGLE Metal Renderer: Apple M1, Unspecified Version)",
        },
        WebGlEntry {
            vendor: "Google Inc. (Apple)",
            renderer: "ANGLE (Apple, ANGLE Metal Renderer: Apple M2, Unspecified Version)",
        },
    ],
    locales: &[
        ("America/Los_Angeles", &["en-US", "en"]),
        ("Europe/Paris", &["fr-FR", "fr", "en-US"]),
        ("Asia/Tokyo", &["ja-JP", "ja", "en-US"]),
    ],
};

static LINUX: OsCatalog = OsCatalog {
    os_version: "6.5.0",
    platform: "Linux x86_64",
    fonts_set_id: "linux_noto",
    ch_platform: "Linux",
    ch_platform_version: "6.5.0",
    ch_architecture: "x86",
    screens: &[(1920, 1080), (2560, 1440), (1366, 768)],
    webgl: &[
        WebGlEntry {
            vendor: "Google Inc. (NVIDIA Corporation)",
            renderer: "ANGLE (NVIDIA Corporation, NVIDIA GeForce GTX 1050/PCIe/SSE2, OpenGL 4.5)",
        },
        WebGlEntry {
            vendor: "Google Inc. (Intel)",
            renderer: "ANGLE (Intel, Mesa Intel(R) UHD Graphics 620 (KBL GT2), OpenGL 4.6)",
        },
    ],
    locales: &[
        ("America/New_York", &["en-US", "en"]),
        ("Europe/Berlin", &["en-US", "en"]),
        ("UTC", &["en-US", "en"]),
    ],
};

/// Android Chrome (desktop Chrome + CDP mobile emulation).
static ANDROID: OsCatalog = OsCatalog {
    os_version: "14",
    platform: "Linux armv8l",
    fonts_set_id: "android_noto",
    ch_platform: "Android",
    ch_platform_version: "14.0.0",
    ch_architecture: "",
    // CSS viewport sizes (Pixel-class)
    screens: &[(390, 844), (412, 915), (360, 800), (393, 873)],
    webgl: &[
        WebGlEntry {
            vendor: "Google Inc. (Google)",
            renderer: "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)",
        },
        WebGlEntry {
            vendor: "Qualcomm",
            renderer: "Adreno (TM) 730",
        },
    ],
    locales: &[
        ("America/New_York", &["en-US", "en"]),
        ("America/Los_Angeles", &["en-US", "en"]),
        ("Europe/London", &["en-GB", "en"]),
    ],
};

pub fn webgl_info(entry: &WebGlEntry) -> WebGlInfo {
    WebGlInfo {
        vendor: entry.vendor.to_string(),
        renderer: entry.renderer.to_string(),
        unmasked_vendor: Some(entry.vendor.to_string()),
        unmasked_renderer: Some(entry.renderer.to_string()),
    }
}

/// Offline country → primary timezone map (subset for MVP).
pub fn timezone_for_country(country: &str) -> Option<&'static str> {
    match country.to_uppercase().as_str() {
        "US" => Some("America/New_York"),
        "GB" | "UK" => Some("Europe/London"),
        "DE" => Some("Europe/Berlin"),
        "FR" => Some("Europe/Paris"),
        "JP" => Some("Asia/Tokyo"),
        "AU" => Some("Australia/Sydney"),
        "CA" => Some("America/Toronto"),
        "BR" => Some("America/Sao_Paulo"),
        "IN" => Some("Asia/Kolkata"),
        "NL" => Some("Europe/Amsterdam"),
        "SG" => Some("Asia/Singapore"),
        _ => None,
    }
}

pub fn languages_for_country(country: &str) -> Option<&'static [&'static str]> {
    match country.to_uppercase().as_str() {
        "US" | "CA" | "AU" | "GB" | "UK" => Some(&["en-US", "en"]),
        "DE" | "AT" => Some(&["de-DE", "de", "en-US"]),
        "FR" => Some(&["fr-FR", "fr", "en-US"]),
        "JP" => Some(&["ja-JP", "ja", "en-US"]),
        "BR" => Some(&["pt-BR", "pt", "en-US"]),
        "IN" => Some(&["en-IN", "en", "hi-IN"]),
        "NL" => Some(&["nl-NL", "nl", "en-US"]),
        "SG" => Some(&["en-SG", "en", "zh-CN"]),
        _ => None,
    }
}
