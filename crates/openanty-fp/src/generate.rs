use openanty_proto::{
    ClientHintBrand, ClientHints, FingerprintDocument, FingerprintTemplate, NoiseSeeds, OsFamily,
    ScreenInfo,
};
use rand::{rngs::StdRng, Rng, SeedableRng};

use crate::catalogs::{self, catalog, webgl_info};

pub fn generate(
    template: FingerprintTemplate,
    os_override: Option<OsFamily>,
    binary_major: u32,
    geo_timezone: Option<&str>,
    geo_country: Option<&str>,
) -> FingerprintDocument {
    let os = os_override.unwrap_or_else(|| template.default_os());
    let mut rng = StdRng::from_entropy();
    let cat = catalog(os);

    let screen_idx = rng.gen_range(0..cat.screens.len());
    let (w, h) = cat.screens[screen_idx];
    let webgl_idx = rng.gen_range(0..cat.webgl.len());
    let locale_idx = rng.gen_range(0..cat.locales.len());
    let (default_tz, default_langs) = cat.locales[locale_idx];

    let timezone = geo_timezone.unwrap_or(default_tz).to_string();
    let languages: Vec<String> = if let Some(c) = geo_country {
        catalogs::languages_for_country(c)
            .map(|l| l.iter().map(|s| s.to_string()).collect())
            .unwrap_or_else(|| default_langs.iter().map(|s| s.to_string()).collect())
    } else {
        default_langs.iter().map(|s| s.to_string()).collect()
    };

    let major = if binary_major == 0 { 130 } else { binary_major };
    let full_version = format!("{major}.0.0.0");
    let models = ["Pixel 7", "Pixel 7a", "Pixel 8"];
    let model = models[rng.gen_range(0..models.len())].to_string();
    let ua = match os {
        OsFamily::Windows => format!(
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{full_version} Safari/537.36"
        ),
        OsFamily::Macos => format!(
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{full_version} Safari/537.36"
        ),
        OsFamily::Linux => format!(
            "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{full_version} Safari/537.36"
        ),
        OsFamily::Android => format!(
            "Mozilla/5.0 (Linux; Android 14; {model}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/{full_version} Mobile Safari/537.36"
        ),
    };

    let (hc, dm) = match template {
        FingerprintTemplate::Win11ChromeHigh => (16, 16.0),
        FingerprintTemplate::Win11ChromeMid => (8, 8.0),
        FingerprintTemplate::MacosChromeMSeries => (8, 8.0),
        FingerprintTemplate::LinuxChromeGeneric => (8, 8.0),
        FingerprintTemplate::AndroidChromePixel => (8, 8.0),
        FingerprintTemplate::RandomCoherent => {
            let options = [(4, 4.0), (8, 8.0), (12, 16.0), (16, 16.0)];
            options[rng.gen_range(0..options.len())]
        }
    };

    let mobile = os == OsFamily::Android || template.is_mobile();
    let dpr = if os == OsFamily::Macos {
        2.0
    } else if mobile {
        2.625
    } else {
        1.0
    };
    let touch = if mobile { 5 } else { 0 };
    let avail_h = if mobile {
        h
    } else {
        h.saturating_sub(40)
    };

    FingerprintDocument {
        schema_version: FingerprintDocument::SCHEMA_VERSION,
        template: template.as_str().to_string(),
        os,
        os_version: cat.os_version.to_string(),
        platform: cat.platform.to_string(),
        user_agent: ua,
        binary_major_required: major,
        languages,
        timezone,
        screen: ScreenInfo {
            width: w,
            height: h,
            avail_width: w,
            avail_height: avail_h,
            color_depth: 24,
            device_pixel_ratio: dpr,
        },
        hardware_concurrency: hc,
        device_memory: dm,
        max_touch_points: touch,
        webgl: webgl_info(&cat.webgl[webgl_idx]),
        fonts_set_id: cat.fonts_set_id.to_string(),
        webrtc_policy: "public_only".into(),
        do_not_track: None,
        client_hints: ClientHints {
            brands: vec![
                ClientHintBrand {
                    brand: "Chromium".into(),
                    version: major.to_string(),
                },
                ClientHintBrand {
                    brand: "Google Chrome".into(),
                    version: major.to_string(),
                },
                ClientHintBrand {
                    brand: "Not_A Brand".into(),
                    version: "24".into(),
                },
            ],
            mobile,
            platform: cat.ch_platform.to_string(),
            platform_version: cat.ch_platform_version.to_string(),
            architecture: cat.ch_architecture.to_string(),
            bitness: if mobile { "64".into() } else { "64".into() },
            ua_full_version: full_version,
            model: if mobile { model } else { String::new() },
        },
        noise: NoiseSeeds {
            canvas: rng.gen(),
            audio: rng.gen(),
        },
        geo_country: geo_country.map(|s| s.to_uppercase()),
    }
}
