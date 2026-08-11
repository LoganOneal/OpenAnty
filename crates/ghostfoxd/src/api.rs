//! REST API (OpenAPI-aligned paths from DESIGN.md).

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use ghostfox_core::GhostfoxService;
use ghostfox_proto::*;
use serde::Deserialize;
use std::sync::Arc;
use tower_http::trace::TraceLayer;

#[derive(Clone)]
pub struct AppState {
    pub service: Arc<GhostfoxService>,
}

pub fn router(service: Arc<GhostfoxService>) -> Router {
    let state = AppState { service };
    Router::new()
        .route("/v1/system/status", get(system_status))
        .route("/v1/system/doctor", get(doctor))
        .route("/v1/profiles", get(list_profiles).post(create_profile))
        .route(
            "/v1/profiles/{id}",
            get(get_profile).delete(delete_profile),
        )
        .route("/v1/profiles/{id}/proxy", axum::routing::put(apply_proxy))
        .route(
            "/v1/profiles/{id}/cookies/import",
            post(import_cookies),
        )
        .route(
            "/v1/profiles/{id}/cookies/export",
            get(export_cookies),
        )
        .route("/v1/sessions", get(list_sessions).post(launch_session))
        .route("/v1/sessions/{id}", get(get_session))
        .route("/v1/sessions/{id}/stop", post(stop_session))
        .route("/v1/sessions/{id}/heartbeat", post(heartbeat))
        .route("/v1/sessions/{id}/cdp", get(get_cdp))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn auth(headers: &HeaderMap, service: &GhostfoxService) -> Result<(), ApiError> {
    if std::env::var("GHOSTFOX_INSECURE_NO_TOKEN").ok().as_deref() == Some("1") {
        return Ok(());
    }
    let expected = &service.token;
    let provided = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .or_else(|| {
            headers
                .get("x-ghostfox-token")
                .and_then(|v| v.to_str().ok())
        });
    match provided {
        Some(t) if t == expected => Ok(()),
        _ => Err(ApiError::from_err(
            GhostfoxError::app(ErrorCode::Unauthorized, "missing or invalid API token"),
            GhostfoxService::request_id(),
        )),
    }
}

fn host_ok(headers: &HeaderMap) -> Result<(), ApiError> {
    if let Some(host) = headers.get("host").and_then(|v| v.to_str().ok()) {
        let host = host.split(':').next().unwrap_or(host);
        let allowed = ["127.0.0.1", "localhost", "[::1]", "::1"];
        if !allowed.contains(&host) {
            return Err(ApiError::from_err(
                GhostfoxError::app(
                    ErrorCode::UnauthorizedBind,
                    format!("Host header not allowlisted: {host}"),
                ),
                GhostfoxService::request_id(),
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
    fn from_err(err: GhostfoxError, request_id: String) -> Self {
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
            body: serde_json::json!({
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
    let profile = state
        .service
        .create_profile(req)
        .map_err(|e| ApiError::from_err(e, rid.clone()))?;
    Ok(Json(ProfileResult::success(rid, profile)))
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
    let rid = GhostfoxService::request_id();
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
