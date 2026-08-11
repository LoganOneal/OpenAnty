//! Proxy check and simple country/TZ alignment helpers.

use chrono::Utc;
use ghostfox_proto::{ProxyCheckResult, ProxyConfig, ProxyStatus};
use std::time::Instant;

const DEFAULT_CHECK_URLS: &[&str] = &[
    "https://api.ipify.org",
    "https://ifconfig.me/ip",
    "https://icanhazip.com",
];

pub async fn check_proxy(proxy: &ProxyConfig) -> ProxyCheckResult {
    let started = Instant::now();
    let url = proxy
        .check_url
        .as_deref()
        .unwrap_or(DEFAULT_CHECK_URLS[0]);

    let client = match build_client(proxy) {
        Ok(c) => c,
        Err(e) => {
            return ProxyCheckResult {
                ok: false,
                exit_ip: None,
                country: None,
                region: None,
                city: None,
                timezone_guess: None,
                latency_ms: None,
                checked_at: Utc::now().to_rfc3339(),
                source: "client_build".into(),
                message: Some(e),
            };
        }
    };

    match client.get(url).send().await {
        Ok(resp) => {
            let latency = started.elapsed().as_millis() as u64;
            match resp.text().await {
                Ok(body) => {
                    let ip = body.trim().to_string();
                    let ok = !ip.is_empty() && ip.len() < 64;
                    let country = None; // offline geo optional; leave null
                    let timezone_guess = None;
                    ProxyCheckResult {
                        ok,
                        exit_ip: if ok { Some(ip) } else { None },
                        country,
                        region: None,
                        city: None,
                        timezone_guess,
                        latency_ms: Some(latency),
                        checked_at: Utc::now().to_rfc3339(),
                        source: format!("ip_echo:{url}"),
                        message: if ok {
                            None
                        } else {
                            Some("empty or invalid exit IP body".into())
                        },
                    }
                }
                Err(e) => ProxyCheckResult {
                    ok: false,
                    exit_ip: None,
                    country: None,
                    region: None,
                    city: None,
                    timezone_guess: None,
                    latency_ms: Some(started.elapsed().as_millis() as u64),
                    checked_at: Utc::now().to_rfc3339(),
                    source: format!("ip_echo:{url}"),
                    message: Some(e.to_string()),
                },
            }
        }
        Err(e) => ProxyCheckResult {
            ok: false,
            exit_ip: None,
            country: None,
            region: None,
            city: None,
            timezone_guess: None,
            latency_ms: Some(started.elapsed().as_millis() as u64),
            checked_at: Utc::now().to_rfc3339(),
            source: format!("ip_echo:{url}"),
            message: Some(e.to_string()),
        },
    }
}

fn build_client(proxy: &ProxyConfig) -> Result<reqwest::Client, String> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(proxy.check_timeout_ms))
        .danger_accept_invalid_certs(false);

    let mut proxy_url = proxy.server.clone();
    // Inject userinfo if provided separately
    if let (Some(user), Some(pass)) = (&proxy.username, &proxy.password) {
        if let Some(rest) = proxy_url.split("://").nth(1) {
            let scheme = proxy_url.split("://").next().unwrap_or("http");
            proxy_url = format!("{scheme}://{}:{}@{}", urlencoding_lite(user), urlencoding_lite(pass), rest);
        }
    }

    let p = reqwest::Proxy::all(&proxy_url).map_err(|e| e.to_string())?;
    builder = builder.proxy(p);
    builder.build().map_err(|e| e.to_string())
}

fn urlencoding_lite(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

pub fn status_from_check(configured: bool, check: &ProxyCheckResult) -> ProxyStatus {
    ProxyStatus {
        configured,
        ok: check.ok,
        exit_ip: check.exit_ip.clone(),
        country: check.country.clone(),
        region: check.region.clone(),
        timezone_guess: check.timezone_guess.clone(),
        latency_ms: check.latency_ms,
        checked_at: Some(check.checked_at.clone()),
        message: check.message.clone(),
    }
}

/// Chrome `--proxy-server` argument value.
pub fn chrome_proxy_server(proxy: &ProxyConfig) -> String {
    // Chrome wants scheme://host:port without embedded credentials for some schemes;
    // credentials via extension or separate — for MVP pass full server host:port.
    let server = proxy.server.clone();
    if server.contains("://") {
        server
    } else {
        format!("http://{server}")
    }
}
