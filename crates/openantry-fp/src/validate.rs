use openantry_proto::{ConsistencyReport, FingerprintDocument, OsFamily};

use crate::catalogs::catalog;

/// Hard/soft consistency rules for fingerprint documents.
pub fn validate(doc: &FingerprintDocument) -> ConsistencyReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if doc.schema_version != FingerprintDocument::SCHEMA_VERSION {
        errors.push(format!(
            "unsupported schema_version {}",
            doc.schema_version
        ));
    }

    if doc.user_agent.is_empty() {
        errors.push("user_agent empty".into());
    }

    // OS ↔ platform / UA consistency (hard)
    match doc.os {
        OsFamily::Windows => {
            if !doc.user_agent.contains("Windows") {
                errors.push("Windows OS requires Windows token in user_agent".into());
            }
            if doc.platform != "Win32" {
                warnings.push(format!("unexpected platform {} for Windows", doc.platform));
            }
        }
        OsFamily::Macos => {
            if !doc.user_agent.contains("Macintosh") && !doc.user_agent.contains("Mac OS") {
                errors.push("macOS OS requires Macintosh token in user_agent".into());
            }
        }
        OsFamily::Linux => {
            if !doc.user_agent.contains("Linux") {
                errors.push("Linux OS requires Linux token in user_agent".into());
            }
        }
    }

    if doc.hardware_concurrency == 0 || doc.hardware_concurrency > 256 {
        errors.push("hardware_concurrency out of range".into());
    }
    if doc.device_memory <= 0.0 || doc.device_memory > 128.0 {
        errors.push("device_memory out of range".into());
    }
    if doc.screen.width < 320 || doc.screen.height < 240 {
        errors.push("screen geometry too small".into());
    }

    let cat = catalog(doc.os);
    if doc.fonts_set_id != cat.fonts_set_id
        && !doc.fonts_set_id.starts_with(match doc.os {
            OsFamily::Windows => "win",
            OsFamily::Macos => "mac",
            OsFamily::Linux => "linux",
        })
    {
        warnings.push(format!(
            "fonts_set_id {} unusual for {:?}",
            doc.fonts_set_id, doc.os
        ));
    }

    // WebGL vendor hints vs OS (soft)
    let renderer = doc.webgl.renderer.to_lowercase();
    if doc.os == OsFamily::Macos && renderer.contains("direct3d") {
        errors.push("macOS fingerprint cannot use Direct3D WebGL renderer".into());
    }
    if doc.os == OsFamily::Windows && renderer.contains("metal renderer") {
        warnings.push("Windows fingerprint with Metal WebGL is unusual".into());
    }

    if !["disable", "public_only", "proxy", "default"].contains(&doc.webrtc_policy.as_str()) {
        warnings.push(format!("unknown webrtc_policy {}", doc.webrtc_policy));
    }

    if doc.languages.is_empty() {
        errors.push("languages empty".into());
    }
    if doc.timezone.is_empty() {
        errors.push("timezone empty".into());
    }

    ConsistencyReport {
        ok: errors.is_empty(),
        warnings,
        errors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate;
    use openantry_proto::FingerprintTemplate;

    #[test]
    fn generated_docs_validate() {
        for t in [
            FingerprintTemplate::Win11ChromeMid,
            FingerprintTemplate::MacosChromeMSeries,
            FingerprintTemplate::LinuxChromeGeneric,
        ] {
            let doc = generate(t, None, 130, None, None);
            let r = validate(&doc);
            assert!(r.ok, "{:?}: {:?}", t, r.errors);
        }
    }
}
