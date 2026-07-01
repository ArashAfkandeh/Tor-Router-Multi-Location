use std::time::Duration;

use axum::{
    extract::{Path, State, Query},
    http::{header, HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::{info, error};

use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};

use crate::config::{self, RouteConfig, SettingsUpdate};
// web restart is signalled via the daemon's restart channel; no exec/spawn here.
use crate::daemon::{NodeStatus, RegistryMsg, NOT_CONNECTED};

use parking_lot::RwLock;
use std::sync::Arc;

// ─── Shared state ────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct AppState {
    pub restart_tx: mpsc::Sender<i64>,
    pub registry_tx: mpsc::Sender<RegistryMsg>,
    pub pool: deadpool_sqlite::Pool,
    pub shared_config: Arc<RwLock<crate::config::Config>>,
    pub shared_settings: Arc<RwLock<crate::config::Settings>>,
}

// ─── Wire up the server ──────────────────────────────────────────────────────

pub async fn start_web_server(
    bind_addr: String,
    restart_tx: mpsc::Sender<i64>,
    registry_tx: mpsc::Sender<RegistryMsg>,
    pool: deadpool_sqlite::Pool,
    shared_config: Arc<RwLock<crate::config::Config>>,
    shared_settings: Arc<RwLock<crate::config::Settings>>,
    web_dir: Option<String>,
    server_handle: axum_server::Handle<std::net::SocketAddr>,
) {
    let state = AppState {
        restart_tx,
        registry_tx,
        pool,
        shared_config,
        shared_settings: shared_settings.clone(),
    };

    // ── API routes ──────────────────────────────────────────────────────────
    // NOTE: axum matches static segments before parameterised ones, so
    // /api/routes/restart-all will win over /api/routes/:id.
    let api_routes = Router::new()
        .route("/api/login",               post(login))
        .route("/api/routes",              get(list_routes).post(create_route_handler))
        .route("/api/routes/{id}/probe",    get(probe_route_handler))
        .route("/api/routes/restart-all",  post(restart_all_handler))
        .route("/api/routes/{id}/restart",  post(restart_by_id_handler))
        .route("/api/routes/{id}",          put(update_route_handler).delete(delete_route_handler))
        .route("/api/settings",            get(get_settings_handler).put(save_settings_handler))
        .route("/api/countries",           get(get_countries))
        .route("/api/logs",                get(get_logs))
        // Legacy CLI endpoint – keep backward-compat
        .route("/restart",                 post(legacy_restart))
        .route("/status",                  get(legacy_status))
        .route("/probe",                   get(legacy_probe))
        .with_state(state.clone());

    let settings = shared_settings.read().clone();
    
    let mut base_path = settings.web_base_path.trim().trim_end_matches('/').to_string();
    if !base_path.is_empty() && !base_path.starts_with('/') {
        base_path = format!("/{}", base_path);
    }
    
    let app = if base_path.is_empty() || base_path == "/" {
        let mut app = Router::new().merge(api_routes);
        if let Some(ref dir) = web_dir {
            let serve = tower_http::services::ServeDir::new(dir).fallback(tower_http::services::ServeFile::new(format!("{}/index.html", dir)));
            app = app.fallback_service(serve);
        } else {
            app = app.fallback(|| async { (axum::http::StatusCode::NOT_FOUND, "Web panel not configured. Start the daemon with --web-dir <path/to/dist>") });
        }
        app
    } else {
        let mut nested = Router::new().merge(api_routes);
        if let Some(ref dir) = web_dir {
            let serve = tower_http::services::ServeDir::new(dir).fallback(tower_http::services::ServeFile::new(format!("{}/index.html", dir)));
            nested = nested.fallback_service(serve);
        } else {
            let not_found = axum::routing::any(|| async { (axum::http::StatusCode::NOT_FOUND, "Web panel not configured.") });
            nested = nested.fallback_service(not_found);
        }
        
        let app = Router::new().nest(&base_path, nested);
        
        app.route("/", axum::routing::get(move || {
            let bp = format!("{}/", base_path);
            async move { axum::response::Redirect::temporary(&bp) }
        }))
    };

    let addr: std::net::SocketAddr = match bind_addr.parse() {
        Ok(a) => a,
        Err(e) => { error!("❌ Invalid bind address {}: {}", bind_addr, e); return; }
    };

    if settings.use_custom_cert {
        if let (Some(cert_path), Some(key_path)) = (settings.custom_cert_path, settings.custom_key_path) {
            info!("🔒 Starting web server with Custom SSL on https://{}", addr);
            let tls_config = axum_server::tls_rustls::RustlsConfig::from_pem_file(cert_path, key_path).await;
            match tls_config {
                Ok(config) => {
                    if let Err(e) = axum_server::bind_rustls(addr, config)
                        .handle(server_handle.clone())
                        .serve(app.into_make_service())
                        .await
                    {
                        error!("❌ Custom SSL server error: {}", e);
                    }
                    return;
                }
                Err(e) => {
                    error!("❌ Failed to load custom SSL certificates: {}", e);
                    // Fall back to HTTP
                }
            }
        } else {
            error!("❌ use_custom_cert is true but cert_path or key_path is missing. Falling back to HTTP");
        }
    } else if let Some(domain) = settings.domain {
        if !domain.trim().is_empty() {
            info!("🔒 Starting web server with Auto-SSL for domain {} on https://{}", domain, addr);
            
            let acme_state = rustls_acme::AcmeConfig::new(vec![domain.trim().to_string()])
                .cache(rustls_acme::caches::DirCache::new("./acme_cache"))
                .directory_lets_encrypt(true);
            
            let mut state = acme_state.state();
            let rustls_config = state.default_rustls_config();
            let acceptor_443 = state.axum_acceptor(rustls_config.clone());
            let acceptor_panel = state.axum_acceptor(rustls_config);
            
            tokio::spawn(async move {
                use tokio_stream::StreamExt;
                loop {
                    if let Some(event) = state.next().await {
                        match event {
                            Ok(ok) => tracing::info!("acme event: {:?}", ok),
                            Err(err) => tracing::error!("acme error: {:?}", err),
                        }
                    } else {
                        break;
                    }
                }
            });

            // Spawn ACME Challenge server on port 443
            let mut addr_443 = addr;
            addr_443.set_port(443);
            let handle_443 = server_handle.clone();
            tokio::spawn(async move {
                tracing::info!("🔒 ACME TLS-ALPN-01 listening on {}", addr_443);
                let empty_app = axum::Router::new().route("/", axum::routing::get(|| async { "ACME Challenge Server" }));
                if let Err(e) = axum_server::bind(addr_443)
                    .handle(handle_443)
                    .acceptor(acceptor_443)
                    .serve(empty_app.into_make_service())
                    .await
                {
                    tracing::error!("❌ ACME port 443 error: {}", e);
                }
            });

            if let Err(e) = axum_server::bind(addr)
                .handle(server_handle.clone())
                .acceptor(acceptor_panel)
                .serve(app.into_make_service())
                .await
            {
                error!("❌ Web server SSL error: {}", e);
            }
            return;
        }
    }

    if web_dir.is_some() {
        info!("🌐 Web panel listening on http://{}", addr);
    }
    if let Err(e) = axum_server::bind(addr).handle(server_handle).serve(app.into_make_service()).await {
        error!("❌ Web server error: {}", e);
    }
}

// ─── Session helpers ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
}

lazy_static::lazy_static! {
    static ref JWT_SECRET: String = format!("{:x}{:x}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), std::process::id());
}

fn generate_token() -> String {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(7))
        .expect("valid timestamp")
        .timestamp();
        
    let claims = Claims {
        sub: "admin".to_owned(),
        exp: expiration as usize,
    };
    
    encode(&Header::default(), &claims, &EncodingKey::from_secret(JWT_SECRET.as_bytes())).unwrap()
}

fn extract_session(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';')
        .find_map(|part| {
            let part = part.trim();
            part.strip_prefix("session=").map(|t| t.to_string())
        })
}

fn require_auth(_state: &AppState, headers: &HeaderMap) -> Result<(), (StatusCode, &'static str)> {
    let token = extract_session(headers)
        .ok_or((StatusCode::UNAUTHORIZED, "Not authenticated"))?;
        
    match decode::<Claims>(&token, &DecodingKey::from_secret(JWT_SECRET.as_bytes()), &Validation::default()) {
        Ok(_) => Ok(()),
        Err(_) => Err((StatusCode::UNAUTHORIZED, "Invalid session")),
    }
}

// ─── Auth endpoint ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct LoginRequest { username: String, password: String }

async fn login(
    State(state): State<AppState>,
    Json(body): Json<LoginRequest>,
) -> impl IntoResponse {
    let settings = state.shared_settings.read().clone();

    if body.username == settings.admin_username && body.password == settings.admin_password {
        let token = generate_token();
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
struct RouteStatusResponse<'a> {
    id: String,
    name: &'a str,
    bind_address: &'a str,
    input_port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<&'a str>,
    country_code: String,
    swap_interval_minutes: u64,
    test_interval_minutes: u64,
    latency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tor_ip: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_checked_at: Option<&'a str>,
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

fn node_to_response<'a>(cfg: &'a RouteConfig, node: Option<&'a NodeStatus>) -> RouteStatusResponse<'a> {
    let (lat, tor_ip, last_checked_at) = match node {
        Some(n) => (
            n.latency,
            n.tor_ip.as_deref(),
            n.last_checked_at.as_deref(),
        ),
        None => (
            NOT_CONNECTED,
            cfg.tor_ip.as_deref(),
            cfg.last_checked_at.as_deref(),
        ),
    };
    RouteStatusResponse {
        id:                   cfg.id.to_string(),
        name:                 &cfg.name,
        bind_address:         cfg.bind_address.as_deref().unwrap_or("127.0.0.0"),
        input_port:           cfg.input_port,
        username:             cfg.username.as_deref(),
        password:             cfg.password.as_deref(),
        country_code:         cfg.country_code.to_uppercase(),
        swap_interval_minutes: cfg.swap_interval_minutes.unwrap_or(1440),
        test_interval_minutes: cfg.test_interval_minutes.unwrap_or(15),
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

    let cfg = state.shared_config.read().clone();
    
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let _ = state.registry_tx.send(RegistryMsg::GetAllStatus { reply: reply_tx }).await;
    let nodes = reply_rx.await.unwrap_or_default();
    
    let list: Vec<RouteStatusResponse> = cfg.routes.iter()
        .map(|r| node_to_response(r, nodes.get(&r.id)))
        .collect();
        
    let minified = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        minified,
    ).into_response()
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
    swap_interval_minutes: Option<u64>,
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
            swap_interval_minutes: Some(b.swap_interval_minutes.unwrap_or(1440)),
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
    let mut route: RouteConfig = body.into();
    match db_create_route(&state.pool, route.clone()).await {
        Ok(id) => {
            route.id = id;
            state.shared_config.write().routes.push(route);
            Json(serde_json::json!({ "id": id.to_string(), "ok": true })).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
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
    if let Ok(old_r) = db_get_route_by_id(&state.pool, id).await {
        route.restart_trigger = old_r.restart_trigger;
        route.tor_ip = old_r.tor_ip;
        route.last_checked_at = old_r.last_checked_at;
    }

    if let Err(e) = db_update_route(&state.pool, id, route.clone()).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, e).into_response();
    }
    
    route.id = id;
    if let Some(r) = state.shared_config.write().routes.iter_mut().find(|r| r.id == id) {
        *r = route;
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

    match db_delete_route(&state.pool, id).await {
        Ok(_) => {
            state.shared_config.write().routes.retain(|r| r.id != id);
            Json(serde_json::json!({ "ok": true })).into_response()
        },
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
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
    match db_get_route_by_id(&state.pool, id).await {
        Ok(mut route) => {
            route.restart_trigger = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string());
            let _ = db_update_route(&state.pool, id, route.clone()).await;
            if let Some(r) = state.shared_config.write().routes.iter_mut().find(|r| r.id == id) {
                r.restart_trigger = route.restart_trigger.clone();
            }
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
    
    let routes = state.shared_config.read().routes.clone();
    
    for mut route in routes {
        route.restart_trigger = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string());
        let _ = db_update_route(&state.pool, route.id, route.clone()).await;
        count += 1;
        if let Some(r) = state.shared_config.write().routes.iter_mut().find(|r| r.id == route.id) {
            r.restart_trigger = route.restart_trigger.clone();
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
    let s = state.shared_settings.read().clone();
    Json(serde_json::json!({
        "web_panel_port":   s.web_panel_port,
        "web_bind_address": s.web_bind_address,
        "api_port":         s.api_port,
        "domain":           s.domain,
        "use_custom_cert":  s.use_custom_cert,
        "custom_cert_path": s.custom_cert_path,
        "custom_key_path":  s.custom_key_path,
        "web_base_path":    s.web_base_path,
        "log_level":        s.log_level,
        "admin_username":   s.admin_username,
        "admin_password":   s.admin_password,
    })).into_response()
}

async fn save_settings_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(update): Json<SettingsUpdate>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    let mut settings = state.shared_settings.read().clone();
    if let Some(p) = update.web_panel_port   { settings.web_panel_port   = p; }
    if let Some(a) = update.web_bind_address { settings.web_bind_address = a; }
    if let Some(p) = update.api_port         { settings.api_port         = p; }
    if let Some(u) = update.admin_username   { settings.admin_username   = u; }
    if let Some(pw) = update.admin_password  { settings.admin_password   = pw; }
    if let Some(uc) = update.use_custom_cert { settings.use_custom_cert  = uc; }
    if let Some(wb) = update.web_base_path   { settings.web_base_path    = wb; }
    if let Some(ll) = update.log_level       { settings.log_level        = ll; }
    settings.custom_cert_path = update.custom_cert_path;
    settings.custom_key_path  = update.custom_key_path;
    settings.domain     = update.domain;
    match db_save_settings(&state.pool, settings.clone()).await {
        Ok(_) => {
            *state.shared_settings.write() = settings.clone();
            // Signal run_daemon to only restart the web server (do not stop Tor routes)
            let _ = state.restart_tx.send(crate::daemon::WEB_RESTART_SIGNAL).await;
            let mut response = Json(serde_json::json!({
                "ok": true,
                "restarting": true,
                "web_panel_port": settings.web_panel_port,
                "web_bind_address": settings.web_bind_address
            })).into_response();
            response.headers_mut().insert(axum::http::header::CONNECTION, axum::http::HeaderValue::from_static("close"));
            response
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
    let cfg = state.shared_config.read().clone();
    if let Some(mut route) = cfg.routes.into_iter().find(|r| r.name == q.route) {
        route.restart_trigger = Some(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().to_string());
        let _ = db_update_route(&state.pool, route.id, route.clone()).await;
        // Update shared config as well
        if let Some(r) = state.shared_config.write().routes.iter_mut().find(|r| r.id == route.id) {
            r.restart_trigger = route.restart_trigger.clone();
        }
        return (StatusCode::OK, format!("Restart triggered for {}\n", q.route));
    }
    (StatusCode::SERVICE_UNAVAILABLE, "System busy or route not found\n".to_string())
}

async fn legacy_status(State(state): State<AppState>) -> impl IntoResponse {
    let cfg = state.shared_config.read().clone();
    
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
    let _ = state.registry_tx.send(RegistryMsg::GetAllStatus { reply: reply_tx }).await;
    let nodes = reply_rx.await.unwrap_or_default();
    
    let list: Vec<RouteStatusResponse> = cfg.routes.iter()
        .map(|r| node_to_response(r, nodes.get(&r.id)))
        .collect();
        
    let minified = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
    (
        axum::http::StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        minified,
    ).into_response()
}

#[derive(Deserialize)]
struct ProbeQuery {
    bind: String,
    port: u16,
}

async fn legacy_probe(Query(q): Query<ProbeQuery>) -> impl IntoResponse {
    let mut connect_bind = q.bind.as_str();
    if connect_bind == "0.0.0.0" {
        connect_bind = "127.0.0.1";
    }
    let proxy_url = format!("socks5h://{}:{}", connect_bind, q.port);
    let (lat, ip) = crate::tor_process::measure_latency_with_proxy(&proxy_url).await;
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

    let route = match db_get_route_by_id(&state.pool, id).await {
        Ok(r) => r,
        Err(_) => return (StatusCode::NOT_FOUND, "Route not found").into_response(),
    };

    let bind_address = route.bind_address.unwrap_or_else(|| "0.0.0.0".to_string());
    let mut connect_bind = bind_address.as_str();
    if connect_bind == "0.0.0.0" {
        connect_bind = "127.0.0.1";
    }
    let proxy_url = if let (Some(u), Some(p)) = (&route.username, &route.password) {
        if !u.is_empty() && !p.is_empty() {
            format!("socks5h://{}:{}@{}:{}", u, p, connect_bind, route.input_port)
        } else {
            format!("socks5h://{}:{}", connect_bind, route.input_port)
        }
    } else {
        format!("socks5h://{}:{}", connect_bind, route.input_port)
    };
    let (lat, ip) = crate::tor_process::measure_latency_with_proxy(&proxy_url).await;
    let lat_str = latency_to_string(lat);

    Json(serde_json::json!({ "latency": lat_str, "tor_ip": ip })).into_response()
}

use std::time::Instant;

lazy_static::lazy_static! {
    static ref COUNTRIES_CACHE: tokio::sync::RwLock<Option<(Instant, String)>> = tokio::sync::RwLock::new(None);
    static ref HTTP_CLIENT_API: reqwest::Client = reqwest::Client::builder().timeout(Duration::from_secs(10)).build().unwrap();
}

async fn get_countries(
    State(_state): State<AppState>,
    _headers: HeaderMap,
) -> impl IntoResponse {
    {
        let cache = COUNTRIES_CACHE.read().await;
        if let Some((timestamp, data)) = &*cache {
            if timestamp.elapsed() < Duration::from_secs(3600) {
                return (
                    axum::http::StatusCode::OK,
                    [(axum::http::header::CONTENT_TYPE, "application/json")],
                    data.clone(),
                ).into_response();
            }
        }
    }

    let url = "https://onionoo.torproject.org/details?running=true&flag=Exit";

    #[derive(serde::Deserialize, serde::Serialize)]
    struct OnionooResponse {
        relays: Vec<OnionooRelay>,
    }
    
    #[derive(serde::Deserialize, serde::Serialize)]
    struct OnionooRelay {
        #[serde(skip_serializing_if = "Option::is_none")]
        country: Option<String>,
    }

    if let Ok(res) = HTTP_CLIENT_API.get(url).send().await {
        if let Ok(text) = res.text().await {
            if let Ok(mut parsed) = serde_json::from_str::<OnionooResponse>(&text) {
                // Keep only relays that have a country code to save space
                parsed.relays.retain(|r| r.country.is_some());
                if let Ok(minified) = serde_json::to_string(&parsed) {
                    let mut cache = COUNTRIES_CACHE.write().await;
                    *cache = Some((Instant::now(), minified.clone()));
                    return (
                        axum::http::StatusCode::OK,
                        [(axum::http::header::CONTENT_TYPE, "application/json")],
                        minified,
                    ).into_response();
                }
            }
        }
    }
    
    (StatusCode::INTERNAL_SERVER_ERROR, "Failed to fetch countries from Tor Project").into_response()
}

async fn get_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&state, &headers) { return e.into_response(); }
    
    let guard = crate::daemon::APP_LOGS.read();
    let minified = serde_json::to_string(&serde_json::json!({ "logs": &*guard })).unwrap_or_else(|_| "{}".to_string());
    
    (
        StatusCode::OK,
        [(axum::http::header::CONTENT_TYPE, "application/json")],
        minified,
    ).into_response()
}

// ─── Async DB Wrappers ───────────────────────────────────────────────────────

pub async fn db_load_settings(pool: &deadpool_sqlite::Pool) -> Result<config::Settings, String> {
    let conn = pool.get().await.map_err(|e| e.to_string())?;
    conn.interact(|c| config::load_settings_conn(c)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

async fn db_save_settings(pool: &deadpool_sqlite::Pool, settings: config::Settings) -> Result<(), String> {
    let conn = pool.get().await.map_err(|e| e.to_string())?;
    conn.interact(move |c| config::save_settings_conn(c, &settings)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

async fn db_get_route_by_id(pool: &deadpool_sqlite::Pool, id: i64) -> Result<RouteConfig, String> {
    let conn = pool.get().await.map_err(|e| e.to_string())?;
    conn.interact(move |c| config::get_route_by_id_conn(c, id)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

async fn db_create_route(pool: &deadpool_sqlite::Pool, route: RouteConfig) -> Result<i64, String> {
    let conn = pool.get().await.map_err(|e| e.to_string())?;
    conn.interact(move |c| config::create_route_conn(c, &route)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

async fn db_update_route(pool: &deadpool_sqlite::Pool, id: i64, route: RouteConfig) -> Result<(), String> {
    let conn = pool.get().await.map_err(|e| e.to_string())?;
    conn.interact(move |c| config::update_route_conn(c, id, &route)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}

async fn db_delete_route(pool: &deadpool_sqlite::Pool, id: i64) -> Result<(), String> {
    let conn = pool.get().await.map_err(|e| e.to_string())?;
    conn.interact(move |c| config::delete_route_conn(c, id)).await.map_err(|e| e.to_string())?.map_err(|e| e.to_string())
}
