use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    extract::{Path, State, Query},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tower_http::services::ServeDir;
use tracing::{info, error};

use crate::config::{self, RouteConfig, SettingsUpdate};
// web restart is signalled via the daemon's restart channel; no exec/spawn here.
use crate::daemon::{ActiveNode, SharedNodes, NOT_CONNECTED};

// ─── Shared state ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub restart_tx: mpsc::Sender<i64>,
    pub nodes: SharedNodes,
    pub db_path: String,
    // In-memory session tokens (reset on daemon restart – intentional)
    pub sessions: Arc<RwLock<HashSet<String>>>,
}

// ─── Wire up the server ──────────────────────────────────────────────────────

pub async fn start_web_server(
    bind_addr: String,
    restart_tx: mpsc::Sender<i64>,
    nodes: SharedNodes,
    db_path: String,
    web_dir: Option<String>,
) {
    let state = AppState {
        restart_tx,
        nodes,
        db_path,
        sessions: Arc::new(RwLock::new(HashSet::new())),
    };

    // ── API routes ──────────────────────────────────────────────────────────
    // NOTE: axum matches static segments before parameterised ones, so
    // /api/routes/restart-all will win over /api/routes/:id.
    let api = Router::new()
        .route("/api/login",               post(login))
        .route("/api/routes",              get(list_routes).post(create_route_handler))
        .route("/api/routes/:id/probe",    get(probe_route_handler))
        .route("/api/routes/restart-all",  post(restart_all_handler))
        .route("/api/routes/:id/restart",  post(restart_by_id_handler))
        .route("/api/routes/:id",          put(update_route_handler).delete(delete_route_handler))
        .route("/api/settings",            get(get_settings_handler).put(save_settings_handler))
        .route("/api/countries",           get(get_countries))
        // Legacy CLI endpoint – keep backward-compat
        .route("/restart",                 post(legacy_restart))
        .route("/status",                  get(legacy_status))
        .route("/probe",                   get(legacy_probe))
        .with_state(state);

    // ── Static files (web panel) ────────────────────────────────────────────
    let app = if let Some(ref dir) = web_dir {
        // Serve the built React app; fall back to index.html for SPA routing
        let serve = ServeDir::new(&dir)
            .fallback(tower_http::services::ServeFile::new(format!("{}/index.html", dir)));
        api.fallback_service(serve)
    } else {
        api.fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                "Web panel not configured. Start the daemon with --web-dir <path/to/dist>",
            )
        })
    };

    let addr: std::net::SocketAddr = match bind_addr.parse() {
        Ok(a) => a,
        Err(e) => { error!("❌ Invalid bind address {}: {}", bind_addr, e); return; }
    };

    if web_dir.is_some() {
        info!("🌐 Web panel listening on http://{}", addr);
    }
    if let Err(e) = axum::Server::bind(&addr).serve(app.into_make_service()).await {
        error!("❌ Web server error: {}", e);
    }
}

// ─── Session helpers ─────────────────────────────────────────────────────────

fn generate_token() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ns = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:x}{:x}", ns, std::process::id())
}

fn extract_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';')
        .find_map(|part| {
            let part = part.trim();
            part.strip_prefix("session=").map(|t| t.to_string())
        })
}

fn require_auth(state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    let token = extract_session(headers)
        .ok_or((StatusCode::UNAUTHORIZED, "Not authenticated"))?;
    if state.sessions.read().contains(&token) {
        Ok(())
    } else {
        Err((StatusCode::UNAUTHORIZED, "Invalid session"))
    }
}

// ─── Auth endpoint ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LoginRequest { username: String, password: String }

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let settings = config::load_settings(&state.db_path)
        .unwrap_or_default();

    if body.username == settings.admin_username && body.password == settings.admin_password {
        let token = generate_token();
        state.sessions.write().insert(token.clone());
        let cookie = format!("session={}; Path=/; HttpOnly; SameSite=Strict", token);
        (
            StatusCode::OK,
            [(header::SET_COOKIE, cookie)],
            Json(serde_json::json!({ "ok": true })),
        ).into_response()
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(header::SET_COOKIE, String::new())],
            Json(serde_json::json!({ "error": "Invalid credentials" })),
        ).into_response()
    }
}

// ─── Route status (the JSON the web panel displays) ──────────────────────────

#[derive(Serialize)]
struct RouteStatusResponse {
    id: String,
    name: String,
    bind_address: String,
    input_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
    country_code: String,
    swap_interval_hours: u64,
    test_interval_minutes: u64,
    latency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tor_ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_checked_at: Option<String>,
    status: &'static str,
}

fn latency_to_status(lat: Duration) -> &'static str {
    if lat >= NOT_CONNECTED { "error" }
    else if lat >= Duration::from_millis(800) { "warning" }
    else { "healthy" }
}

fn latency_to_string(lat: Duration) -> String {
    if lat >= NOT_CONNECTED {
        "Connecting/Error".to_string()
    } else if lat.as_millis() > 0 {
        format!("{}ms", lat.as_millis())
    } else {
        "Pending".to_string()
    }
}

fn node_to_response(cfg: &RouteConfig, node: Option<&Arc<ActiveNode>>) -> RouteStatusResponse {
    let (lat, tor_ip, last_checked_at) = match node {
        Some(n) => (
            *n.latency.read(),
            n.tor_ip.read().clone(),
            n.last_checked_at.read().clone(),
        ),
        None => (
            NOT_CONNECTED,
            cfg.tor_ip.clone(),
            cfg.last_checked_at.clone(),
        ),
    };
    RouteStatusResponse {
        id:                   cfg.id.to_string(),
        name:                 cfg.name.clone(),
        bind_address:         cfg.bind_address.clone().unwrap_or_else(|| "127.0.0.0".to_string()),
        input_port:           cfg.input_port,
        username:             cfg.username.clone(),
        password:             cfg.password.clone(),
        country_code:         cfg.country_code.to_uppercase(),
        swap_interval_hours:  cfg.swap_interval_hours.unwrap_or(24),
        test_interval_minutes:cfg.test_interval_minutes.unwrap_or(15),
        latency:              latency_to_string(lat),
        tor_ip,
        last_checked_at,
        status:               latency_to_status(lat),
    }
}

// ─── Route CRUD handlers ─────────────────────────────────────────────────────

async fn list_routes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }

    let cfg = match config::load_from_db(&state.db_path) {
        Ok(c) => c,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    };
    let nodes = state.nodes.read();
    let list: Vec<RouteStatusResponse> = cfg.routes.iter()
        .map(|r| node_to_response(r, nodes.get(&r.id).map(|a| a)))
        .collect();
    Json(list).into_response()
}

// Body the panel sends when creating/editing a route
#[derive(Deserialize)]
struct RouteBody {
    name: String,
    bind_address: Option<String>,
    input_port: u16,
    username: Option<String>,
    password: Option<String>,
    country_code: String,
    swap_interval_hours: Option<u64>,
    test_interval_minutes: Option<u64>,
}

impl From<RouteBody> for RouteConfig {
    fn from(b: RouteBody) -> Self {
        RouteConfig {
            id: 0,
            name: b.name,
            bind_address: b.bind_address.or_else(|| Some("127.0.0.1".to_string())),
            input_port: b.input_port,
            username: b.username.filter(|s| !s.is_empty()),
            password: b.password.filter(|s| !s.is_empty()),
            country_code: b.country_code.to_lowercase(),
            swap_interval_hours: Some(b.swap_interval_hours.unwrap_or(24)),
            test_interval_minutes: Some(b.test_interval_minutes.unwrap_or(15)),
            restart_trigger: None,
            tor_ip: None,
            last_checked_at: None,
        }
    }
}

async fn create_route_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<RouteBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let route: RouteConfig = body.into();
    match config::create_route(&state.db_path, &route) {
        Ok(id) => Json(serde_json::json!({ "id": id.to_string(), "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn update_route_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
    Json(body): Json<RouteBody>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let id: i64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid ID").into_response(),
    };

    let mut route: RouteConfig = body.into();
    
    // Preserve old restart_trigger
    if let Ok(old_r) = config::get_route_by_id(&state.db_path, id) {
        route.restart_trigger = old_r.restart_trigger;
    }

    if let Err(e) = config::update_route(&state.db_path, id, &route) {
        return (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response();
    }

    Json(serde_json::json!({ "ok": true })).into_response()
}

async fn delete_route_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let id: i64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid ID").into_response(),
    };

    // Signal stop before deleting so the running process is killed cleanly
    let _ = state.restart_tx.try_send(id);

    match config::delete_route(&state.db_path, id) {
        Ok(_) => Json(serde_json::json!({ "ok": true })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ─── Restart handlers ────────────────────────────────────────────────────────

async fn restart_by_id_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id_str): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let id: i64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid ID").into_response(),
    };
    match config::get_route_by_id(&state.db_path, id) {
        Ok(mut route) => {
            route.restart_trigger = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string());
            let _ = config::update_route(&state.db_path, id, &route);
            Json(serde_json::json!({ "ok": true, "name": route.name })).into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND, "Route not found").into_response(),
    }
}

async fn restart_all_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let mut count = 0;
    if let Ok(mut cfg) = config::load_from_db(&state.db_path) {
        for route in &mut cfg.routes {
            route.restart_trigger = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string());
            let _ = config::update_route(&state.db_path, route.id, &route);
            count += 1;
        }
    }
    Json(serde_json::json!({ "ok": true, "restarted": count })).into_response()
}

// ─── Settings handlers ───────────────────────────────────────────────────────

async fn get_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    match config::load_settings(&state.db_path) {
        Ok(s) => Json(serde_json::json!({
            "web_panel_port":   s.web_panel_port,
            "web_bind_address": s.web_bind_address,
            "api_port":         s.api_port,
        })).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

async fn save_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<SettingsUpdate>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let mut settings = config::load_settings(&state.db_path).unwrap_or_default();
    if let Some(p) = update.web_panel_port   { settings.web_panel_port   = p; }
    if let Some(a) = update.web_bind_address { settings.web_bind_address = a; }
    if let Some(p) = update.api_port         { settings.api_port         = p; }
    if let Some(u) = update.admin_username   { settings.admin_username   = u; }
    if let Some(pw) = update.admin_password  { settings.admin_password   = pw; }
    match config::save_settings(&state.db_path, &settings) {
        Ok(_) => {
            // Signal run_daemon to only restart the web server (do not stop Tor routes)
            let _ = state.restart_tx.send(crate::daemon::WEB_RESTART_SIGNAL).await;
            Json(serde_json::json!({
                "ok": true,
                "restarting": true,
                "web_panel_port": settings.web_panel_port,
                "web_bind_address": settings.web_bind_address
            })).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

// ─── Legacy CLI endpoints (backward compat) ──────────────────────────────────

#[derive(Deserialize)]
struct LegacyRestartQuery { route: String }

async fn legacy_restart(
    State(state): State<AppState>,
    axum::extract::Query(q): axum::extract::Query<LegacyRestartQuery>,
) -> impl IntoResponse {
    if let Ok(cfg) = config::load_from_db(&state.db_path) {
        if let Some(mut route) = cfg.routes.into_iter().find(|r| r.name == q.route) {
            route.restart_trigger = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string());
            let _ = config::update_route(&state.db_path, route.id, &route);
            return (StatusCode::OK, format!("Restart triggered for {}\n", q.route));
        }
    }
    (StatusCode::SERVICE_UNAVAILABLE, "System busy or route not found\n".to_string())
}

async fn legacy_status(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = config::load_from_db(&state.db_path).unwrap_or(crate::config::Config { routes: vec![] });
    let nodes = state.nodes.read();
    let list: Vec<RouteStatusResponse> = cfg.routes.iter()
        .map(|r| node_to_response(r, nodes.get(&r.id).map(|a| a)))
        .collect();
    Json(list)
}

#[derive(Deserialize)]
struct ProbeQuery {
    bind: String,
    port: u16,
}

async fn legacy_probe(Query(q): Query<ProbeQuery>) -> impl IntoResponse {
    let proxy_url = format!("socks5h://{}:{}", q.bind, q.port);
    let (lat, ip) = crate::tor_process::measure_latency(&proxy_url).await;
    let lat_str = latency_to_string(lat);
    Json(serde_json::json!({ "latency": lat_str, "tor_ip": ip }))
}

async fn probe_route_handler(
    State(state): State<AppState>,
    Path(id_str): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }

    let id: i64 = match id_str.parse() {
        Ok(v) => v,
        Err(_) => return (StatusCode::BAD_REQUEST, "Invalid ID").into_response(),
    };

    let route = match config::get_route_by_id(&state.db_path, id) {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Route not found").into_response(),
    };

    let bind_address = route.bind_address.unwrap_or_else(|| "0.0.0.0".to_string());
    let proxy_url = format!("socks5h://{}:{}", bind_address, route.input_port);
    let (lat, ip) = crate::tor_process::measure_latency(&proxy_url).await;
    let lat_str = latency_to_string(lat);

    Json(serde_json::json!({ "latency": lat_str, "tor_ip": ip })).into_response()
}

async fn get_countries(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    let cache_path = std::env::temp_dir().join("tor_countries_cache.json");
    
    // Check if cache exists and is less than 24 hours old
    let mut use_cache = false;
    if let Ok(metadata) = std::fs::metadata(&cache_path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                if elapsed.as_secs() < 24 * 60 * 60 {
                    use_cache = true;
                }
            }
        }
    }
    
    if use_cache {
        if let Ok(content) = std::fs::read_to_string(&cache_path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                return Json(json).into_response();
            }
        }
    }
    
    // Fetch from API
    let url = "https://onionoo.torproject.org/details?running=true&flag=Exit";
    match reqwest::get(url).await {
        Ok(res) => {
            if let Ok(text) = res.text().await {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                    let _ = std::fs::write(&cache_path, &text);
                    return Json(json).into_response();
                }
            }
        }
        Err(_) => {}
    }
    
    // If fetch failed, try to return stale cache
    if let Ok(content) = std::fs::read_to_string(&cache_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return Json(json).into_response();
        }
    }
    
    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch countries").into_response()
}
