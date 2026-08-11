mod api;
mod mcp;
mod ui_static;

use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use openanty_core::paths;
use openanty_core::OpenAntyService;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "openantyd", version, about = "Open Anty local antidetect daemon")]
struct Cli {
    #[arg(long, global = true, env = "OPENANTY_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run HTTP API daemon (default)
    Serve,
    /// Run MCP server over stdio (for Claude / Cursor / Grok)
    Mcp,
    /// Print system status JSON
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse()?))
        .with_target(false)
        .init();

    let cli = Cli::parse();
    let data_dir = cli.data_dir.unwrap_or_else(paths::data_dir);
    let service = Arc::new(
        OpenAntyService::open_existing(data_dir.clone()).or_else(|_| {
            tracing::info!("initializing Open Anty data dir at {}", data_dir.display());
            OpenAntyService::init(data_dir.clone()).map(|(s, recovery)| {
                if let Some(r) = recovery {
                    eprintln!("=== SAVE THIS RECOVERY KEY OFFLINE ===");
                    eprintln!("{r}");
                    eprintln!("======================================");
                }
                s
            })
        })?,
    );

    match cli.command.unwrap_or(Commands::Serve) {
        Commands::Serve => serve(service).await?,
        Commands::Mcp => mcp::run(service).await?,
        Commands::Status => {
            println!(
                "{}",
                serde_json::to_string_pretty(&service.system_status())?
            );
        }
    }
    Ok(())
}

async fn serve(service: Arc<OpenAntyService>) -> Result<()> {
    write_daemon_meta(&service)?;
    let bind = service.config.bind.clone();
    if !service.config.allow_lan
        && !bind.starts_with("127.0.0.1")
        && !bind.starts_with("localhost")
        && !bind.starts_with("[::1]")
    {
        bail!("refusing non-loopback bind without allow_lan (UNAUTHORIZED_BIND)");
    }

    let app = api::router(service.clone());
    let addr: SocketAddr = bind.parse().context("invalid bind address")?;
    tracing::info!("Open Anty API + UI listening on http://{addr}");
    tracing::info!("Control panel: http://{addr}/");
    tracing::info!("data_dir={}", service.data_dir.display());
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

fn write_daemon_meta(service: &OpenAntyService) -> Result<()> {
    let meta = serde_json::json!({
        "pid": std::process::id(),
        "api_base": service.config.api_base(),
        "bind": service.config.bind,
        "data_dir": service.data_dir,
    });
    std::fs::write(
        service.data_dir.join("daemon.json"),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    std::fs::write(
        service.data_dir.join("openantyd.pid"),
        std::process::id().to_string(),
    )?;
    Ok(())
}
