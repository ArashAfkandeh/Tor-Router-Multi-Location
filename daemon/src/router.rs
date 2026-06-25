use parking_lot::RwLock;
use std::sync::Arc;
use tokio::io::copy_bidirectional;
use tokio::net::{TcpListener, TcpStream};
use tracing::{info, error};

use crate::config::RouteConfig;
use crate::socks5::{handle_socks5_auth, fake_tor_handshake};

#[derive(Clone)]
pub struct Backend {
    pub socks: String,
}

#[derive(Clone)]
pub struct Slot {
    pub active: Option<Backend>,
    pub draining: Option<Backend>,
}

pub async fn start_router_listener(
    bind_address: String,
    port: u16,
    slot: Arc<RwLock<Slot>>,
    config: Arc<RwLock<RouteConfig>>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let listener_res = TcpListener::bind((bind_address.clone(), port)).await;
        let listener = match listener_res {
            Ok(l) => l,
            Err(e) => {
                error!("❌ Router failed to bind to {}:{}: {}", bind_address, port, e);
                return;
            }
        };

        info!("✅ Router listening on {}:{}", bind_address, port);

        loop {
            if let Ok((mut client, _)) = listener.accept().await {
                let active_backend = {
                    let s = slot.read();
                    s.active.clone()
                };

                let route_config = config.read().clone();
                let expected_user = route_config.username.clone().unwrap_or_default();
                let expected_pass = route_config.password.clone().unwrap_or_default();

                if let Some(backend) = active_backend {
                    tokio::spawn(async move {
                        if !handle_socks5_auth(&mut client, &expected_user, &expected_pass).await {
                            return;
                        }

                        if let Ok(mut server) = TcpStream::connect(&backend.socks).await {
                            if !fake_tor_handshake(&mut server).await {
                                return;
                            }
                            let _ = copy_bidirectional(&mut client, &mut server).await;
                        }
                    });
                }
            }
        }
    })
}
