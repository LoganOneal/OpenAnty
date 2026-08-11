use ghostfox_proto::FingerprintDocument;
use sha2::{Digest, Sha256};

/// Hash durable fingerprint fields (excludes launch-bound Client Hints brands/uaFullVersion).
pub fn fingerprint_hash(doc: &FingerprintDocument) -> String {
    let mut hasher = Sha256::new();
    // Canonical subset for stability across CH rebind at launch.
    let durable = serde_json::json!({
        "schema_version": doc.schema_version,
        "template": doc.template,
        "os": doc.os,
        "os_version": doc.os_version,
        "platform": doc.platform,
        "user_agent_major": doc.binary_major_required,
        "languages": doc.languages,
        "timezone": doc.timezone,
        "screen": doc.screen,
        "hardware_concurrency": doc.hardware_concurrency,
        "device_memory": doc.device_memory,
        "max_touch_points": doc.max_touch_points,
        "webgl": doc.webgl,
        "fonts_set_id": doc.fonts_set_id,
        "webrtc_policy": doc.webrtc_policy,
        "do_not_track": doc.do_not_track,
        "noise": doc.noise,
        "geo_country": doc.geo_country,
        "ch_platform": doc.client_hints.platform,
        "ch_mobile": doc.client_hints.mobile,
    });
    let bytes = serde_json::to_vec(&durable).expect("fingerprint serialize");
    hasher.update(bytes);
    format!("sha256:{}", hex::encode(hasher.finalize()))
}
