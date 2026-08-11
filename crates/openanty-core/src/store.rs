//! SQLite metadata store with encrypted sensitive blobs.

use chrono::{DateTime, Utc};
use openanty_proto::{Cookie, FingerprintDocument, ProxyConfig, SessionStatus};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::crypto::MasterKey;

pub struct Store {
    conn: Mutex<Connection>,
    key: MasterKey,
    data_dir: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ProfileRow {
    pub id: String,
    pub name: String,
    pub tags_json: String,
    pub notes: Option<String>,
    pub fingerprint: FingerprintDocument,
    pub fingerprint_hash: String,
    pub proxy: Option<ProxyConfig>,
    pub cookies: Vec<Cookie>,
    pub cookies_pending_apply: bool,
    pub data_path: PathBuf,
    pub lock_session_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionRow {
    pub id: String,
    pub profile_id: String,
    pub pid: Option<u32>,
    pub debug_port: Option<u16>,
    pub cdp_ws_url: Option<String>,
    pub status: SessionStatus,
    pub headed: bool,
    pub started_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
}

impl Store {
    pub fn open(data_dir: &Path, key: MasterKey) -> Result<Self, String> {
        crate::paths::ensure_dir(data_dir).map_err(|e| e.to_string())?;
        let db_path = data_dir.join("openanty.db");
        // Migrate legacy misnamed DB from rebrand glitch
        let legacy = data_dir.join("OpenAnty.db");
        if !db_path.exists() && legacy.exists() {
            let _ = std::fs::rename(&legacy, &db_path);
        }
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY
            );
            CREATE TABLE IF NOT EXISTS profiles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                tags_json TEXT NOT NULL DEFAULT '[]',
                notes TEXT,
                fingerprint_ciphertext BLOB NOT NULL,
                fingerprint_hash TEXT NOT NULL,
                proxy_ciphertext BLOB,
                cookies_ciphertext BLOB,
                cookies_pending_apply INTEGER NOT NULL DEFAULT 0,
                data_path TEXT NOT NULL,
                lock_session_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                deleted_at TEXT
            );
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                pid INTEGER,
                debug_port INTEGER,
                cdp_ws_url TEXT,
                status TEXT NOT NULL,
                headed INTEGER NOT NULL,
                started_at TEXT NOT NULL,
                expires_at TEXT,
                last_heartbeat_at TEXT
            );
            CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts TEXT NOT NULL,
                action TEXT NOT NULL,
                entity_type TEXT,
                entity_id TEXT,
                payload_json TEXT
            );
            "#,
        )
        .map_err(|e| e.to_string())?;

        let store = Self {
            conn: Mutex::new(conn),
            key,
            data_dir: data_dir.to_path_buf(),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let version: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        if version < 1 {
            conn.execute("INSERT INTO schema_migrations(version) VALUES (1)", [])
                .map_err(|e| e.to_string())?;
        }
        // Phase B/C/D feature tables
        crate::features::ensure_feature_tables(&conn)?;
        if version < 2 {
            conn.execute("INSERT INTO schema_migrations(version) VALUES (2)", [])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    /// Raw connection access for feature tables (short lock).
    pub fn with_conn<T>(&self, f: impl FnOnce(&Connection) -> Result<T, String>) -> Result<T, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        f(&conn)
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn insert_profile(&self, row: &ProfileRow) -> Result<(), String> {
        let fp = self.key.encrypt_json(&row.fingerprint)?;
        let proxy = match &row.proxy {
            Some(p) => Some(self.key.encrypt_json(p)?),
            None => None,
        };
        let cookies = if row.cookies.is_empty() {
            None
        } else {
            Some(self.key.encrypt_json(&row.cookies)?)
        };
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"INSERT INTO profiles(
                id, name, tags_json, notes, fingerprint_ciphertext, fingerprint_hash,
                proxy_ciphertext, cookies_ciphertext, cookies_pending_apply, data_path,
                lock_session_id, created_at, updated_at
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)"#,
            params![
                row.id,
                row.name,
                row.tags_json,
                row.notes,
                fp,
                row.fingerprint_hash,
                proxy,
                cookies,
                row.cookies_pending_apply as i32,
                row.data_path.to_string_lossy(),
                row.lock_session_id,
                row.created_at.to_rfc3339(),
                row.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_profile(&self, row: &ProfileRow) -> Result<(), String> {
        let fp = self.key.encrypt_json(&row.fingerprint)?;
        let proxy = match &row.proxy {
            Some(p) => Some(self.key.encrypt_json(p)?),
            None => None,
        };
        let cookies = if row.cookies.is_empty() {
            None
        } else {
            Some(self.key.encrypt_json(&row.cookies)?)
        };
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"UPDATE profiles SET
                name=?2, tags_json=?3, notes=?4, fingerprint_ciphertext=?5, fingerprint_hash=?6,
                proxy_ciphertext=?7, cookies_ciphertext=?8, cookies_pending_apply=?9,
                lock_session_id=?10, updated_at=?11
              WHERE id=?1 AND deleted_at IS NULL"#,
            params![
                row.id,
                row.name,
                row.tags_json,
                row.notes,
                fp,
                row.fingerprint_hash,
                proxy,
                cookies,
                row.cookies_pending_apply as i32,
                row.lock_session_id,
                row.updated_at.to_rfc3339(),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_profile(&self, id: &str) -> Result<Option<ProfileRow>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, name, tags_json, notes, fingerprint_ciphertext, fingerprint_hash,
                          proxy_ciphertext, cookies_ciphertext, cookies_pending_apply, data_path,
                          lock_session_id, created_at, updated_at
                   FROM profiles WHERE id=?1 AND deleted_at IS NULL"#,
            )
            .map_err(|e| e.to_string())?;
        let row = stmt
            .query_row(params![id], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                    r.get::<_, Option<Vec<u8>>>(7)?,
                    r.get::<_, i32>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, Option<String>>(10)?,
                    r.get::<_, String>(11)?,
                    r.get::<_, String>(12)?,
                ))
            })
            .optional()
            .map_err(|e| e.to_string())?;
        drop(stmt);
        drop(conn);
        row.map(|t| self.map_profile(t)).transpose()
    }

    #[allow(clippy::type_complexity)]
    fn map_profile(
        &self,
        t: (
            String,
            String,
            String,
            Option<String>,
            Vec<u8>,
            String,
            Option<Vec<u8>>,
            Option<Vec<u8>>,
            i32,
            String,
            Option<String>,
            String,
            String,
        ),
    ) -> Result<ProfileRow, String> {
        let (
            id,
            name,
            tags_json,
            notes,
            fp_ct,
            fingerprint_hash,
            proxy_ct,
            cookies_ct,
            pending,
            data_path,
            lock_session_id,
            created_at,
            updated_at,
        ) = t;
        let fingerprint: FingerprintDocument = self.key.decrypt_json(&fp_ct)?;
        let proxy = match proxy_ct {
            Some(b) => Some(self.key.decrypt_json(&b)?),
            None => None,
        };
        let cookies: Vec<Cookie> = match cookies_ct {
            Some(b) => self.key.decrypt_json(&b)?,
            None => vec![],
        };
        Ok(ProfileRow {
            id,
            name,
            tags_json,
            notes,
            fingerprint,
            fingerprint_hash,
            proxy,
            cookies,
            cookies_pending_apply: pending != 0,
            data_path: PathBuf::from(data_path),
            lock_session_id,
            created_at: DateTime::parse_from_rfc3339(&created_at)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| e.to_string())?,
            updated_at: DateTime::parse_from_rfc3339(&updated_at)
                .map(|d| d.with_timezone(&Utc))
                .map_err(|e| e.to_string())?,
        })
    }

    pub fn list_profiles(&self, limit: u32) -> Result<Vec<ProfileRow>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                r#"SELECT id, name, tags_json, notes, fingerprint_ciphertext, fingerprint_hash,
                          proxy_ciphertext, cookies_ciphertext, cookies_pending_apply, data_path,
                          lock_session_id, created_at, updated_at
                   FROM profiles WHERE deleted_at IS NULL
                   ORDER BY updated_at DESC LIMIT ?1"#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit as i64], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Vec<u8>>(4)?,
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<Vec<u8>>>(6)?,
                    r.get::<_, Option<Vec<u8>>>(7)?,
                    r.get::<_, i32>(8)?,
                    r.get::<_, String>(9)?,
                    r.get::<_, Option<String>>(10)?,
                    r.get::<_, String>(11)?,
                    r.get::<_, String>(12)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            let t = row.map_err(|e| e.to_string())?;
            out.push(self.map_profile(t)?);
        }
        Ok(out)
    }

    pub fn soft_delete_profile(&self, id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let n = conn
            .execute(
                "UPDATE profiles SET deleted_at=?2 WHERE id=?1 AND deleted_at IS NULL",
                params![id, Utc::now().to_rfc3339()],
            )
            .map_err(|e| e.to_string())?;
        Ok(n > 0)
    }

    pub fn insert_session(&self, row: &SessionRow) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"INSERT INTO sessions(id, profile_id, pid, debug_port, cdp_ws_url, status, headed,
                                    started_at, expires_at, last_heartbeat_at)
               VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)"#,
            params![
                row.id,
                row.profile_id,
                row.pid.map(|p| p as i64),
                row.debug_port.map(|p| p as i64),
                row.cdp_ws_url,
                row.status.as_str(),
                row.headed as i32,
                row.started_at.to_rfc3339(),
                row.expires_at.map(|t| t.to_rfc3339()),
                row.last_heartbeat_at.map(|t| t.to_rfc3339()),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_session(&self, row: &SessionRow) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            r#"UPDATE sessions SET pid=?2, debug_port=?3, cdp_ws_url=?4, status=?5,
                   expires_at=?6, last_heartbeat_at=?7 WHERE id=?1"#,
            params![
                row.id,
                row.pid.map(|p| p as i64),
                row.debug_port.map(|p| p as i64),
                row.cdp_ws_url,
                row.status.as_str(),
                row.expires_at.map(|t| t.to_rfc3339()),
                row.last_heartbeat_at.map(|t| t.to_rfc3339()),
            ],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn get_session(&self, id: &str) -> Result<Option<SessionRow>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            r#"SELECT id, profile_id, pid, debug_port, cdp_ws_url, status, headed,
                      started_at, expires_at, last_heartbeat_at
               FROM sessions WHERE id=?1"#,
            params![id],
            |r| {
                Ok(SessionRow {
                    id: r.get(0)?,
                    profile_id: r.get(1)?,
                    pid: r.get::<_, Option<i64>>(2)?.map(|p| p as u32),
                    debug_port: r.get::<_, Option<i64>>(3)?.map(|p| p as u16),
                    cdp_ws_url: r.get(4)?,
                    status: SessionStatus::parse(&r.get::<_, String>(5)?)
                        .unwrap_or(SessionStatus::Stopped),
                    headed: r.get::<_, i32>(6)? != 0,
                    started_at: DateTime::parse_from_rfc3339(&r.get::<_, String>(7)?)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: r
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|d| d.with_timezone(&Utc)),
                    last_heartbeat_at: r
                        .get::<_, Option<String>>(9)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|d| d.with_timezone(&Utc)),
                })
            },
        )
        .optional()
        .map_err(|e| e.to_string())
    }

    pub fn list_sessions(&self, running_only: bool) -> Result<Vec<SessionRow>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let sql = if running_only {
            r#"SELECT id, profile_id, pid, debug_port, cdp_ws_url, status, headed,
                      started_at, expires_at, last_heartbeat_at
               FROM sessions WHERE status IN ('starting','running')
               ORDER BY started_at DESC"#
        } else {
            r#"SELECT id, profile_id, pid, debug_port, cdp_ws_url, status, headed,
                      started_at, expires_at, last_heartbeat_at
               FROM sessions ORDER BY started_at DESC LIMIT 100"#
        };
        let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SessionRow {
                    id: r.get(0)?,
                    profile_id: r.get(1)?,
                    pid: r.get::<_, Option<i64>>(2)?.map(|p| p as u32),
                    debug_port: r.get::<_, Option<i64>>(3)?.map(|p| p as u16),
                    cdp_ws_url: r.get(4)?,
                    status: SessionStatus::parse(&r.get::<_, String>(5)?)
                        .unwrap_or(SessionStatus::Stopped),
                    headed: r.get::<_, i32>(6)? != 0,
                    started_at: DateTime::parse_from_rfc3339(&r.get::<_, String>(7)?)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    expires_at: r
                        .get::<_, Option<String>>(8)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|d| d.with_timezone(&Utc)),
                    last_heartbeat_at: r
                        .get::<_, Option<String>>(9)?
                        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                        .map(|d| d.with_timezone(&Utc)),
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn audit(&self, action: &str, entity_type: &str, entity_id: &str, payload: &str) {
        if let Ok(conn) = self.conn.lock() {
            let _ = conn.execute(
                "INSERT INTO audit_events(ts, action, entity_type, entity_id, payload_json) VALUES (?1,?2,?3,?4,?5)",
                params![Utc::now().to_rfc3339(), action, entity_type, entity_id, payload],
            );
        }
    }
}
