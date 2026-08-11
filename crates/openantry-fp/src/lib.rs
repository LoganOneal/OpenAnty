//! Constraint-based fingerprint generation and validation.

mod catalogs;
mod generate;
mod hash;
mod validate;

pub use catalogs::{languages_for_country, timezone_for_country};
pub use generate::generate;
pub use hash::fingerprint_hash;
pub use validate::validate;

use openantry_proto::{
    ConsistencyReport, FingerprintDocument, FingerprintDocumentPartial, FingerprintTemplate,
    OsFamily,
};

/// Generate a coherent fingerprint, optionally applying typed overrides.
pub fn generate_with_overrides(
    template: FingerprintTemplate,
    os: Option<OsFamily>,
    binary_major: u32,
    overrides: Option<&FingerprintDocumentPartial>,
    geo_timezone: Option<&str>,
    geo_country: Option<&str>,
) -> Result<(FingerprintDocument, ConsistencyReport), String> {
    let mut doc = generate(template, os, binary_major, geo_timezone, geo_country);
    if let Some(o) = overrides {
        apply_overrides(&mut doc, o);
    }
    let report = validate(&doc);
    if !report.errors.is_empty() {
        return Err(format!(
            "fingerprint inconsistent: {}",
            report.errors.join("; ")
        ));
    }
    Ok((doc, report))
}

fn apply_overrides(doc: &mut FingerprintDocument, o: &FingerprintDocumentPartial) {
    if let Some(tz) = &o.timezone {
        doc.timezone = tz.clone();
    }
    if let Some(langs) = &o.languages {
        doc.languages = langs.clone();
    }
    if let Some(ua) = &o.user_agent {
        doc.user_agent = ua.clone();
    }
    if let Some(hc) = o.hardware_concurrency {
        doc.hardware_concurrency = hc;
    }
    if let Some(dm) = o.device_memory {
        doc.device_memory = dm;
    }
    if let Some(w) = o.screen_width {
        doc.screen.width = w;
        doc.screen.avail_width = w;
    }
    if let Some(h) = o.screen_height {
        doc.screen.height = h;
        doc.screen.avail_height = h.saturating_sub(40);
    }
    if let Some(p) = &o.webrtc_policy {
        doc.webrtc_policy = p.clone();
    }
    if let Some(c) = &o.geo_country {
        doc.geo_country = Some(c.clone());
    }
}
