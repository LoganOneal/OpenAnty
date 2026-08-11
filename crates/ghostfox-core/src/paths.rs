use std::path::PathBuf;

/// Resolve the GhostFox data directory (design § Data directory paths).
pub fn data_dir() -> PathBuf {
    if let Ok(p) = std::env::var("GHOSTFOX_DATA_DIR") {
        return PathBuf::from(p);
    }
    #[cfg(target_os = "windows")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("GhostFox")
    }
    #[cfg(target_os = "macos")]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("GhostFox")
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("ghostfox")
    }
}

pub fn ensure_dir(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}
