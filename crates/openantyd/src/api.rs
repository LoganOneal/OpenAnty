//! REST API (OpenAPI-aligned paths from DESIGN.md) + UI + phase feature routes.

use axum::extract::{Path, Query, State};
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use openanty_core::OpenAntyService;
use openanty_proto::*;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use crate::ui_static;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<OpenAntyService>,
}

pub fn router(service: Arc<OpenAntyService>) -> Router {
    let state = AppState { service };
    Router::new()
        // Control panel UI (Dolphin-like)
        .route("/", get(ui_index))
        .route("/ui", get(ui_index))
        .route("/assets/app.css", get(ui_css))
        .route("/assets/app.js", get(ui_js))
        .route("/v1/ui/bootstrap", get(ui_bootstrap))
        // Core
        .route("/v1/system/status", get(system_status))
        .route("/v1/system/doctor", get(doctor))
        .route("/v1/profiles", get(list_profiles).post(create_profile))
        .route("/v1/profiles/bulk", post(bulk_profiles))
        .route(
            "/v1/profiles/{id}",
            get(get_profile).delete(delete_profile),
        )
        .route("/v1/profiles/{id}/proxy", put(apply_proxy))
        .route("/v1/profiles/{id}/cookies/import", post(import_cookies))
        .route("/v1/profiles/{id}/cookies/export", get(export_cookies))
        .route("/v1/sessions", get(list_sessions).post(launch_session))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/stop", post(stop_session))
        .route("/v1/sessions/{id}/heartbeat", post(heartbeat))
        .route("/v1/sessions/{id}/cdp", get(get_cdp))
        // Phase B/C/D
        .route("/v1/proxy-pool", get(list_proxy_pool).post(add_proxy_pool))
        .route("/v1/proxy-pool/{id}", delete(delete_proxy_pool))
        .route("/v1/extensions", get(list_extensions).post(add_extension))
        .route("/v1/extensions/{id}", delete(delete_extension))
        .route("/v1/scenarios", get(list_scenarios).post(save_scenario))
        .route("/v1/scenarios/{id}", delete(delete_scenario))
        .route("/v1/scenarios/run", post(run_scenario))
        .route("/v1/cookie-robot/run", post(cookie_robot))
        .route("/v1/synchronizer/navigate", post(sync_navigate))
        .route("/v1/users", get(list_users).post(add_user))
        .route("/v1/users/{id}", delete(delete_user))
        .route("/v1/fingerprint/health", post(fp_health))
        // AdsPower-compatible shim subset
        .route("/browser/list", get(adspower_list))
        .route("/browser/start", get(adspower_start))
        .route("/browser/stop", get(adspower_stop))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn auth(headers: &HeaderMap, service: &OpenAntyService) -> Result<(), ApiError> {
    if std::env::var("OPENANTY_INSECURE_NO_TOKEN").ok().as_deref() == Some("1") {
        return Ok(());
    }
    let expected = &service.token;
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| {
            headers
                .get("x-openanty-token")
                .and_then(|v| v.to_str().ok())
        });
    match provided {
        Some(t) if t == expected => Ok(()),
        _ => Err(ApiError::from_err(
            OpenAntyError::app(ErrorCode::Unauthorized, "missing or invalid API token"),
            OpenAntyService::request_id(),
        )),
    }
}

fn host_ok(headers: &HeaderMap) -> Result<(), ApiError> {
    if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
        let host = host.split(':').next().unwrap_or(host);
        let allowed = ["127.0.0.1", "localhost", "[::1]", "::1"];
        if !allowed.contains(&host) {
            return Err(ApiError::from_err(
                OpenAntyError::app(
                    ErrorCode::UnauthorizedBind,
                    format!("Host header not allowlisted: {host}"),
                ),
                OpenAntyService::request_id(),
            ));
        }
    }
    Ok(())
}

struct ApiError {
    status: StatusCode,
    body: serde_json::Value,
}

impl ApiError {
    fn from_err(err: OpenAntyError, request_id: String) -> Self {
        let body = err.body();
        let status = match err.code() {
            ErrorCode::Unauthorized | ErrorCode::UnauthorizedBind => StatusCode::UNAUTHORIZED,
            ErrorCode::ProfileNotFound | ErrorCode::SessionNotFound => StatusCode::NOT_FOUND,
            ErrorCode::InvalidRequest | ErrorCode::FingerprintInconsistent => {
                StatusCode::BAD_REQUEST
            }
            ErrorCode::SessionAlreadyRunning | ErrorCode::ResourceLimit | ErrorCode::PortConflict => {
                StatusCode::CONFLICT
            }
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        Self {
            status,
            body: json!({
                "ok": false,
                "error": body,
                "request_id": request_id,
            }),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.body)).into_response()
    }
}

async fn ui_index(State(state): State<AppState>) -> impl IntoResponse {
    let html = ui_static::index_with_token(&state.service.token, env!("CARGO_PKG_VERSION"));
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        Html(html),
    )
}

async fn ui_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        ui_static::APP_CSS,
    )
}

async fn ui_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "application/javascript; charset=utf-8")],
        ui_static::APP_JS,
    )
}

async fn ui_bootstrap(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    // Localhost-only convenience: inject token for control panel (desktop-app UX)
    Ok(Json(json!({
        "ok": true,
        "token": state.service.token,
        "version": env!("CARGO_PKG_VERSION"),
        "data_dir": state.service.data_dir.display().to_string(),
        "api_base": state.service.config.api_base(),
    })))
}

async fn system_status(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SystemStatus>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    Ok(Json(state.service.system_status()))
}

async fn doctor(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    Ok(Json(state.service.doctor()))
}

#[derive(Deserialize)]
struct ListQuery {
    limit: Option<u32>,
}

async fn list_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<ProfileListResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let items = state
        .service
        .list_profiles(q.limit.unwrap_or(50))
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(ProfileListResult {
        ok: true,
        error: None,
        request_id: rid,
        items,
        next_cursor: None,
    }))
}

async fn create_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<CreateProfileRequest>,
) -> Result<Json<ProfileResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let profile = state
        .service
        .create_profile(req)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(ProfileResult::success(rid, profile)))
}

#[derive(Deserialize)]
struct BulkBody {
    count: Option<u32>,
    name_prefix: Option<String>,
    tags: Option<Vec<String>>,
}

async fn bulk_profiles(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<BulkBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let items = state
        .service
        .bulk_create_profiles(
            body.count.unwrap_or(5),
            body.name_prefix.as_deref().unwrap_or("profile"),
            body.tags,
        )
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(json!({ "ok": true, "request_id": rid, "items": items, "count": items.len() })))
}

#[derive(Deserialize)]
struct GetProfileQuery {
    include_secrets: Option<bool>,
}

async fn get_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Query(q): Query<GetProfileQuery>,
) -> Result<Json<ProfileResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let profile = state
        .service
        .get_profile(&id, q.include_secrets.unwrap_or(false))
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(ProfileResult::success(rid, profile)))
}

async fn delete_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<OkResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    state
        .service
        .delete_profile(&id)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(OkResult::success(rid, Some("deleted".into()))))
}

async fn apply_proxy(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ApplyProxyRequest>,
) -> Result<Json<ProxyApplyResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let (profile, proxy_status, regenerated) = state
        .service
        .apply_proxy(&id, req)
        .await
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(ProxyApplyResult {
        ok: true,
        error: None,
        request_id: rid,
        profile_id: Some(profile.id),
        proxy_status: Some(proxy_status),
        fingerprint_hash: Some(profile.fingerprint_hash),
        fingerprint_regenerated: regenerated,
    }))
}

#[derive(Deserialize)]
struct ImportCookiesBody {
    cookies: Vec<Cookie>,
    #[serde(default = "default_true")]
    merge: bool,
}

fn default_true() -> bool {
    true
}

async fn import_cookies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<ImportCookiesBody>,
) -> Result<Json<CookieImportResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let (imported, skipped, pending) = state
        .service
        .import_cookies(&id, body.cookies, body.merge)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(CookieImportResult {
        ok: true,
        error: None,
        request_id: rid,
        profile_id: Some(id),
        imported,
        skipped_expired: skipped,
        merged: body.merge,
        applied_live: false,
        cookies_pending_apply: pending,
        failed: vec![],
    }))
}

async fn export_cookies(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<CookieExportResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let cookies = state
        .service
        .export_cookies(&id)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    let count = cookies.len() as u32;
    Ok(Json(CookieExportResult {
        ok: true,
        error: None,
        request_id: rid,
        profile_id: Some(id),
        source: "blob".into(),
        cookies,
        count,
    }))
}

async fn list_sessions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SessionListResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let items = state
        .service
        .list_sessions()
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(SessionListResult {
        ok: true,
        error: None,
        request_id: rid,
        items,
        next_cursor: None,
    }))
}

async fn launch_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<LaunchSessionRequest>,
) -> Result<Json<SessionResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let session = state
        .service
        .launch_session(req)
        .await
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(SessionResult::success(rid, session)))
}

async fn get_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SessionResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let session = state
        .service
        .get_session(&id)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(SessionResult::success(rid, session)))
}

async fn stop_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<OkResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    state
        .service
        .stop_session(&id)
        .await
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(OkResult::success(rid, Some("stopped".into()))))
}

async fn heartbeat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SessionResult>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let rid = OpenAntyService::request_id();
    let session = state
        .service
        .heartbeat(&id)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(SessionResult::success(rid, session)))
}

async fn get_cdp(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SessionResult>, ApiError> {
    get_session(State(state), headers, Path(id)).await
}

// —— Feature routes ——

#[derive(Deserialize)]
struct ProxyPoolBody {
    name: String,
    server: String,
    username: Option<String>,
    password: Option<String>,
}

async fn list_proxy_pool(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let items = state
        .service
        .list_proxy_pool()
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn add_proxy_pool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ProxyPoolBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let item = state
        .service
        .add_proxy_pool(
            &body.name,
            &body.server,
            body.username.as_deref(),
            body.password.as_deref(),
        )
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "item": item })))
}

async fn delete_proxy_pool(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    state
        .service
        .delete_proxy_pool(&id)
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ExtBody {
    name: String,
    path: String,
    enabled: Option<bool>,
}

async fn list_extensions(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let items = state
        .service
        .list_extensions()
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn add_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ExtBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let item = state
        .service
        .add_extension(&body.name, &body.path, body.enabled.unwrap_or(true))
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "item": item })))
}

async fn delete_extension(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    state
        .service
        .delete_extension(&id)
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct ScenarioBody {
    name: Option<String>,
    steps: Option<Vec<serde_json::Value>>,
}

async fn list_scenarios(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let items = state
        .service
        .list_scenarios()
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn save_scenario(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<ScenarioBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let name = body.name.unwrap_or_else(|| "scenario".into());
    let steps = body.steps.unwrap_or_default();
    let item = state
        .service
        .save_scenario(&name, steps)
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "item": item })))
}

async fn delete_scenario(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    state
        .service
        .delete_scenario(&id)
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true })))
}

#[derive(Deserialize)]
struct RunScenarioBody {
    profile_id: String,
    scenario: Option<ScenarioBody>,
    headed: Option<bool>,
}

async fn run_scenario(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RunScenarioBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let steps = body
        .scenario
        .and_then(|s| s.steps)
        .unwrap_or_default();
    let res = state
        .service
        .run_scenario(&body.profile_id, steps, body.headed.unwrap_or(true))
        .await
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct CookieRobotBody {
    profile_id: String,
    urls: Vec<String>,
    headed: Option<bool>,
    export_after: Option<bool>,
}

async fn cookie_robot(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CookieRobotBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let res = state
        .service
        .cookie_robot(
            &body.profile_id,
            body.urls,
            body.headed.unwrap_or(false),
            body.export_after.unwrap_or(true),
        )
        .await
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct SyncBody {
    master_session_id: String,
    follower_session_ids: Vec<String>,
    url: String,
}

async fn sync_navigate(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<SyncBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let res = state
        .service
        .synchronizer_navigate(
            &body.master_session_id,
            body.follower_session_ids,
            &body.url,
        )
        .await
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(res))
}

#[derive(Deserialize)]
struct UserBody {
    username: String,
    role: Option<String>,
}

async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let items = state
        .service
        .list_users()
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "items": items })))
}

async fn add_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<UserBody>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let item = state
        .service
        .add_user(&body.username, body.role.as_deref().unwrap_or("operator"))
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true, "item": item })))
}

async fn delete_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    state
        .service
        .delete_user(&id)
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({ "ok": true })))
}

async fn fp_health(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    Ok(Json(state.service.fingerprint_health_sample()))
}

// —— AdsPower-compatible shim ——

async fn adspower_list(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let items = state
        .service
        .list_profiles(200)
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    let list: Vec<_> = items
        .into_iter()
        .map(|p| {
            json!({
                "user_id": p.id,
                "name": p.name,
                "remark": p.notes.unwrap_or_default(),
                "group_id": "",
                "domain_name": "",
                "username": "",
                "password": "",
                "last_open_time": p.updated_at.to_rfc3339(),
            })
        })
        .collect();
    Ok(Json(json!({ "code": 0, "msg": "success", "data": { "list": list } })))
}

#[derive(Deserialize)]
struct AdsQuery {
    user_id: Option<String>,
}

async fn adspower_start(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AdsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let id = q
        .user_id
        .ok_or_else(|| {
            ApiError::from_err(
                OpenAntyError::app(ErrorCode::InvalidRequest, "user_id required"),
                OpenAntyService::request_id(),
            )
        })?;
    let session = state
        .service
        .launch_session(LaunchSessionRequest {
            profile_id: id,
            headed: true,
            start_url: None,
            ttl_seconds: 3600,
            force: false,
            locale_from_proxy: true,
        })
        .await
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    Ok(Json(json!({
        "code": 0,
        "msg": "success",
        "data": {
            "ws": {
                "puppeteer": session.cdp_ws_url,
                "selenium": session.cdp_ws_url,
            },
            "debug_port": session.debug_port.map(|p| p.to_string()),
            "webdriver": ""
        }
    })))
}

async fn adspower_stop(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(q): Query<AdsQuery>,
) -> Result<Json<serde_json::Value>, ApiError> {
    host_ok(&headers)?;
    auth(&headers, &state.service)?;
    let profile_id = q.user_id.unwrap_or_default();
    let sessions = state
        .service
        .list_sessions()
        .map_err(|e| ApiError::from_err(e, OpenAntyService::request_id()))?;
    for s in sessions {
        if s.profile_id == profile_id {
            let _ = state.service.stop_session(&s.id).await;
        }
    }
    Ok(Json(json!({ "code": 0, "msg": "success" })))
}
