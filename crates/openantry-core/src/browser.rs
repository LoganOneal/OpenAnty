//! Locate a Chromium/Chrome binary for session launch.

use std::path::{Path, PathBuf};

pub struct BrowserInfo {
    pub path: PathBuf,
    pub major: u32,
}

pub fn resolve_browser(config_path: Option<&str>) -> Result<BrowserInfo, String> {
    if let Some(p) = config_path {
        let path = PathBuf::from(p);
        if path.exists() {
            return Ok(BrowserInfo {
                major: detect_major(&path).unwrap_or(130),
                path,
            });
        }
        return Err(format!("OPENANTRY_BROWSER_PATH does not exist: {p}"));
    }

    for candidate in candidates() {
        if candidate.exists() {
            return Ok(BrowserInfo {
                major: detect_major(&candidate).unwrap_or(130),
                path: candidate,
            });
        }
    }

    // which chrom{e,ium}
    for name in ["chrome", "google-chrome", "chromium", "chromium-browser"] {
        if let Ok(path) = which::which(name) {
            return Ok(BrowserInfo {
                major: detect_major(&path).unwrap_or(130),
                path,
            });
        }
    }

    Err(
        "No Chrome/Chromium found. Install Chrome or set OPENANTRY_BROWSER_PATH / config browser_path."
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

fn detect_major(path: &Path) -> Option<u32> {
    // `--version` can hang if Chrome is busy; hard-timeout via thread + kill.
    let path = path.to_path_buf();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let output = std::process::Command::new(&path)
            .arg("--version")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output();
        let _ = tx.send(output.ok());
    });
    match rx.recv_timeout(std::time::Duration::from_secs(3)) {
        Ok(Some(output)) => {
            let text = String::from_utf8_lossy(&output.stdout);
            parse_major(&text).or_else(|| parse_major(&String::from_utf8_lossy(&output.stderr)))
        }
        _ => None,
    }
}

fn parse_major(text: &str) -> Option<u32> {
    // "Google Chrome 130.0.6723.117" or "Chromium 130.0...."
    for part in text.split_whitespace() {
        if let Some(maj) = part.split('.').next() {
            if let Ok(n) = maj.parse::<u32>() {
                if (50..300).contains(&n) {
                    return Some(n);
                }
            }
        }
    }
    // fallback scan digits.digits
    let re = regex_lite(text);
    re
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
        assert_eq!(
            parse_major("Google Chrome 130.0.6723.117"),
            Some(130)
        );
    }
}
