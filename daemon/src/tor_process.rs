use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant as StdInstant};

use parking_lot::RwLock;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tracing::{info, warn, error};

use crate::config::RouteConfig;
use crate::daemon::NOT_CONNECTED;
use crate::router::{Backend, Slot};

pub fn extract_assets(assets_dir: &Path) -> std::io::Result<PathBuf> {
    let tor_bin_path = assets_dir.join(if cfg!(windows) { "tor.exe" } else { "tor" });
    let geoip_path   = assets_dir.join("geoip");
    let geoip6_path  = assets_dir.join("geoip6");

    std::fs::write(&tor_bin_path, crate::TOR_BINARY_DATA)?;
    std::fs::write(&geoip_path,   crate::GEOIP_DATA)?;
    std::fs::write(&geoip6_path,  crate::GEOIP6_DATA)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tor_bin_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tor_bin_path, perms)?;
    }

    Ok(tor_bin_path)
}

#[derive(serde::Deserialize)]
struct TorIpResponse {
    #[serde(rename = "IP")]
    ip: String,
}

pub async fn measure_latency(proxy_url: &str) -> (Duration, Option<String>) {
    let proxy = match reqwest::Proxy::all(proxy_url) {
        Ok(p) => p,
        Err(_) => return (NOT_CONNECTED, None),
    };
    let client = match reqwest::Client::builder()
        .proxy(proxy)
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (NOT_CONNECTED, None),
    };

    let start = StdInstant::now();
    let latency = match client.get("https://www.gstatic.com/generate_204").send().await {
        Ok(r) if r.status().is_success() => start.elapsed(),
        _ => NOT_CONNECTED,
    };

    let mut ip = match client.get("https://check.torproject.org/api/ip").send().await {
        Ok(resp) if resp.status().is_success() => resp.json::<TorIpResponse>().await.ok().map(|r| r.ip),
        _ => None,
    };
    
    if ip.is_none() {
        if let Ok(resp) = client.get("https://api.ipify.org").send().await {
            if resp.status().is_success() {
                if let Ok(text) = resp.text().await {
                    let text = text.trim().to_string();
                    if !text.is_empty() && text.len() <= 45 {
                        ip = Some(text);
                    }
                }
            }
        }
    }

    (latency, ip)
}

pub(crate) fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format!("{}", secs * 1000)
}

pub struct TorInstance {
    pub socks_addr: String,
    pub kill_tx: tokio::sync::oneshot::Sender<()>,
}

impl TorInstance {
    pub fn stop(self) {
        let _ = self.kill_tx.send(());
    }
}

pub fn get_free_port() -> Result<u16, String> {
    std::net::TcpListener::bind("127.0.0.1:0")
        .map_err(|e| e.to_string())
        .map(|l| l.local_addr().unwrap().port())
}

pub async fn start_tor_instance(
    name: &str,
    country_code: &str,
    tor_bin: PathBuf,
    tor_data_root: PathBuf,
    geoip_path: PathBuf,
    geoip6_path: PathBuf,
) -> Result<TorInstance, String> {
    let port = get_free_port()?;
    let socks_addr = format!("127.0.0.1:{}", port);

    let instance_name = format!("{}_{}", name, now_iso());
    let instance_dir = tor_data_root.join(&instance_name);
    std::fs::create_dir_all(&instance_dir).map_err(|e| e.to_string())?;

    let torrc_path = instance_dir.join("torrc");
    let mut torrc = String::new();
    torrc.push_str(&format!("SocksPort {}\n", socks_addr));
    torrc.push_str(&format!("DataDirectory {}\n", instance_dir.display()));
    torrc.push_str(&format!("GeoIPFile {}\n",     geoip_path.display()));
    torrc.push_str(&format!("GeoIPv6File {}\n",   geoip6_path.display()));
    
    let cc = country_code.trim().to_lowercase();
    if !cc.is_empty() {
        torrc.push_str(&format!("ExitNodes {{{}}}\n", cc));
        torrc.push_str("StrictNodes 1\n");
    }
    
    torrc.push_str("Log notice stdout\n");
    torrc.push_str("AvoidDiskWrites 1\n");

    std::fs::write(&torrc_path, torrc).map_err(|e| e.to_string())?;

    let mut cmd = Command::new(&tor_bin);
    cmd.arg("-f").arg(&torrc_path)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped())
       .kill_on_drop(true);

    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    
    let (bootstrap_tx, bootstrap_rx) = tokio::sync::oneshot::channel::<bool>();
    let log_name = instance_name.clone();
    
    tokio::spawn(async move {
        let mut stdout_lines = BufReader::new(stdout).lines();
        let mut stderr_lines = BufReader::new(stderr).lines();
        let mut bootstrap_tx = Some(bootstrap_tx);
        
        loop {
            tokio::select! {
                Ok(Some(line)) = stdout_lines.next_line() => {
                    if line.contains("Bootstrapped 100%") {
                        if let Some(tx) = bootstrap_tx.take() { let _ = tx.send(true); }
                    }
                    if line.contains("[warn]") || line.contains("[err]") {
                        warn!("[{}] {}", log_name, line);
                    }
                }
                Ok(Some(line)) = stderr_lines.next_line() => {
                    error!("[{}] STDERR: {}", log_name, line);
                }
                else => break,
            }
        }
        if let Some(tx) = bootstrap_tx.take() { let _ = tx.send(false); }
    });

    let bootstrapped = tokio::select! {
        r = bootstrap_rx => r.unwrap_or(false),
        _ = tokio::time::sleep(Duration::from_secs(120)) => false,
    };

    if !bootstrapped {
        let _ = child.kill().await;
        let _ = std::fs::remove_dir_all(&instance_dir);
        return Err("Bootstrap timeout".to_string());
    }

    let (kill_tx, kill_rx) = tokio::sync::oneshot::channel::<()>();
    let instance_dir_clone = instance_dir.clone();
    
    tokio::spawn(async move {
        tokio::select! {
            _ = kill_rx => { let _ = child.kill().await; }
            _ = child.wait() => {}
        }
        let _ = child.wait().await;
        tokio::time::sleep(Duration::from_secs(2)).await;
        let _ = std::fs::remove_dir_all(&instance_dir_clone);
    });

    Ok(TorInstance {
        socks_addr,
        kill_tx,
    })
}

pub fn spawn_route_worker(
    route_arc: Arc<RwLock<RouteConfig>>,
    tor_bin: PathBuf,
    tor_data_root: PathBuf,
    geoip_path: PathBuf,
    geoip6_path: PathBuf,
    global_nodes: crate::daemon::SharedNodes,
    db_path: String,
    slot: Arc<RwLock<Slot>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut active_instance: Option<TorInstance> = None;
        let mut last_swap: u64 = 0;
        
        let route_id = route_arc.read().id;

        let node_state = Arc::new(crate::daemon::ActiveNode {
            latency: Arc::new(RwLock::new(NOT_CONNECTED)),
            tor_ip: Arc::new(RwLock::new(None)),
            last_checked_at: Arc::new(RwLock::new(None)),
        });
        global_nodes.write().insert(route_id, node_state.clone());

        loop {
            let (name, swap_hours, test_minutes, country_code) = {
                let r = route_arc.read();
                (
                    r.name.clone(),
                    r.swap_interval_minutes.unwrap_or(1440) as u64,
                    r.test_interval_minutes.unwrap_or(15) as u64,
                    r.country_code.clone(),
                )
            };

            let now_time = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
            let swap_allowed = (now_time - last_swap) >= swap_hours * 3600;
            
            let mut active_lat = NOT_CONNECTED;
            if let Some(inst) = &active_instance {
                let proxy_url = format!("socks5h://{}", inst.socks_addr);
                let (lat, ip) = measure_latency(&proxy_url).await;
                active_lat = lat;
                
                let iso = now_iso();
                *node_state.latency.write() = lat;
                *node_state.tor_ip.write() = ip.clone();
                *node_state.last_checked_at.write() = Some(iso.clone());
                let _ = crate::config::update_route_state_by_name(&db_path, &name, ip.as_deref(), Some(&iso));
            }
            
            if active_instance.is_none() || swap_allowed || active_lat == NOT_CONNECTED {
                info!("🔄 [{}] Worker spawning test Tor instance...", name);
                match start_tor_instance(
                    &name,
                    &country_code,
                    tor_bin.clone(),
                    tor_data_root.clone(),
                    geoip_path.clone(),
                    geoip6_path.clone()
                ).await {
                    Ok(test_inst) => {
                        let proxy_url = format!("socks5h://{}", test_inst.socks_addr);
                        let (test_lat, test_ip) = measure_latency(&proxy_url).await;
                        
                        if active_instance.is_none() || (test_lat != NOT_CONNECTED && test_lat < active_lat) {
                            info!("✅ [{}] Swapping Router to new instance! (latency: {}ms)", name, test_lat.as_millis());
                            
                            let old_instance = active_instance.take();
                            
                            {
                                let mut s = slot.write();
                                s.draining = s.active.clone();
                                s.active = Some(Backend { socks: test_inst.socks_addr.clone() });
                            }
                            
                            active_instance = Some(test_inst);
                            last_swap = now_time;
                            
                            let iso = now_iso();
                            *node_state.latency.write() = test_lat;
                            *node_state.tor_ip.write() = test_ip.clone();
                            *node_state.last_checked_at.write() = Some(iso.clone());
                            let _ = crate::config::update_route_state_by_name(&db_path, &name, test_ip.as_deref(), Some(&iso));
                            
                            if let Some(old) = old_instance {
                                old.stop();
                            }
                        } else {
                            info!("➖ [{}] Test instance not better ({}ms >= {}ms), discarding.", name, test_lat.as_millis(), active_lat.as_millis());
                            test_inst.stop();
                        }
                    }
                    Err(e) => {
                        error!("⚠️ [{}] Failed to spawn test Tor instance: {}", name, e);
                    }
                }
            }
            
            let test_interval = std::time::Duration::from_secs(test_minutes * 60);
            tokio::time::sleep(test_interval).await;
        }
    })
}
