//! Shared protocol types for OpenAnty (API, MCP, CLI).

mod cookie;
mod error;
mod fingerprint;
mod profile;
mod proxy;
mod result;
mod session;

pub use cookie::*;
pub use error::*;
pub use fingerprint::*;
pub use profile::*;
pub use proxy::*;
pub use result::*;
pub use session::*;

/// API semantic version exposed by the daemon.
pub const API_SEMVER: &str = "1.0.0";
/// Daemon package version (mirrors workspace for status endpoints).
pub const DAEMON_VERSION: &str = env!("CARGO_PKG_VERSION");
