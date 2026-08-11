use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use openanty_core::{paths, OpenAntyService};
use openanty_proto::*;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "openanty", version, about = "Open Anty CLI — agent-first antidetect browser control")]
struct Cli {
    #[arg(long, global = true, env = "OPENANTY_DATA_DIR")]
    data_dir: Option<PathBuf>,

    #[arg(long, global = true, env = "OPENANTY_API_BASE")]
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
        /// Binary name when not using --npx (default: openantyd)
        #[arg(long, default_value = "openantyd")]
        command: String,
        /// Emit npx-based config (best for agents / zero local install)
        #[arg(long)]
        npx: bool,
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
    /// Open the Dolphin-style control panel in your browser (starts API if needed)
    Ui {
        /// Do not auto-open the system browser
        #[arg(long)]
        no_open: bool,
        /// Bind address for the UI server (default from config / 127.0.0.1:3847)
        #[arg(long)]
        bind: Option<String>,
    },
    /// BYO Gmail / IMAP for OTP extraction
    Mail {
        #[command(subcommand)]
        cmd: MailCmd,
    },
}

#[derive(Subcommand, Debug)]
enum MailCmd {
    /// Show mail config status (no secrets)
    Status,
    /// Save Gmail/IMAP credentials (Gmail App Password recommended)
    Connect {
        /// Email address
        username: String,
        /// App password (or IMAP password). Prefer env OPENANTY_MAIL_PASSWORD to avoid shell history.
        #[arg(long, env = "OPENANTY_MAIL_PASSWORD")]
        password: Option<String>,
        #[arg(long, default_value = "gmail")]
        provider: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long, default_value = "INBOX")]
        folder: String,
        /// Skip IMAP login test
        #[arg(long)]
        no_test: bool,
    },
    /// Remove saved credentials
    Disconnect,
    /// List recent messages
    List {
        #[arg(long, default_value_t = 10)]
        limit: u32,
    },
    /// Poll for a verification code
    WaitOtp {
        #[arg(long)]
        from: Option<String>,
        #[arg(long)]
        subject: Option<String>,
        #[arg(long, default_value_t = 120)]
        timeout: u64,
        #[arg(long, default_value_t = 5)]
        poll: u64,
    },
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
            let (svc, recovery) = OpenAntyService::init(data_dir.clone())
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            println!("Initialized Open Anty at {}", svc.data_dir.display());
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
                println!("Open Anty doctor — ok={}", report["ok"]);
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
        Commands::McpConfig { command, npx } => {
            let data_dir = data_dir.display().to_string().replace('\\', "\\\\");
            if npx {
                println!(
                    r#"{{
  "mcpServers": {{
    "openanty": {{
      "command": "npx",
      "args": ["-y", "openanty@latest", "mcp"],
      "env": {{
        "OPENANTY_DATA_DIR": "{data_dir}"
      }}
    }}
  }}
}}"#
                );
            } else {
                println!(
                    r#"{{
  "mcpServers": {{
    "openanty": {{
      "command": "{command}",
      "args": ["mcp"],
      "env": {{
        "OPENANTY_DATA_DIR": "{data_dir}"
      }}
    }}
  }}
}}"#
                );
            }
            eprintln!();
            eprintln!("Tip: for agents with Node, prefer: openanty mcp-config --npx");
            eprintln!("Or run directly: npx -y openanty@latest mcp");
        }
        Commands::Status => {
            let svc = open_svc(data_dir)?;
            println!("{}", serde_json::to_string_pretty(&svc.system_status())?);
        }
        Commands::Ui { no_open, bind } => {
            let svc = open_svc(data_dir.clone())?;
            let url = bind
                .as_ref()
                .map(|b| format!("http://{b}/"))
                .unwrap_or_else(|| format!("{}/", svc.config.api_base()));
            println!("Open Anty control panel (Dolphin-style)");
            println!("  URL:      {url}");
            println!("  data_dir: {}", svc.data_dir.display());
            println!();
            println!("Start the API server if it is not running:");
            println!("  openantyd serve");
            println!();
            if !no_open {
                #[cfg(target_os = "windows")]
                {
                    let _ = std::process::Command::new("cmd")
                        .args(["/C", "start", "", &url])
                        .spawn();
                }
                #[cfg(target_os = "macos")]
                {
                    let _ = std::process::Command::new("open").arg(&url).spawn();
                }
                #[cfg(all(unix, not(target_os = "macos")))]
                {
                    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
                }
                println!("Opened browser. If the page fails to load, run: openantyd serve");
            }
        }
        Commands::Mail { cmd } => {
            let svc = open_svc(data_dir)?;
            match cmd {
                MailCmd::Status => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &svc.mail_status().map_err(|e| anyhow::anyhow!(e.to_string()))?
                        )?
                    );
                }
                MailCmd::Connect {
                    username,
                    password,
                    provider,
                    host,
                    port,
                    folder,
                    no_test,
                } => {
                    let password = password
                        .or_else(|| std::env::var("OPENANTY_MAIL_PASSWORD").ok())
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "password required via --password or OPENANTY_MAIL_PASSWORD"
                            )
                        })?;
                    let res = svc
                        .mail_connect(
                            &provider,
                            &username,
                            &password,
                            host.as_deref(),
                            port,
                            Some(&folder),
                            None,
                            !no_test,
                        )
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                }
                MailCmd::Disconnect => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &svc.mail_disconnect()
                                .map_err(|e| anyhow::anyhow!(e.to_string()))?
                        )?
                    );
                }
                MailCmd::List { limit } => {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(
                            &svc.mail_list(limit)
                                .map_err(|e| anyhow::anyhow!(e.to_string()))?
                        )?
                    );
                }
                MailCmd::WaitOtp {
                    from,
                    subject,
                    timeout,
                    poll,
                } => {
                    let req = openanty_core::mail::WaitOtpRequest {
                        timeout_seconds: timeout,
                        poll_seconds: poll,
                        from_contains: from,
                        subject_contains: subject,
                        body_contains: None,
                        otp_regex: None,
                        max_age_minutes: 30,
                        scan_limit: 20,
                        to_contains: None,
                    };
                    let res = svc
                        .mail_wait_otp(req)
                        .await
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    println!("{}", serde_json::to_string_pretty(&res)?);
                    if !res.found {
                        std::process::exit(2);
                    }
                }
            }
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

fn open_svc(data_dir: PathBuf) -> Result<OpenAntyService> {
    match OpenAntyService::open_existing(data_dir.clone()) {
        Ok(s) => Ok(s),
        Err(_) => {
            eprintln!("note: data dir not initialized — running init");
            let (s, r) = OpenAntyService::init(data_dir)
                .map_err(|e| anyhow::anyhow!(e.to_string()))?;
            if let Some(key) = r {
                eprintln!("recovery key (save offline): {key}");
            }
            Ok(s)
        }
    }
}
