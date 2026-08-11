//! Locate a Chromium/Chrome binary for session launch.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

#[derive(Clone)]
pub struct BrowserInfo {
    pub path: PathBuf,
    pub major: u32,
}

static CACHED: OnceLock<BrowserInfo> = OnceLock::new();

/// Resolve browser once and cache (avoids repeated Chrome --version probes).
pub fn resolve_browser(config_path: Option<&str>) -> Result<BrowserInfo, String> {
    if let Some(cached) = CACHED.get() {
        // If env override matches or no override, reuse cache.
        if config_path.is_none()
            || config_path.map(PathBuf::from).as_ref() == Some(&cached.path)
        {
            return Ok(BrowserInfo {
                path: cached.path.clone(),
                major: cached.major,
            });
        }
    }

    let info = resolve_browser_uncached(config_path)?;
    let _ = CACHED.set(BrowserInfo {
        path: info.path.clone(),
        major: info.major,
    });
    Ok(info)
}

fn resolve_browser_uncached(config_path: Option<&str>) -> Result<BrowserInfo, String> {
    if let Some(p) = config_path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(BrowserInfo {
                major: detect_major(&path).unwrap_or(130),
                path,
            });
        }
        return Err(format!("OPENANTY_BROWSER_PATH does not exist: {p}"));
    }

    for candidate in candidates() {
        if candidate.exists() {
            return Ok(BrowserInfo {
                major: detect_major(&candidate).unwrap_or(130),
                path: candidate,
            });
        }
    }

    for name in ["chrome", "google-chrome", "chromium", "chromium-browser", "msedge"] {
        if let Ok(path) = which::which(name) {
            return Ok(BrowserInfo {
                major: detect_major(&path).unwrap_or(130),
                path,
            });
        }
    }

    Err(
        "No Chrome/Chromium found. Install Chrome or set OPENANTY_BROWSER_PATH / config browser_path."
            .into(),
    )
}

fn candidates() -> Vec<PathBuf> {
    let mut out = Vec::new();
    #[cfg(target_os = "windows")]
    {
        let pf = std::env::var("PROGRAMFILES").unwrap_or_else(|_| r"C:\Program Files".into());
        let pf86 =
            std::env::var("PROGRAMFILES(X86)").unwrap_or_else(|_| r"C:\Program Files (x86)".into());
        let local = std::env::var("LOCALAPPDATA").unwrap_or_default();
        out.push(PathBuf::from(&pf).join(r"Google\Chrome\Application\chrome.exe"));
        out.push(PathBuf::from(&pf86).join(r"Google\Chrome\Application\chrome.exe"));
        if !local.is_empty() {
            out.push(PathBuf::from(&local).join(r"Google\Chrome\Application\chrome.exe"));
        }
        out.push(PathBuf::from(&pf).join(r"Chromium\Application\chrome.exe"));
        out.push(PathBuf::from(&pf).join(r"Microsoft\Edge\Application\msedge.exe"));
    }
    #[cfg(target_os = "macos")]
    {
        out.push(PathBuf::from(
            "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
        ));
        out.push(PathBuf::from(
            "/Applications/Chromium.app/Contents/MacOS/Chromium",
        ));
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        out.push(PathBuf::from("/usr/bin/google-chrome"));
        out.push(PathBuf::from("/usr/bin/google-chrome-stable"));
        out.push(PathBuf::from("/usr/bin/chromium"));
        out.push(PathBuf::from("/usr/bin/chromium-browser"));
        out.push(PathBuf::from("/snap/bin/chromium"));
    }
    out
}

/// Probe Chrome major version with a hard timeout. Never blocks the daemon/CLI.
fn detect_major(path: &Path) -> Option<u32> {
    if std::env::var("OPENANTY_SKIP_BROWSER_VERSION").ok().as_deref() == Some("1") {
        return Some(130);
    }

    let mut cmd = Command::new(path);
    // Prefer a fast, non-interactive probe. Some Chrome builds hang on plain --version
    // when another instance is running; headless + no-sandbox reduces that.
    cmd.args([
        "--headless=new",
        "--disable-gpu",
        "--no-first-run",
        "--version",
    ])
    .stdin(Stdio::null())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = cmd.spawn().ok()?;
    let start = Instant::now();
    let limit = Duration::from_millis(2500);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    // Still try to read stdout for version string.
                }
                break;
            }
            Ok(None) => {
                if start.elapsed() > limit {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(_) => {
                let _ = child.kill();
                return None;
            }
        }
    }

    let output = child.wait_with_output().ok()?;
    let text = String::from_utf8_lossy(&output.stdout);
    parse_major(&text).or_else(|| parse_major(&String::from_utf8_lossy(&output.stderr)))
}

fn parse_major(text: &str) -> Option<u32> {
    for part in text.split_whitespace() {
        if let Some(maj) = part.split('.').next() {
            if let Ok(n) = maj.parse::<u32>() {
                if (50..300).contains(&n) {
                    return Some(n);
                }
            }
        }
    }
    regex_lite(text)
}

fn regex_lite(text: &str) -> Option<u32> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            if i < bytes.len() && bytes[i] == b'.' {
                let maj: u32 = text[start..i].parse().ok()?;
                if (50..300).contains(&maj) {
                    return Some(maj);
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_chrome_version() {
        assert_eq!(parse_major("Google Chrome 130.0.6723.117"), Some(130));
        assert_eq!(
            parse_major("Google Chrome 151.0.7922.77"),
            Some(151)
        );
    }
}
