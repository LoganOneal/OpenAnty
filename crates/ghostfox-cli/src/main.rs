use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use ghostfox_core::{paths, GhostfoxService};
use ghostfox_proto::*;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "ghostfox", version, about = "GhostFox CLI — agent-first antidetect browser control")]
struct Cli {
    #[arg(long, global = true, env = "GHOSTFOX_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[arg(long, global = true, env = "GHOSTFOX_API_BASE")]
    api_base: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize data directory, encryption key, and API token
    Init,
    /// Run environment doctor checks
    Doctor {
        #[arg(long)]
        json: bool,
    },
    /// Print MCP configuration snippet for Claude / Cursor / Grok
    McpConfig {
        #[arg(long, default_value = "ghostfoxd")]
        command: String,
    },
    /// Profile operations
    Profile {
        #[command(subcommand)]
        cmd: ProfileCmd,
    },
    /// Session operations
    Session {
        #[command(subcommand)]
        cmd: SessionCmd,
    },
    /// Show system status (local service)
    Status,
}

#[derive(Subcommand, Debug)]
enum ProfileCmd {
    Create {
        name: String,
        #[arg(long)]
        template: Option<String>,
        #[arg(long)]
        os: Option<String>,
    },
    List,
    Get {
        id: String,
        #[arg(long)]
        secrets: bool,
    },
    Delete {
        id: String,
    },
    ImportCookies {
        id: String,
        #[arg(long)]
        file: PathBuf,
    },
    ExportCookies {
        id: String,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Subcommand, Debug)]
enum SessionCmd {
    Launch {
        profile_id: String,
        #[arg(long, default_value_t = true)]
        headed: bool,
        #[arg(long)]
        headless: bool,
        #[arg(long)]
        start_url: Option<String>,
        #[arg(long)]
        force: bool,
    },
    Stop {
        session_id: String,
    },
    List,
    Cdp {
        session_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let data_dir = cli.data_dir.unwrap_or_else(paths::data_dir);

    match cli.command {
        Commands::Init => {
            let (svc, recovery) = GhostfoxService::init(data_dir.clone())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("Initialized GhostFox at {}", svc.data_dir.display());
            println!("API token: {}", svc.token);
            println!("Token file: {}", svc.data_dir.join("api.token").display());
            if let Some(r) = recovery {
                println!();
                println!("=== SAVE THIS RECOVERY KEY OFFLINE, THEN DELETE recovery.key.ONCE.txt ===");
                println!("{r}");
                println!("=========================================================================");
            }
            println!();
            println!("{}", serde_json::to_string_pretty(&svc.doctor())?);
        }
        Commands::Doctor { json } => {
            let svc = open_svc(data_dir)?;
            let report = svc.doctor();
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                println!("GhostFox doctor — ok={}", report["ok"]);
                if let Some(checks) = report["checks"].as_array() {
                    for c in checks {
                        let pass = c["pass"].as_bool().unwrap_or(false);
                        let mark = if pass { "PASS" } else { "FAIL" };
                        println!(
                            "  [{mark}] {} — {}",
                            c["id"].as_str().unwrap_or("?"),
                            c["detail"].as_str().unwrap_or("")
                        );
                    }
                }
                if report["ok"] != true {
                    std::process::exit(1);
                }
            }
        }
        Commands::McpConfig { command } => {
            let data_dir = data_dir.display().to_string().replace('\\', "\\\\");
            println!(
                r#"{{
  "mcpServers": {{
    "ghostfox": {{
      "command": "{command}",
      "args": ["mcp"],
      "env": {{
        "GHOSTFOX_DATA_DIR": "{data_dir}"
      }}
    }}
  }}
}}"#
            );
        }
        Commands::Status => {
            let svc = open_svc(data_dir)?;
            println!("{}", serde_json::to_string_pretty(&svc.system_status())?);
        }
        Commands::Profile { cmd } => match cmd {
            ProfileCmd::Create { name, template, os } => {
                let svc = open_svc(data_dir)?;
                let profile = svc
                    .create_profile(CreateProfileRequest {
                        name,
                        template,
                        os,
                        proxy: None,
                        fingerprint_overrides: None,
                        tags: None,
                        notes: None,
                    })
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{}", serde_json::to_string_pretty(&profile)?);
            }
            ProfileCmd::List => {
                let svc = open_svc(data_dir)?;
                let items = svc
                    .list_profiles(100)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{}", serde_json::to_string_pretty(&items)?);
            }
            ProfileCmd::Get { id, secrets } => {
                let svc = open_svc(data_dir)?;
                let profile = svc
                    .get_profile(&id, secrets)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{}", serde_json::to_string_pretty(&profile)?);
            }
            ProfileCmd::Delete { id } => {
                let svc = open_svc(data_dir)?;
                svc.delete_profile(&id)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{{\"ok\":true,\"deleted\":\"{id}\"}}");
            }
            ProfileCmd::ImportCookies { id, file } => {
                let svc = open_svc(data_dir)?;
                let text = std::fs::read_to_string(&file)
                    .with_context(|| format!("read {}", file.display()))?;
                let cookies: Vec<Cookie> = serde_json::from_str(&text)?;
                let (imported, skipped, pending) = svc
                    .import_cookies(&id, cookies, true)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!(
                    "{}",
                    serde_json::json!({
                        "ok": true,
                        "imported": imported,
                        "skipped_expired": skipped,
                        "cookies_pending_apply": pending
                    })
                );
            }
            ProfileCmd::ExportCookies { id, out } => {
                let svc = open_svc(data_dir)?;
                let cookies = svc
                    .export_cookies(&id)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                let text = serde_json::to_string_pretty(&cookies)?;
                if let Some(path) = out {
                    std::fs::write(&path, &text)?;
                    println!("wrote {} cookies to {}", cookies.len(), path.display());
                } else {
                    println!("{text}");
                }
            }
        },
        Commands::Session { cmd } => match cmd {
            SessionCmd::Launch {
                profile_id,
                headed,
                headless,
                start_url,
                force,
            } => {
                let svc = open_svc(data_dir)?;
                let session = svc
                    .launch_session(LaunchSessionRequest {
                        profile_id,
                        headed: if headless { false } else { headed },
                        start_url,
                        ttl_seconds: 3600,
                        force,
                        locale_from_proxy: true,
                    })
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{}", serde_json::to_string_pretty(&session)?);
            }
            SessionCmd::Stop { session_id } => {
                let svc = open_svc(data_dir)?;
                svc.stop_session(&session_id)
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{{\"ok\":true,\"stopped\":\"{session_id}\"}}");
            }
            SessionCmd::List => {
                let svc = open_svc(data_dir)?;
                let items = svc
                    .list_sessions()
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                println!("{}", serde_json::to_string_pretty(&items)?);
            }
            SessionCmd::Cdp { session_id } => {
                let svc = open_svc(data_dir)?;
                let session = svc
                    .get_session(&session_id)
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                match session.cdp_ws_url {
                    Some(url) => println!("{url}"),
                    None => bail!("session has no cdp_ws_url (not running?)"),
                }
            }
        },
    }
    Ok(())
}

fn open_svc(data_dir: PathBuf) -> Result<GhostfoxService> {
    match GhostfoxService::open_existing(data_dir.clone()) {
        Ok(s) => Ok(s),
        Err(_) => {
            eprintln!("note: data dir not initialized — running init");
            let (s, r) = GhostfoxService::init(data_dir)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let Some(key) = r {
                eprintln!("recovery key (save offline): {key}");
            }
            Ok(s)
        }
    }
}
