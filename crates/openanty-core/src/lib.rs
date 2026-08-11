//! OpenAnty core: paths, crypto, SQLite store, profiles, sessions, proxy, browser.

pub mod browser;
pub mod cdp_page;
pub mod config;
pub mod crypto;
pub mod features;
pub mod mail;
pub mod paths;
pub mod proxy;
pub mod service;
pub mod store;

pub use config::Config;
pub use service::OpenAntyService;
