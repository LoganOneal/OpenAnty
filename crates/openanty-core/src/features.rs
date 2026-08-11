//! Phase B/C/D feature storage helpers: proxy pool, extensions, scenarios, users.

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyPoolItem {
    pub id: String,
    pub name: String,
    pub server: String,
    pub username: Option<String>,
    pub password: Option<String>,
    pub last_status: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionItem {
    pub id: String,
    pub name: String,
    pub path: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioItem {
    pub id: String,
    pub name: String,
    pub steps: Vec<Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserItem {
    pub id: String,
    pub username: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
}

pub fn ensure_feature_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS proxy_pool (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            server TEXT NOT NULL,
            username TEXT,
            password TEXT,
            last_status TEXT,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS extensions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            path TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS scenarios (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            steps_json TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            role TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )
    .map_err(|e| e.to_string())
}

pub fn list_proxy_pool(conn: &Connection) -> Result<Vec<ProxyPoolItem>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id,name,server,username,password,last_status,created_at FROM proxy_pool ORDER BY created_at DESC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ProxyPoolItem {
                id: r.get(0)?,
                name: r.get(1)?,
                server: r.get(2)?,
                username: r.get(3)?,
                password: r.get(4)?,
                last_status: r.get(5)?,
                created_at: parse_ts(r.get::<_, String>(6)?),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn add_proxy_pool(
    conn: &Connection,
    name: &str,
    server: &str,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<ProxyPoolItem, String> {
    let id = format!("pxp_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    conn.execute(
        "INSERT INTO proxy_pool(id,name,server,username,password,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
        params![id, name, server, username, password, now.to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(ProxyPoolItem {
        id,
        name: name.into(),
        server: server.into(),
        username: username.map(|s| s.into()),
        password: password.map(|s| s.into()),
        last_status: None,
        created_at: now,
    })
}

pub fn delete_proxy_pool(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM proxy_pool WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

pub fn list_extensions(conn: &Connection) -> Result<Vec<ExtensionItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id,name,path,enabled,created_at FROM extensions ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(ExtensionItem {
                id: r.get(0)?,
                name: r.get(1)?,
                path: r.get(2)?,
                enabled: r.get::<_, i32>(3)? != 0,
                created_at: parse_ts(r.get::<_, String>(4)?),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn add_extension(conn: &Connection, name: &str, path: &str, enabled: bool) -> Result<ExtensionItem, String> {
    let id = format!("ext_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    conn.execute(
        "INSERT INTO extensions(id,name,path,enabled,created_at) VALUES(?1,?2,?3,?4,?5)",
        params![id, name, path, enabled as i32, now.to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(ExtensionItem {
        id,
        name: name.into(),
        path: path.into(),
        enabled,
        created_at: now,
    })
}

pub fn delete_extension(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM extensions WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

pub fn list_scenarios(conn: &Connection) -> Result<Vec<ScenarioItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id,name,steps_json,created_at FROM scenarios ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            let steps_json: String = r.get(2)?;
            let steps: Vec<Value> = serde_json::from_str(&steps_json).unwrap_or_default();
            Ok(ScenarioItem {
                id: r.get(0)?,
                name: r.get(1)?,
                steps,
                created_at: parse_ts(r.get::<_, String>(3)?),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn save_scenario(conn: &Connection, name: &str, steps: &[Value]) -> Result<ScenarioItem, String> {
    let id = format!("scn_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    let steps_json = serde_json::to_string(steps).map_err(|e| e.to_string())?;
    conn.execute(
        "INSERT INTO scenarios(id,name,steps_json,created_at) VALUES(?1,?2,?3,?4)",
        params![id, name, steps_json, now.to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(ScenarioItem {
        id,
        name: name.into(),
        steps: steps.to_vec(),
        created_at: now,
    })
}

pub fn delete_scenario(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM scenarios WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

pub fn get_scenario(conn: &Connection, id: &str) -> Result<Option<ScenarioItem>, String> {
    conn.query_row(
        "SELECT id,name,steps_json,created_at FROM scenarios WHERE id=?1",
        params![id],
        |r| {
            let steps_json: String = r.get(2)?;
            let steps: Vec<Value> = serde_json::from_str(&steps_json).unwrap_or_default();
            Ok(ScenarioItem {
                id: r.get(0)?,
                name: r.get(1)?,
                steps,
                created_at: parse_ts(r.get::<_, String>(3)?),
            })
        },
    )
    .optional()
    .map_err(|e| e.to_string())
}

pub fn list_users(conn: &Connection) -> Result<Vec<UserItem>, String> {
    let mut stmt = conn
        .prepare("SELECT id,username,role,created_at FROM users ORDER BY created_at DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |r| {
            Ok(UserItem {
                id: r.get(0)?,
                username: r.get(1)?,
                role: r.get(2)?,
                created_at: parse_ts(r.get::<_, String>(3)?),
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

pub fn add_user(conn: &Connection, username: &str, role: &str) -> Result<UserItem, String> {
    let id = format!("usr_{}", Uuid::new_v4().simple());
    let now = Utc::now();
    conn.execute(
        "INSERT INTO users(id,username,role,created_at) VALUES(?1,?2,?3,?4)",
        params![id, username, role, now.to_rfc3339()],
    )
    .map_err(|e| e.to_string())?;
    Ok(UserItem {
        id,
        username: username.into(),
        role: role.into(),
        created_at: now,
    })
}

pub fn delete_user(conn: &Connection, id: &str) -> Result<bool, String> {
    let n = conn
        .execute("DELETE FROM users WHERE id=?1", params![id])
        .map_err(|e| e.to_string())?;
    Ok(n > 0)
}

fn parse_ts(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}
