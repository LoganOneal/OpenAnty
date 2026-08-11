//! Embedded Dolphin-style control panel (Open Anty branding).

pub const INDEX_HTML: &str = include_str!("../../../ui/index.html");
pub const APP_CSS: &str = include_str!("../../../ui/assets/app.css");
pub const APP_JS: &str = include_str!("../../../ui/assets/app.js");

pub fn index_with_token(token: &str, version: &str) -> String {
    INDEX_HTML
        .replace("{{TOKEN}}", token)
        .replace("{{VERSION}}", version)
}
