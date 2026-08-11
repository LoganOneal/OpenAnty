//! High-level OpenAnty service: profiles, sessions, cookies, proxy.

use chrono::{Duration, Utc};
use openanty_fp::{fingerprint_hash, generate_with_overrides, validate};
use openanty_proto::*;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::Duration as StdDuration;
use uuid::Uuid;

use crate::browser::{self, BrowserInfo};
use crate::config::{self, Config};
use crate::crypto::MasterKey;
use crate::proxy;
use crate::store::{ProfileRow, SessionRow, Store};

pub struct OpenAntyService {
    pub store: Store,
    pub config: Config,
    pub data_dir: PathBuf,
    pub token: String,
    children: Mutex<HashMap<String, Child>>,
    browser: Mutex<Option<BrowserInfo>>,
}

impl OpenAntyService {
    pub fn init(data_dir: PathBuf) -> Result<(Self, Option<String>), OpenAntyError> {
        crate::paths::ensure_dir(&data_dir).map_err(|e| {
            OpenAntyError::app(ErrorCode::Internal, e.to_string())
        })?;
        let (key, recovery) = MasterKey::load_or_create(&data_dir).map_err(|e| {
            OpenAntyError::app(ErrorCode::Internal, e.to_string())
        })?;
        let config = Config::load(&data_dir);
        let token = config::read_or_create_token(&data_dir).map_err(|e| {
            OpenAntyError::app(ErrorCode::Internal, e.to_string())
        })?;
        let store = Store::open(&data_dir, key).map_err(|e| {
            OpenAntyError::app(ErrorCode::Internal, e.to_string())
        })?;
        let browser = browser::resolve_browser(config.browser_path.as_deref()).ok();
        Ok((
            Self {
                store,
                config,
                data_dir,
                token,
                children: Mutex::new(HashMap::new()),
                browser: Mutex::new(browser),
            },
            recovery,
        ))
    }

    pub fn open_existing(data_dir: PathBuf) -> Result<Self, OpenAntyError> {
        if !data_dir.join("openanty.db").exists()
            && !data_dir.join("OpenAnty.db").exists() // legacy misnamed file
            && !data_dir.join("api.token").exists()
        {
            return Err(OpenAntyError::app(
                ErrorCode::NotInitialized,
                "Open Anty not initialized — run `openanty init` first",
            )
            .with_hint("openanty init"));
        }
        Self::init(data_dir).map(|(s, _)| s)
    }

    pub fn request_id() -> String {
        format!("req_{}", Uuid::new_v4().simple())
    }

    fn browser_info(&self) -> Result<BrowserInfo, OpenAntyError> {
        let mut guard = self.browser.lock();
        if let Some(b) = guard.as_ref() {
            return Ok(BrowserInfo {
                path: b.path.clone(),
                major: b.major,
            });
        }
        let info = browser::resolve_browser(self.config.browser_path.as_deref()).map_err(|e| {
            OpenAntyError::app(ErrorCode::BinaryMissing, e)
                .with_hint("Install Chrome or set browser_path in config / OPENANTY_BROWSER_PATH")
        })?;
        *guard = Some(BrowserInfo {
            path: info.path.clone(),
            major: info.major,
        });
        Ok(info)
    }

    fn to_profile(&self, row: &ProfileRow, include_secrets: bool) -> Profile {
        let report = validate(&row.fingerprint);
        Profile {
            id: row.id.clone(),
            name: row.name.clone(),
            tags: serde_json::from_str(&row.tags_json).unwrap_or_default(),
            notes: row.notes.clone(),
            fingerprint_hash: row.fingerprint_hash.clone(),
            fingerprint_summary: FingerprintSummary {
                os: format!("{:?} {}", row.fingerprint.os, row.fingerprint.os_version),
                browser: format!("Chrome/{}", row.fingerprint.binary_major_required),
                template: row.fingerprint.template.clone(),
                fingerprint_hash: row.fingerprint_hash.clone(),
                enforcement: None,
                warnings: report.warnings.clone(),
                consistency: report,
            },
            proxy_configured: row.proxy.is_some(),
            cookies_pending_apply: row.cookies_pending_apply,
            created_at: row.created_at,
            updated_at: row.updated_at,
            fingerprint: if include_secrets {
                Some(row.fingerprint.clone())
            } else {
                None
            },
        }
    }

    pub fn create_profile(&self, req: CreateProfileRequest) -> Result<Profile, OpenAntyError> {
        let browser = self.browser_info().unwrap_or(BrowserInfo {
            path: PathBuf::from("chrome"),
            major: 130,
        });
        let template = req.template_enum();
        let (doc, _report) = generate_with_overrides(
            template,
            req.os_enum(),
            browser.major,
            req.fingerprint_overrides.as_ref(),
            None,
            None,
        )
        .map_err(|e| OpenAntyError::app(ErrorCode::FingerprintInconsistent, e))?;

        let id = format!("prf_{}", Uuid::new_v4().simple());
        let now = Utc::now();
        let data_path = self.data_dir.join("profiles").join(&id);
        crate::paths::ensure_dir(&data_path).map_err(|e| {
            OpenAntyError::app(ErrorCode::Internal, e.to_string())
        })?;

        let row = ProfileRow {
            id: id.clone(),
            name: req.name,
            tags_json: serde_json::to_string(&req.tags.unwrap_or_default()).unwrap_or_else(|_| "[]".into()),
            notes: req.notes,
            fingerprint_hash: fingerprint_hash(&doc),
            fingerprint: doc,
            proxy: req.proxy,
            cookies: vec![],
            cookies_pending_apply: false,
            data_path,
            lock_session_id: None,
            created_at: now,
            updated_at: now,
        };
        self.store
            .insert_profile(&row)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        self.store.audit("profile.create", "profile", &id, "{}");
        Ok(self.to_profile(&row, false))
    }

    pub fn list_profiles(&self, limit: u32) -> Result<Vec<Profile>, OpenAntyError> {
        let rows = self
            .store
            .list_profiles(limit.clamp(1, 200))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        Ok(rows.iter().map(|r| self.to_profile(r, false)).collect())
    }

    pub fn get_profile(&self, id: &str, include_secrets: bool) -> Result<Profile, OpenAntyError> {
        let row = self
            .store
            .get_profile(id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::ProfileNotFound, "profile not found"))?;
        Ok(self.to_profile(&row, include_secrets))
    }

    pub fn delete_profile(&self, id: &str) -> Result<(), OpenAntyError> {
        if let Some(row) = self
            .store
            .get_profile(id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
        {
            if row.lock_session_id.is_some() {
                return Err(OpenAntyError::app(
                    ErrorCode::SessionAlreadyRunning,
                    "stop the session before deleting the profile",
                ));
            }
        }
        let ok = self
            .store
            .soft_delete_profile(id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        if !ok {
            return Err(OpenAntyError::app(
                ErrorCode::ProfileNotFound,
                "profile not found",
            ));
        }
        self.store.audit("profile.delete", "profile", id, "{}");
        Ok(())
    }

    pub async fn apply_proxy(
        &self,
        profile_id: &str,
        req: ApplyProxyRequest,
    ) -> Result<(Profile, ProxyStatus, bool), OpenAntyError> {
        let mut row = self
            .store
            .get_profile(profile_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::ProfileNotFound, "profile not found"))?;

        let mut status = ProxyStatus::none();
        status.configured = true;
        let mut regenerated = false;

        if req.check {
            let check = proxy::check_proxy(&req.proxy).await;
            status = proxy::status_from_check(true, &check);
            if !check.ok {
                return Err(OpenAntyError::app(
                    ErrorCode::ProxyDead,
                    check
                        .message
                        .unwrap_or_else(|| "proxy check failed".into()),
                )
                .with_hint("Verify proxy URL, credentials, and network access"));
            }
            if req.align_geo {
                // Prefer explicit country from check; offline map is best-effort.
                if let Some(country) = check.country.as_deref().or(status.country.as_deref()) {
                    let tz = openanty_fp::timezone_for_country(country);
                    let browser = self.browser_info().unwrap_or(BrowserInfo {
                        path: PathBuf::from("chrome"),
                        major: row.fingerprint.binary_major_required,
                    });
                    let template = FingerprintTemplate::parse(&row.fingerprint.template)
                        .unwrap_or(FingerprintTemplate::Win11ChromeMid);
                    let (doc, _) = generate_with_overrides(
                        template,
                        Some(row.fingerprint.os),
                        browser.major,
                        None,
                        tz,
                        Some(country),
                    )
                    .map_err(|e| OpenAntyError::app(ErrorCode::FingerprintInconsistent, e))?;
                    row.fingerprint = doc;
                    row.fingerprint_hash = fingerprint_hash(&row.fingerprint);
                    regenerated = true;
                }
            }
        }

        row.proxy = Some(req.proxy);
        row.updated_at = Utc::now();
        self.store
            .update_profile(&row)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        self.store
            .audit("profile.apply_proxy", "profile", profile_id, "{}");
        Ok((self.to_profile(&row, false), status, regenerated))
    }

    pub fn import_cookies(
        &self,
        profile_id: &str,
        cookies: Vec<Cookie>,
        merge: bool,
    ) -> Result<(u32, u32, bool), OpenAntyError> {
        let mut row = self
            .store
            .get_profile(profile_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::ProfileNotFound, "profile not found"))?;

        let now = Utc::now().timestamp() as f64;
        let mut skipped = 0u32;
        let mut valid = Vec::new();
        for c in cookies {
            if c.expires > 0.0 && c.expires < now {
                skipped += 1;
                continue;
            }
            if c.name.is_empty() || c.domain.is_empty() {
                skipped += 1;
                continue;
            }
            valid.push(c);
        }
        let imported = valid.len() as u32;
        if merge {
            let mut map: HashMap<(String, String, String), Cookie> = HashMap::new();
            for c in row.cookies.drain(..) {
                map.insert(
                    (c.name.clone(), c.domain.clone(), c.path.clone()),
                    c,
                );
            }
            for c in valid {
                map.insert(
                    (c.name.clone(), c.domain.clone(), c.path.clone()),
                    c,
                );
            }
            row.cookies = map.into_values().collect();
        } else {
            row.cookies = valid;
        }
        row.cookies_pending_apply = true;
        row.updated_at = Utc::now();
        self.store
            .update_profile(&row)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        Ok((imported, skipped, true))
    }

    pub fn export_cookies(&self, profile_id: &str) -> Result<Vec<Cookie>, OpenAntyError> {
        let row = self
            .store
            .get_profile(profile_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::ProfileNotFound, "profile not found"))?;
        Ok(row.cookies)
    }

    fn alloc_port(&self) -> Result<u16, OpenAntyError> {
        for _ in 0..40 {
            let port = fastrand_port(self.config.cdp_port_start, self.config.cdp_port_end);
            if TcpListener::bind(("127.0.0.1", port)).is_ok() {
                return Ok(port);
            }
        }
        Err(OpenAntyError::app(
            ErrorCode::PortConflict,
            "no free CDP ports in configured range",
        ))
    }

    pub async fn launch_session(&self, req: LaunchSessionRequest) -> Result<Session, OpenAntyError> {
        let mut profile = self
            .store
            .get_profile(&req.profile_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::ProfileNotFound, "profile not found"))?;

        if let Some(existing) = &profile.lock_session_id {
            let mut should_takeover = req.force;
            if !should_takeover {
                if let Ok(Some(ses)) = self.store.get_session(existing) {
                    if matches!(ses.status, SessionStatus::Running | SessionStatus::Starting) {
                        // If CDP is dead, treat as crashed and reclaim automatically.
                        let alive = if let Some(port) = ses.debug_port {
                            cdp_port_alive(port).await
                        } else {
                            false
                        };
                        if alive {
                            return Err(OpenAntyError::app(
                                ErrorCode::SessionAlreadyRunning,
                                format!("session {existing} already running"),
                            )
                            .with_hint("pass force=true to takeover or stop_session first"));
                        }
                        should_takeover = true;
                    }
                }
            }
            if should_takeover {
                let _ = self.stop_session(existing).await;
                profile = self
                    .store
                    .get_profile(&req.profile_id)
                    .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
                    .ok_or_else(|| {
                        OpenAntyError::app(ErrorCode::ProfileNotFound, "profile not found")
                    })?;
            }
        }

        let active = self
            .store
            .list_sessions(true)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        if active.len() as u32 >= self.config.sessions_cap {
            return Err(OpenAntyError::app(
                ErrorCode::ResourceLimit,
                format!("max concurrent sessions ({})", self.config.sessions_cap),
            ));
        }

        let browser = self.browser_info()?;
        let port = self.alloc_port()?;
        let session_id = format!("ses_{}", Uuid::new_v4().simple());
        let user_data = &profile.data_path;
        crate::paths::ensure_dir(user_data).map_err(|e| {
            OpenAntyError::app(ErrorCode::Internal, e.to_string())
        })?;

        // Clear stale Chromium locks so relaunch after kill is reliable.
        for name in ["SingletonLock", "SingletonCookie", "SingletonSocket", "lockfile"] {
            let p = user_data.join(name);
            let _ = std::fs::remove_file(p);
        }

        let mut args = vec![
            format!("--remote-debugging-port={port}"),
            format!("--remote-debugging-address=127.0.0.1"),
            "--remote-allow-origins=*".into(),
            format!("--user-data-dir={}", user_data.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-background-networking".into(),
            "--disable-sync".into(),
            "--disable-features=Translate,MediaRouter".into(),
            "--disable-dev-shm-usage".into(),
            "--no-sandbox".into(),
        ];
        if !req.headed {
            args.push("--headless=new".into());
            args.push("--disable-gpu".into());
        }
        // Phase B: load enabled unpacked extensions
        if let Ok(exts) = self.list_extensions() {
            let paths: Vec<String> = exts
                .into_iter()
                .filter(|e| e.enabled)
                .map(|e| e.path)
                .filter(|p| std::path::Path::new(p).exists())
                .collect();
            if !paths.is_empty() {
                args.push(format!("--load-extension={}", paths.join(",")));
                args.push(format!(
                    "--disable-extensions-except={}",
                    paths.join(",")
                ));
            }
        }
        if let Some(proxy) = &profile.proxy {
            args.push(format!(
                "--proxy-server={}",
                proxy::chrome_proxy_server(proxy)
            ));
        }
        // Lang / timezone best-effort via prefs-like flags
        args.push(format!("--lang={}", profile.fingerprint.languages.first().cloned().unwrap_or_else(|| "en-US".into())));
        if let Some(url) = &req.start_url {
            args.push(url.clone());
        } else {
            args.push("about:blank".into());
        }

        #[cfg(target_os = "windows")]
        let (child_opt, pid) = {
            // Use `cmd /c start` so Chrome is not a child of the CLI/shell job object.
            // Otherwise PowerShell/MCP hosts block until the browser exits.
            let mut full_args: Vec<String> = vec![
                "/C".into(),
                "start".into(),
                "".into(), // window title
                "/B".into(),
                browser.path.display().to_string(),
            ];
            full_args.extend(args.clone());
            let mut cmd = Command::new("cmd.exe");
            cmd.args(&full_args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW for the cmd wrapper
            let status = cmd.status().map_err(|e| {
                OpenAntyError::app(
                    ErrorCode::BinaryMissing,
                    format!("failed to spawn browser via cmd: {e}"),
                )
            })?;
            if !status.success() {
                return Err(OpenAntyError::app(
                    ErrorCode::BinaryMissing,
                    format!("cmd start failed with {status}"),
                ));
            }
            // PID unknown when using start — discover via CDP later; store 0 placeholder.
            (None, 0u32)
        };

        #[cfg(not(target_os = "windows"))]
        let (child_opt, pid) = {
            let mut cmd = Command::new(&browser.path);
            cmd.args(&args)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            let child = cmd.spawn().map_err(|e| {
                OpenAntyError::app(
                    ErrorCode::BinaryMissing,
                    format!("failed to spawn browser: {e}"),
                )
            })?;
            let pid = child.id();
            (Some(child), pid)
        };

        let cdp_ws_url = wait_for_cdp(port, 30).await.map_err(|e| {
            let _ = kill_process(pid);
            OpenAntyError::app(ErrorCode::Internal, e)
        })?;

        // Apply cookies via CDP before returning URL
        let mut cookies_applied = CookiesApplied {
            attempted: 0,
            applied: 0,
            failed: 0,
        };
        if !profile.cookies.is_empty() {
            cookies_applied.attempted = profile.cookies.len() as u32;
            match apply_cookies_cdp(&cdp_ws_url, &profile.cookies).await {
                Ok(n) => {
                    cookies_applied.applied = n;
                    cookies_applied.failed = cookies_applied.attempted.saturating_sub(n);
                    profile.cookies_pending_apply = cookies_applied.failed > 0;
                }
                Err(_) => {
                    cookies_applied.failed = cookies_applied.attempted;
                }
            }
        }

        if self.config.experimental_js_stealth {
            let _ = inject_stealth_init(&cdp_ws_url).await;
        }

        let now = Utc::now();
        let ttl = req.ttl_seconds.clamp(60, self.config.max_session_ttl_seconds);
        let expires = now + Duration::seconds(ttl as i64);

        let ses = SessionRow {
            id: session_id.clone(),
            profile_id: profile.id.clone(),
            pid: if pid == 0 { None } else { Some(pid) },
            debug_port: Some(port),
            cdp_ws_url: Some(cdp_ws_url.clone()),
            status: SessionStatus::Running,
            headed: req.headed,
            started_at: now,
            expires_at: Some(expires),
            last_heartbeat_at: Some(now),
        };
        self.store
            .insert_session(&ses)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;

        profile.lock_session_id = Some(session_id.clone());
        profile.updated_at = now;
        self.store
            .update_profile(&profile)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;

        if let Some(child) = child_opt {
            self.children.lock().insert(session_id.clone(), child);
        }

        let report = validate(&profile.fingerprint);
        let mut warnings = report.warnings.clone();
        warnings.push("document_only:canvas".into());
        warnings.push("document_only:webgl".into());
        warnings.push("enforcement:stock".into());

        let proxy_status = if let Some(p) = &profile.proxy {
            ProxyStatus {
                configured: true,
                ok: true,
                exit_ip: None,
                country: None,
                region: None,
                timezone_guess: None,
                latency_ms: None,
                checked_at: None,
                message: Some(format!("proxy {}", p.server)),
            }
        } else {
            ProxyStatus::none()
        };

        Ok(Session {
            id: session_id,
            profile_id: profile.id,
            status: SessionStatus::Running,
            cdp_ws_url: Some(cdp_ws_url.clone()),
            debug_port: Some(port),
            headed: req.headed,
            expires_at: Some(expires),
            connect: Session::connect_snippets(&cdp_ws_url),
            proxy_status,
            fingerprint_summary: FingerprintSummary {
                os: format!("{:?} {}", profile.fingerprint.os, profile.fingerprint.os_version),
                browser: format!("Chrome/{}", browser.major),
                template: profile.fingerprint.template,
                fingerprint_hash: profile.fingerprint_hash,
                enforcement: Some("stock".into()),
                warnings,
                consistency: report,
            },
            cookies_applied: Some(cookies_applied),
            started_at: now,
        })
    }

    pub async fn stop_session(&self, session_id: &str) -> Result<(), OpenAntyError> {
        let mut ses = self
            .store
            .get_session(session_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::SessionNotFound, "session not found"))?;

        ses.status = SessionStatus::Stopping;
        let _ = self.store.update_session(&ses);

        // Harvest cookies then close browser via CDP (works even without stored PID).
        if let Some(url) = &ses.cdp_ws_url {
            if let Ok(Ok(cookies)) = tokio::time::timeout(
                StdDuration::from_secs(2),
                harvest_cookies_cdp(url),
            )
            .await
            {
                if let Ok(Some(mut profile)) = self.store.get_profile(&ses.profile_id) {
                    profile.cookies = cookies;
                    profile.cookies_pending_apply = false;
                    profile.updated_at = Utc::now();
                    let _ = self.store.update_profile(&profile);
                }
            }
            let _ = tokio::time::timeout(StdDuration::from_secs(2), cdp_browser_close(url)).await;
        }

        if let Some(mut child) = self.children.lock().remove(session_id) {
            let _ = child.kill();
            let _ = std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        if let Some(pid) = ses.pid {
            kill_process(pid);
        }
        if let Some(port) = ses.debug_port {
            kill_by_debug_port(port);
        }
        // Brief pause so user-data-dir lock is released on Windows.
        tokio::time::sleep(StdDuration::from_millis(400)).await;

        ses.status = SessionStatus::Stopped;
        ses.cdp_ws_url = None;
        let _ = self.store.update_session(&ses);

        if let Ok(Some(mut profile)) = self.store.get_profile(&ses.profile_id) {
            if profile.lock_session_id.as_deref() == Some(session_id) {
                profile.lock_session_id = None;
                profile.updated_at = Utc::now();
                let _ = self.store.update_profile(&profile);
            }
        }
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Result<Session, OpenAntyError> {
        let ses = self
            .store
            .get_session(id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::SessionNotFound, "session not found"))?;
        self.session_to_api(&ses)
    }

    pub fn list_sessions(&self) -> Result<Vec<Session>, OpenAntyError> {
        let rows = self
            .store
            .list_sessions(false)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        rows.iter().map(|s| self.session_to_api(s)).collect()
    }

    pub fn heartbeat(&self, session_id: &str) -> Result<Session, OpenAntyError> {
        let mut ses = self
            .store
            .get_session(session_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::SessionNotFound, "session not found"))?;
        if !matches!(ses.status, SessionStatus::Running) {
            return Err(OpenAntyError::app(
                ErrorCode::SessionExpired,
                "session is not running",
            ));
        }
        let now = Utc::now();
        ses.last_heartbeat_at = Some(now);
        ses.expires_at = Some(now + Duration::seconds(3600));
        self.store
            .update_session(&ses)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        self.session_to_api(&ses)
    }

    fn session_to_api(&self, ses: &SessionRow) -> Result<Session, OpenAntyError> {
        let profile = self
            .store
            .get_profile(&ses.profile_id)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?
            .ok_or_else(|| OpenAntyError::app(ErrorCode::ProfileNotFound, "profile missing"))?;
        let report = validate(&profile.fingerprint);
        let cdp = ses.cdp_ws_url.clone().unwrap_or_default();
        Ok(Session {
            id: ses.id.clone(),
            profile_id: ses.profile_id.clone(),
            status: ses.status,
            cdp_ws_url: ses.cdp_ws_url.clone(),
            debug_port: ses.debug_port,
            headed: ses.headed,
            expires_at: ses.expires_at,
            connect: Session::connect_snippets(&cdp),
            proxy_status: if profile.proxy.is_some() {
                let mut s = ProxyStatus::none();
                s.configured = true;
                s.ok = true;
                s
            } else {
                ProxyStatus::none()
            },
            fingerprint_summary: FingerprintSummary {
                os: format!("{:?} {}", profile.fingerprint.os, profile.fingerprint.os_version),
                browser: format!("Chrome/{}", profile.fingerprint.binary_major_required),
                template: profile.fingerprint.template,
                fingerprint_hash: profile.fingerprint_hash,
                enforcement: Some("stock".into()),
                warnings: report.warnings.clone(),
                consistency: report,
            },
            cookies_applied: None,
            started_at: ses.started_at,
        })
    }

    pub fn system_status(&self) -> SystemStatus {
        let browser = self.browser_info().ok();
        let active = self.store.list_sessions(true).map(|v| v.len()).unwrap_or(0) as u32;
        SystemStatus {
            ok: true,
            version: DAEMON_VERSION.to_string(),
            api_semver: API_SEMVER.to_string(),
            browser_path: browser.as_ref().map(|b| b.path.display().to_string()),
            browser_major: browser.as_ref().map(|b| b.major),
            sessions_active: active,
            sessions_cap: self.config.sessions_cap,
            bind: self.config.bind.clone(),
            pid: std::process::id(),
            data_dir: self.data_dir.display().to_string(),
            features: SystemFeatures {
                patched_chromium: false,
                lan_bind: self.config.allow_lan,
                mcp: true,
            },
        }
    }

    pub fn doctor(&self) -> serde_json::Value {
        let browser = self.browser_info();
        let mut checks = vec![];
        checks.push(json_check(
            "data_dir",
            self.data_dir.exists(),
            self.data_dir.display().to_string(),
        ));
        checks.push(json_check(
            "api_token",
            !self.token.is_empty(),
            "present".to_string(),
        ));
        match &browser {
            Ok(b) => checks.push(json_check(
                "browser",
                true,
                format!("{} (major {})", b.path.display(), b.major),
            )),
            Err(e) => checks.push(json_check("browser", false, e.to_string())),
        }
        checks.push(json_check(
            "bind_localhost",
            self.config.bind.starts_with("127.0.0.1")
                || self.config.bind.starts_with("localhost"),
            self.config.bind.clone(),
        ));
        let ok = checks.iter().all(|c| c["pass"].as_bool() == Some(true));
        serde_json::json!({
            "ok": ok,
            "version": DAEMON_VERSION,
            "checks": checks,
        })
    }

    // —— Phase B/C/D feature APIs ——

    pub fn bulk_create_profiles(
        &self,
        count: u32,
        name_prefix: &str,
        tags: Option<Vec<String>>,
    ) -> Result<Vec<Profile>, OpenAntyError> {
        let count = count.clamp(1, 50);
        let mut out = Vec::new();
        for i in 1..=count {
            let p = self.create_profile(CreateProfileRequest {
                name: format!("{name_prefix}-{i:02}"),
                template: None,
                os: None,
                proxy: None,
                fingerprint_overrides: None,
                tags: tags.clone(),
                notes: Some("bulk created".into()),
            })?;
            out.push(p);
        }
        Ok(out)
    }

    pub fn list_proxy_pool(&self) -> Result<Vec<crate::features::ProxyPoolItem>, OpenAntyError> {
        self.store
            .with_conn(crate::features::list_proxy_pool)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn add_proxy_pool(
        &self,
        name: &str,
        server: &str,
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<crate::features::ProxyPoolItem, OpenAntyError> {
        self.store
            .with_conn(|c| crate::features::add_proxy_pool(c, name, server, username, password))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn delete_proxy_pool(&self, id: &str) -> Result<(), OpenAntyError> {
        let ok = self
            .store
            .with_conn(|c| crate::features::delete_proxy_pool(c, id))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        if !ok {
            return Err(OpenAntyError::app(ErrorCode::InvalidRequest, "proxy not found"));
        }
        Ok(())
    }

    pub fn list_extensions(&self) -> Result<Vec<crate::features::ExtensionItem>, OpenAntyError> {
        self.store
            .with_conn(crate::features::list_extensions)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn add_extension(
        &self,
        name: &str,
        path: &str,
        enabled: bool,
    ) -> Result<crate::features::ExtensionItem, OpenAntyError> {
        self.store
            .with_conn(|c| crate::features::add_extension(c, name, path, enabled))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn delete_extension(&self, id: &str) -> Result<(), OpenAntyError> {
        let ok = self
            .store
            .with_conn(|c| crate::features::delete_extension(c, id))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        if !ok {
            return Err(OpenAntyError::app(ErrorCode::InvalidRequest, "extension not found"));
        }
        Ok(())
    }

    pub fn list_scenarios(&self) -> Result<Vec<crate::features::ScenarioItem>, OpenAntyError> {
        self.store
            .with_conn(crate::features::list_scenarios)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn save_scenario(
        &self,
        name: &str,
        steps: Vec<serde_json::Value>,
    ) -> Result<crate::features::ScenarioItem, OpenAntyError> {
        self.store
            .with_conn(|c| crate::features::save_scenario(c, name, &steps))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn delete_scenario(&self, id: &str) -> Result<(), OpenAntyError> {
        let ok = self
            .store
            .with_conn(|c| crate::features::delete_scenario(c, id))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        if !ok {
            return Err(OpenAntyError::app(ErrorCode::InvalidRequest, "scenario not found"));
        }
        Ok(())
    }

    pub fn list_users(&self) -> Result<Vec<crate::features::UserItem>, OpenAntyError> {
        self.store
            .with_conn(crate::features::list_users)
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn add_user(&self, username: &str, role: &str) -> Result<crate::features::UserItem, OpenAntyError> {
        let role = match role {
            "admin" | "operator" | "viewer" => role,
            _ => "viewer",
        };
        self.store
            .with_conn(|c| crate::features::add_user(c, username, role))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))
    }

    pub fn delete_user(&self, id: &str) -> Result<(), OpenAntyError> {
        let ok = self
            .store
            .with_conn(|c| crate::features::delete_user(c, id))
            .map_err(|e| OpenAntyError::app(ErrorCode::Internal, e))?;
        if !ok {
            return Err(OpenAntyError::app(ErrorCode::InvalidRequest, "user not found"));
        }
        Ok(())
    }

    /// Offline fingerprint consistency health (Phase B harness baseline).
    pub fn fingerprint_health_sample(&self) -> serde_json::Value {
        let browser = self.browser_info().unwrap_or(BrowserInfo {
            path: PathBuf::from("chrome"),
            major: 130,
        });
        let mut reports = vec![];
        for template in [
            FingerprintTemplate::Win11ChromeMid,
            FingerprintTemplate::Win11ChromeHigh,
            FingerprintTemplate::MacosChromeMSeries,
        ] {
            match generate_with_overrides(template, None, browser.major, None, None, None) {
                Ok((doc, report)) => {
                    reports.push(serde_json::json!({
                        "template": format!("{:?}", template),
                        "fingerprint_hash": fingerprint_hash(&doc),
                        "valid": report.ok,
                        "warnings": report.warnings,
                        "errors": report.errors,
                    }));
                }
                Err(e) => reports.push(serde_json::json!({
                    "template": format!("{:?}", template),
                    "error": e
                })),
            }
        }
        let all_valid = reports.iter().all(|r| r.get("valid") == Some(&serde_json::json!(true)));
        serde_json::json!({
            "ok": all_valid,
            "engine": "stock_chrome",
            "patched_chromium": false,
            "note": "Phase 0/B: full detector suites require patched Chromium distribution",
            "samples": reports,
        })
    }

    pub async fn run_scenario(
        &self,
        profile_id: &str,
        steps: Vec<serde_json::Value>,
        headed: bool,
    ) -> Result<serde_json::Value, OpenAntyError> {
        let session = self
            .launch_session(LaunchSessionRequest {
                profile_id: profile_id.into(),
                headed,
                start_url: Some("about:blank".into()),
                ttl_seconds: 3600,
                force: false,
                locale_from_proxy: true,
            })
            .await?;
        let cdp = session
            .cdp_ws_url
            .clone()
            .ok_or_else(|| OpenAntyError::app(ErrorCode::Internal, "no cdp url"))?;
        let mut results = Vec::new();
        for (i, step) in steps.iter().enumerate() {
            let action = step.get("action").and_then(|a| a.as_str()).unwrap_or("");
            let step_result = match action {
                "navigate" => {
                    let url = step.get("url").and_then(|u| u.as_str()).unwrap_or("about:blank");
                    match crate::cdp_page::page_navigate(&cdp, url).await {
                        Ok(p) => serde_json::json!({"step": i, "action": action, "ok": true, "url": p.url, "title": p.title}),
                        Err(e) => serde_json::json!({"step": i, "action": action, "ok": false, "error": e}),
                    }
                }
                "wait" => {
                    let ms = step.get("ms").and_then(|m| m.as_u64()).unwrap_or(1000);
                    tokio::time::sleep(StdDuration::from_millis(ms)).await;
                    serde_json::json!({"step": i, "action": action, "ok": true, "ms": ms})
                }
                "click" => {
                    let sel = step.get("selector").and_then(|s| s.as_str()).unwrap_or("");
                    match crate::cdp_page::page_click(&cdp, sel).await {
                        Ok(()) => serde_json::json!({"step": i, "action": action, "ok": true}),
                        Err(e) => serde_json::json!({"step": i, "action": action, "ok": false, "error": e}),
                    }
                }
                "type" => {
                    let sel = step.get("selector").and_then(|s| s.as_str()).unwrap_or("");
                    let text = step.get("text").and_then(|s| s.as_str()).unwrap_or("");
                    match crate::cdp_page::page_type(&cdp, sel, text).await {
                        Ok(()) => serde_json::json!({"step": i, "action": action, "ok": true}),
                        Err(e) => serde_json::json!({"step": i, "action": action, "ok": false, "error": e}),
                    }
                }
                "evaluate" => {
                    let expr = step.get("expression").and_then(|s| s.as_str()).unwrap_or("null");
                    match crate::cdp_page::page_evaluate(&cdp, expr).await {
                        Ok(v) => serde_json::json!({"step": i, "action": action, "ok": true, "value": v}),
                        Err(e) => serde_json::json!({"step": i, "action": action, "ok": false, "error": e}),
                    }
                }
                "content" => {
                    let mode = step.get("mode").and_then(|s| s.as_str()).unwrap_or("text");
                    match crate::cdp_page::page_content(&cdp, mode).await {
                        Ok(p) => serde_json::json!({"step": i, "action": action, "ok": true, "title": p.title, "len": p.content.len()}),
                        Err(e) => serde_json::json!({"step": i, "action": action, "ok": false, "error": e}),
                    }
                }
                other => serde_json::json!({"step": i, "action": other, "ok": false, "error": "unknown action"}),
            };
            results.push(step_result);
        }
        Ok(serde_json::json!({
            "ok": true,
            "session_id": session.id,
            "cdp_ws_url": session.cdp_ws_url,
            "results": results,
        }))
    }

    pub async fn cookie_robot(
        &self,
        profile_id: &str,
        urls: Vec<String>,
        headed: bool,
        export_after: bool,
    ) -> Result<serde_json::Value, OpenAntyError> {
        let session = self
            .launch_session(LaunchSessionRequest {
                profile_id: profile_id.into(),
                headed,
                start_url: urls.first().cloned().or_else(|| Some("about:blank".into())),
                ttl_seconds: 3600,
                force: false,
                locale_from_proxy: true,
            })
            .await?;
        let cdp = session
            .cdp_ws_url
            .clone()
            .ok_or_else(|| OpenAntyError::app(ErrorCode::Internal, "no cdp"))?;
        let mut visited = Vec::new();
        for url in &urls {
            match crate::cdp_page::page_navigate(&cdp, url).await {
                Ok(p) => visited.push(serde_json::json!({"url": p.url, "title": p.title, "ok": true})),
                Err(e) => visited.push(serde_json::json!({"url": url, "ok": false, "error": e})),
            }
            tokio::time::sleep(StdDuration::from_millis(800)).await;
        }
        // Harvest cookies from live browser when possible
        if let Some(ws) = &session.cdp_ws_url {
            if let Ok(cookies) = harvest_cookies_cdp(ws).await {
                let _ = self.import_cookies(profile_id, cookies, true);
            }
        }
        let exported = if export_after {
            Some(self.export_cookies(profile_id)?)
        } else {
            None
        };
        let _ = self.stop_session(&session.id).await;
        Ok(serde_json::json!({
            "ok": true,
            "profile_id": profile_id,
            "visited": visited,
            "cookies": exported,
            "cookie_count": exported.as_ref().map(|c| c.len()).unwrap_or(0),
        }))
    }

    pub async fn synchronizer_navigate(
        &self,
        master_session_id: &str,
        follower_session_ids: Vec<String>,
        url: &str,
    ) -> Result<serde_json::Value, OpenAntyError> {
        let mut results = Vec::new();
        let mut all_ids = vec![master_session_id.to_string()];
        all_ids.extend(follower_session_ids);
        for sid in all_ids {
            match self.get_session(&sid) {
                Ok(ses) => {
                    if let Some(cdp) = ses.cdp_ws_url {
                        match crate::cdp_page::page_navigate(&cdp, url).await {
                            Ok(p) => results.push(serde_json::json!({
                                "session_id": sid, "ok": true, "url": p.url, "title": p.title
                            })),
                            Err(e) => results.push(serde_json::json!({
                                "session_id": sid, "ok": false, "error": e
                            })),
                        }
                    } else {
                        results.push(serde_json::json!({
                            "session_id": sid, "ok": false, "error": "no cdp"
                        }));
                    }
                }
                Err(e) => results.push(serde_json::json!({
                    "session_id": sid, "ok": false, "error": e.to_string()
                })),
            }
        }
        Ok(serde_json::json!({ "ok": true, "url": url, "results": results }))
    }
}

fn json_check(id: &str, pass: bool, detail: String) -> serde_json::Value {
    serde_json::json!({ "id": id, "pass": pass, "detail": detail })
}

fn fastrand_port(start: u16, end: u16) -> u16 {
    use rand::Rng;
    let span = end.saturating_sub(start).max(1);
    start + rand::thread_rng().gen_range(0..=span)
}

async fn cdp_port_alive(port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/json/version");
    let client = match reqwest::Client::builder()
        .timeout(StdDuration::from_millis(500))
        .connect_timeout(StdDuration::from_millis(300))
        .no_proxy()
        .build()
    {
        Ok(c) => c,
        Err(_) => return false,
    };
    matches!(client.get(&url).send().await, Ok(r) if r.status().is_success())
}

async fn wait_for_cdp(port: u16, attempts: u32) -> Result<String, String> {
    let url = format!("http://127.0.0.1:{port}/json/version");
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_millis(800))
        .connect_timeout(StdDuration::from_millis(400))
        .no_proxy()
        .build()
        .map_err(|e| e.to_string())?;
    for i in 0..attempts {
        tokio::time::sleep(StdDuration::from_millis(150)).await;
        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    if let Some(ws) = v.get("webSocketDebuggerUrl").and_then(|x| x.as_str()) {
                        return Ok(ws.to_string());
                    }
                }
            }
            Err(e) => {
                if i + 1 == attempts {
                    return Err(format!(
                        "timed out waiting for CDP on 127.0.0.1:{port}: {e}"
                    ));
                }
            }
        }
    }
    Err(format!(
        "timed out waiting for CDP on 127.0.0.1:{port}"
    ))
}

async fn apply_cookies_cdp(cdp_ws: &str, cookies: &[Cookie]) -> Result<u32, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let connect = tokio::time::timeout(StdDuration::from_secs(3), connect_async(cdp_ws))
        .await
        .map_err(|_| "cdp connect timeout".to_string())?
        .map_err(|e| e.to_string())?;
    let (ws, _) = connect;
    let (mut write, mut read) = ws.split();

    // Enable Network domain before setCookie (required on modern Chrome).
    let enable = serde_json::json!({"id": 1, "method": "Network.enable", "params": {}});
    write
        .send(Message::Text(enable.to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    let _ = tokio::time::timeout(StdDuration::from_millis(800), read.next()).await;

    let mut applied = 0u32;
    let mut id = 2i64;
    for c in cookies {
        let same_site = match c.same_site.as_deref() {
            Some("Strict") | Some("strict") => "Strict",
            Some("None") | Some("none") => "None",
            _ => "Lax",
        };
        let mut params = serde_json::json!({
            "name": c.name,
            "value": c.value,
            "domain": c.domain,
            "path": if c.path.is_empty() { "/".into() } else { c.path.clone() },
            "secure": c.secure,
            "httpOnly": c.http_only,
            "sameSite": same_site,
        });
        if c.expires > 0.0 {
            params["expires"] = serde_json::json!(c.expires);
        }
        let msg = serde_json::json!({
            "id": id,
            "method": "Network.setCookie",
            "params": params,
        });
        write
            .send(Message::Text(msg.to_string().into()))
            .await
            .map_err(|e| e.to_string())?;
        let deadline = tokio::time::Instant::now() + StdDuration::from_secs(2);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(StdDuration::from_millis(500), read.next()).await {
                Ok(Some(Ok(Message::Text(t)))) => {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                        if v.get("id").and_then(|x| x.as_i64()) == Some(id) {
                            // success: result.success == true (Chrome) or result present
                            let ok = v
                                .pointer("/result/success")
                                .and_then(|x| x.as_bool())
                                .unwrap_or_else(|| v.get("result").is_some() && v.get("error").is_none());
                            if ok {
                                applied += 1;
                            }
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
        id += 1;
    }
    Ok(applied)
}

async fn harvest_cookies_cdp(cdp_ws: &str) -> Result<Vec<Cookie>, String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    let (ws, _) = connect_async(cdp_ws)
        .await
        .map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();
    let msg = serde_json::json!({
        "id": 1,
        "method": "Network.getAllCookies",
    });
    write
        .send(Message::Text(msg.to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    let deadline = tokio::time::Instant::now() + StdDuration::from_secs(3);
    while tokio::time::Instant::now() < deadline {
        if let Ok(Some(Ok(Message::Text(t)))) =
            tokio::time::timeout(StdDuration::from_millis(500), read.next()).await
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                if v.get("id").and_then(|x| x.as_i64()) == Some(1) {
                    let mut out = Vec::new();
                    if let Some(arr) = v
                        .pointer("/result/cookies")
                        .and_then(|c| c.as_array())
                    {
                        for c in arr {
                            out.push(Cookie {
                                name: c
                                    .get("name")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .into(),
                                value: c
                                    .get("value")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .into(),
                                domain: c
                                    .get("domain")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("")
                                    .into(),
                                path: c
                                    .get("path")
                                    .and_then(|x| x.as_str())
                                    .unwrap_or("/")
                                    .into(),
                                expires: c.get("expires").and_then(|x| x.as_f64()).unwrap_or(-1.0),
                                http_only: c
                                    .get("httpOnly")
                                    .and_then(|x| x.as_bool())
                                    .unwrap_or(false),
                                secure: c.get("secure").and_then(|x| x.as_bool()).unwrap_or(false),
                                same_site: c
                                    .get("sameSite")
                                    .and_then(|x| x.as_str())
                                    .map(|s| s.to_string()),
                                partition_key: None,
                            });
                        }
                    }
                    return Ok(out);
                }
            }
        }
    }
    Err("timeout harvesting cookies".into())
}

async fn inject_stealth_init(cdp_ws: &str) -> Result<(), String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    let script = r#"
Object.defineProperty(navigator, 'webdriver', { get: () => undefined });
if (!window.chrome) { window.chrome = { runtime: {} }; }
"#;
    let (ws, _) = connect_async(cdp_ws).await.map_err(|e| e.to_string())?;
    let (mut write, mut read) = ws.split();
    let msg = serde_json::json!({
        "id": 99,
        "method": "Page.addScriptToEvaluateOnNewDocument",
        "params": { "source": script }
    });
    write
        .send(Message::Text(msg.to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    let _ = tokio::time::timeout(StdDuration::from_secs(1), read.next()).await;
    Ok(())
}

fn kill_process(pid: u32) {
    if pid == 0 {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("kill")
            .args(["-9", &pid.to_string()])
            .status();
    }
}

fn kill_by_debug_port(port: u16) {
    #[cfg(target_os = "windows")]
    {
        // Best-effort: kill chrome processes that reference this debugging port.
        let filter = format!("eq *remote-debugging-port={port}*");
        let _ = Command::new("wmic")
            .args([
                "process",
                "where",
                &format!("CommandLine like '%remote-debugging-port={port}%'"),
                "call",
                "terminate",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = filter;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = Command::new("pkill")
            .args(["-f", &format!("remote-debugging-port={port}")])
            .status();
    }
}

async fn cdp_browser_close(cdp_ws: &str) -> Result<(), String> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::{connect_async, tungstenite::Message};
    let connect = tokio::time::timeout(StdDuration::from_secs(2), connect_async(cdp_ws))
        .await
        .map_err(|_| "cdp connect timeout".to_string())?
        .map_err(|e| e.to_string())?;
    let (ws, _) = connect;
    let (mut write, mut read) = ws.split();
    let msg = serde_json::json!({"id": 1, "method": "Browser.close"});
    write
        .send(Message::Text(msg.to_string().into()))
        .await
        .map_err(|e| e.to_string())?;
    let _ = tokio::time::timeout(StdDuration::from_millis(800), read.next()).await;
    Ok(())
}

