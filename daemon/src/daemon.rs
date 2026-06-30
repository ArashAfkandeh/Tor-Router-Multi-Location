use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use parking_lot::RwLock;
use tokio::sync::mpsc;
use tokio::time;

use crate::api::start_web_server;
use crate::config::{Config, RouteConfig, init_db};
use crate::router::{Slot, start_router_listener};
use crate::tor_process::spawn_route_worker;

pub const NOT_CONNECTED: Duration = Duration::from_secs(3596400);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(10);
pub const WEB_RESTART_SIGNAL: i64 = -1;

use tracing_subscriber::{EnvFilter, Registry};
use tracing_subscriber::reload::Handle;

pub type LogHandle = Handle<EnvFilter, Registry>;

lazy_static::lazy_static! {
    pub static ref APP_LOGS: Arc<RwLock<Vec<String>>> = Arc::new(RwLock::new(Vec::new()));
    pub static ref LOG_RELOAD_HANDLE: Arc<RwLock<Option<LogHandle>>> = Arc::new(RwLock::new(None));
}

pub fn update_log_level(level: &str) {
    if let Some(handle) = LOG_RELOAD_HANDLE.read().as_ref() {
        let filter_str = format!("tor_router={},hyper=info,reqwest=info,h2=info", level);
        let new_filter = EnvFilter::new(&filter_str);
        if let Err(e) = handle.reload(new_filter) {
            tracing::error!("Failed to reload log level: {}", e);
        } else {
            tracing::info!("Log level dynamically updated to: {}", level);
        }
    }
}

#[derive(Clone)]
pub struct AppLogger;

impl AppLogger {
    pub fn new() -> Self {
        Self
    }
}

impl std::io::Write for AppLogger {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let text = String::from_utf8_lossy(buf).to_string();
        // Also write to stdout
        let mut stdout = std::io::stdout();
        stdout.write_all(buf)?;
        stdout.flush()?;

        let mut logs = APP_LOGS.write();
        if logs.len() > 1000 {
            logs.remove(0);
        }
        logs.push(text);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for AppLogger {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[derive(Clone, Debug)]
pub struct NodeStatus {
    pub latency: Duration,
    pub tor_ip: Option<String>,
    pub last_checked_at: Option<String>,
}

#[derive(Debug)]
pub enum RegistryMsg {
    UpdateStatus { id: i64, latency: Duration, tor_ip: Option<String>, last_checked_at: Option<String> },
    GetAllStatus { reply: tokio::sync::oneshot::Sender<HashMap<i64, NodeStatus>> },
    Remove { id: i64 },
}

pub type RegistryTx = mpsc::Sender<RegistryMsg>;

pub fn start_registry() -> RegistryTx {
    let (tx, mut rx) = mpsc::channel::<RegistryMsg>(100);
    tokio::spawn(async move {
        let mut map: HashMap<i64, NodeStatus> = HashMap::new();
        while let Some(msg) = rx.recv().await {
            match msg {
                RegistryMsg::UpdateStatus { id, latency, tor_ip, last_checked_at } => {
                    map.insert(id, NodeStatus { latency, tor_ip, last_checked_at });
                }
                RegistryMsg::GetAllStatus { reply } => {
                    let _ = reply.send(map.clone());
                }
                RegistryMsg::Remove { id } => {
                    map.remove(&id);
                }
            }
        }
    });
    tx
}

pub async fn check_and_download_updates(assets_dir: &PathBuf) {
    let files_to_check = [
        ("geoip", "https://raw.githubusercontent.com/ArashAfkandeh/ToRouter-Multi-Location/main/assets/geoip"),
        ("geoip6", "https://raw.githubusercontent.com/ArashAfkandeh/ToRouter-Multi-Location/main/assets/geoip6"),
        ("tor-bin", "https://raw.githubusercontent.com/ArashAfkandeh/ToRouter-Multi-Location/main/assets/tor-bin"),
    ];

    let client = match reqwest::Client::builder().timeout(Duration::from_secs(60)).build() {
        Ok(c) => c,
        Err(_) => return,
    };

    let thirty_days_ago = std::time::SystemTime::now() - Duration::from_secs(30 * 24 * 60 * 60);

    for (filename, url) in &files_to_check {
        let path = assets_dir.join(filename);
        let mut needs_download = false;

        if !path.exists() {
            needs_download = true;
        } else if let Ok(metadata) = std::fs::metadata(&path) {
            if let Ok(modified) = metadata.modified() {
                if modified < thirty_days_ago {
                    needs_download = true;
                }
            }
        }

        if needs_download {
            tracing::info!("Downloading update for asset {}...", filename);
            if let Ok(response) = client.get(*url).send().await {
                if response.status().is_success() {
                    if let Ok(bytes) = response.bytes().await {
                        if std::fs::write(&path, &bytes).is_ok() {
                            tracing::info!("Successfully updated asset {}", filename);
                            #[cfg(unix)]
                            if *filename == "tor-bin" {
                                if let Ok(metadata) = std::fs::metadata(&path) {
                                    use std::os::unix::fs::PermissionsExt;
                                    let mut perms = metadata.permissions();
                                    perms.set_mode(0o755);
                                    let _ = std::fs::set_permissions(&path, perms);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

pub struct ManagedRoute {
    pub router_handle: tokio::task::JoinHandle<()>,
    pub worker_handle: tokio::task::JoinHandle<()>,
    pub slot: Arc<RwLock<Slot>>,
    pub config: Arc<RwLock<RouteConfig>>,
}

async fn stop_route(handles: ManagedRoute) {
    handles.router_handle.abort();
    handles.worker_handle.abort();
    
    // We optionally wait for them to finish
    let _ = time::timeout(SHUTDOWN_TIMEOUT, handles.router_handle).await;
    let _ = time::timeout(SHUTDOWN_TIMEOUT, handles.worker_handle).await;
}

use tracing::{info, error, debug};

pub async fn run_daemon(db_path: &str, api_bind: &str, web_dir: Option<String>) {
    let abs_db_path = match fs::canonicalize(db_path) {
        Ok(p) => p,
        Err(_) => PathBuf::from(db_path),
    };
    let abs_db_str = abs_db_path.to_str().unwrap_or(db_path).to_string();

    if let Err(e) = init_db(&abs_db_str) {
        error!("❌ Failed to init database: {}", e);
        process::exit(1);
    }

    let pid = process::id();
    let temp_dir = std::env::temp_dir();
    
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::env::current_dir().unwrap());
    
    let assets_dir = exe_dir.join("assets");
    
    if !assets_dir.exists() {
        fs::create_dir_all(&assets_dir).unwrap();
    }
    
    let tor_data_dir_base = temp_dir.join(format!("tor-router-data-{}", pid));
    fs::create_dir_all(&tor_data_dir_base).unwrap();

    let tor_bin_path = match crate::tor_process::prepare_assets(&assets_dir) {
        Ok(p) => p,
        Err(e) => {
            error!("❌ Failed to prepare Tor assets: {}", e);
            process::exit(1);
        }
    };
    
    let geoip_path = assets_dir.join("geoip");
    let geoip6_path = assets_dir.join("geoip6");

    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("🛑 Exit signal received! Cleaning up...");
        process::exit(0);
    });
    
    // Spawn background job to check for updates monthly
    let updates_assets_dir = assets_dir.clone();
    tokio::spawn(async move {
        loop {
            crate::daemon::check_and_download_updates(&updates_assets_dir).await;
            // Sleep for 30 days
            tokio::time::sleep(Duration::from_secs(30 * 24 * 60 * 60)).await;
        }
    });

    info!("✅ Daemon started (PID: {}). DB: {:?}", pid, abs_db_path);
    info!("💡 Tip: type 'tor-p' in a new terminal to open the CLI.");
    info!("Press Ctrl+C to exit.");

    let (restart_tx, mut restart_rx) = mpsc::channel::<i64>(32);
    let registry_tx = start_registry();

    let api_registry_tx = registry_tx.clone();
    let db_for_api = abs_db_str.clone();
    let mut server_handle = axum_server::Handle::<std::net::SocketAddr>::new();
    let mut web_handle: Option<tokio::task::JoinHandle<()>> = {
        let bind = api_bind.to_string();
        let tx = restart_tx.clone();
        let api_registry_tx = api_registry_tx.clone();
        let db_for_api = db_for_api.clone();
        let web_dir = web_dir.clone();
        let h = server_handle.clone();
        Some(tokio::spawn(async move {
            start_web_server(bind, tx, api_registry_tx, db_for_api, web_dir, h).await;
        }))
    };

    let active_routes: Arc<RwLock<HashMap<i64, ManagedRoute>>> = Arc::new(RwLock::new(HashMap::new()));
    
    let mut ticker = time::interval(Duration::from_secs(5));
    loop {
        tokio::select! {
            route_id = restart_rx.recv() => {
                    if let Some(id) = route_id {
                        if id == -1 {
                            info!("🔁 Web server restart requested");
                            server_handle.shutdown();
                            if let Some(h) = web_handle.take() {
                                let _ = h.await;
                            }
                            if let Ok(s) = crate::config::load_settings(&abs_db_str) {
                                // Update log level dynamically
                                crate::daemon::update_log_level(&s.log_level);
                                
                                let bind = format!("{}:{}", s.web_bind_address, s.web_panel_port);
                                let bind_for_spawn = bind.clone();
                                let tx = restart_tx.clone();
                                let api_registry_tx = registry_tx.clone();
                                let db_for_api = abs_db_str.clone();
                                let web_dir = web_dir.clone();
                                server_handle = axum_server::Handle::<std::net::SocketAddr>::new();
                                let h = server_handle.clone();
                                web_handle = Some(tokio::spawn(async move {
                                    start_web_server(bind_for_spawn, tx, api_registry_tx, db_for_api, web_dir, h).await;
                                }));
                                info!("✅ Web server respawned on {}", bind);
                            } else {
                                error!("❌ Failed to load settings for web restart");
                            }
                            continue;
                        }

                        // For UI restart command (id != -1)
                        // The user requested a manual restart of the route.
                        // We will just change restart_trigger in DB or we can just stop it here.
                        // If we stop it here, it will be respawned in next tick automatically.
                        if let Some(handles) = active_routes.write().remove(&id) {
                            let _ = registry_tx.send(RegistryMsg::Remove { id }).await;
                            info!("🔄 [Route {}] Stopping old process...", id);
                            stop_route(handles).await;
                            info!("✅ [Route {}] Stopped, will respawn on next cycle", id);
                        }
                    }
            }
            _ = ticker.tick() => {
                if let Ok(config) = crate::config::load_from_db(&abs_db_str) {
                    reload_config(
                        config,
                        active_routes.clone(),
                        registry_tx.clone(),
                        &tor_bin_path,
                        &tor_data_dir_base,
                        &geoip_path,
                        &geoip6_path,
                        &abs_db_str,
                    ).await;
                }
            }
        }
    }
}

async fn reload_config(
    config: Config,
    active_handles: Arc<RwLock<HashMap<i64, ManagedRoute>>>,
    registry_tx: RegistryTx,
    tor_bin: &PathBuf,
    tor_data_root: &PathBuf,
    geoip_path: &PathBuf,
    geoip6_path: &PathBuf,
    db_path: &str,
) {
    let mut new_routes: HashMap<i64, RouteConfig> = HashMap::new();
    for mut r in config.routes {
        if r.swap_interval_minutes.unwrap_or(0) == 0 { r.swap_interval_minutes = Some(1440); }
        if r.test_interval_minutes.unwrap_or(0) < 1 { r.test_interval_minutes = Some(15); }
        new_routes.insert(r.id, r);
    }
    
    debug!("Tick: reload_config - found {} routes in DB.", new_routes.len());

    let current_ids: Vec<i64> = active_handles.read().keys().cloned().collect();

    // 1. Delete removed routes
    for id in &current_ids {
        if !new_routes.contains_key(id) {
            if let Some(handles) = active_handles.write().remove(id) {
                let _ = registry_tx.send(RegistryMsg::Remove { id: *id }).await;
                info!("🛑 [Route {}] Deleting route...", id);
                stop_route(handles).await;
            }
        }
    }

    // 2. Add or Update routes
    for (id, new_route) in &new_routes {
        let name = &new_route.name;
        let mut handles_guard = active_handles.write();
        if let Some(managed) = handles_guard.get_mut(id) {
            let old_route = managed.config.read().clone();
            let mut worker_restarted = false;

            // Worker restart condition
            if old_route.country_code != new_route.country_code || old_route.restart_trigger != new_route.restart_trigger {
                info!("🔄 [{}] Restarting Tor Worker due to country_code or manual trigger", name);
                managed.worker_handle.abort();
                {
                    let mut s = managed.slot.write();
                    s.active = None;
                    s.draining = None;
                }
                *managed.config.write() = new_route.clone();
                
                managed.worker_handle = spawn_route_worker(
                    managed.config.clone(),
                    tor_bin.clone(),
                    tor_data_root.clone(),
                    geoip_path.clone(),
                    geoip6_path.clone(),
                    registry_tx.clone(),
                    db_path.to_string(),
                    managed.slot.clone(),
                );
                worker_restarted = true;
            }

            // Router restart condition
            if old_route.bind_address != new_route.bind_address || old_route.input_port != new_route.input_port {
                info!("🔄 [{}] Restarting Router Listener due to bind/port change", name);
                managed.router_handle.abort();
                let bind_address = new_route.bind_address.clone().unwrap_or_else(|| "0.0.0.0".to_string());
                managed.router_handle = start_router_listener(bind_address, new_route.input_port, managed.slot.clone(), managed.config.clone()).await;
            }

            // Inline update if config changed but no restarts needed
            if !worker_restarted && old_route != *new_route {
                info!("📝 [{}] Updating config parameters inline", name);
                *managed.config.write() = new_route.clone();
            }
            
        } else {
            // New route
            info!("🚀 [{}] Starting Route -> exit country {}", name, new_route.country_code.to_uppercase());
            let slot = Arc::new(RwLock::new(Slot { active: None, draining: None }));
            let config = Arc::new(RwLock::new(new_route.clone()));
            let bind_address = new_route.bind_address.clone().unwrap_or_else(|| "0.0.0.0".to_string());
            
            let router_handle = start_router_listener(bind_address, new_route.input_port, slot.clone(), config.clone()).await;
            let worker_handle = spawn_route_worker(
                config.clone(),
                tor_bin.clone(),
                tor_data_root.clone(),
                geoip_path.clone(),
                geoip6_path.clone(),
                registry_tx.clone(),
                db_path.to_string(),
                slot.clone(),
            );
            
            handles_guard.insert(*id, ManagedRoute { router_handle, worker_handle, slot, config });
        }
    }
}
