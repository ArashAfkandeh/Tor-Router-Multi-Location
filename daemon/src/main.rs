mod api;
mod cli;
mod config;
mod daemon;
mod tor_process;
mod router;

use std::env;
use std::fs;
use std::path::PathBuf;
use crate::daemon::run_daemon;
use crate::cli::run_cli;
use tracing_subscriber::EnvFilter;

// Remove include_bytes! variables
// Paths relative to daemon/src/main.rs → daemon/../../assets/

// Default API Bind — on all interfaces
const DEFAULT_API_BIND: &str = "0.0.0.0:9090";

fn setup_auto_symlink() {
    if env::consts::OS == "windows" { return; }
    if let Ok(exe_path) = env::current_exe() {
        let symlink_path = "/usr/local/bin/tor-p";
        if let Ok(linked_to) = fs::read_link(symlink_path) {
            if linked_to == exe_path { return; }
            let _ = fs::remove_file(symlink_path);
        }
        if std::os::unix::fs::symlink(&exe_path, symlink_path).is_ok() {
            println!("\n✨ Shortcut created: '\x1b[36mtor-p\x1b[0m' now works from anywhere.\n");
        }
    }
}

/// Creates db path next to binary (tor_db.sqlite)
fn db_path_next_to_exe() -> String {
    env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("tor_db.sqlite")))
        .unwrap_or_else(|| PathBuf::from("tor_db.sqlite"))
        .to_string_lossy()
        .into_owned()
}

fn print_usage(name: &str) {
    println!("\x1b[1m\x1b[36mToRouter\x1b[0m");
    println!();
    println!("  \x1b[36m{} --run\x1b[0m                     Run daemon without web panel", name);
    println!("  \x1b[36m{} --web-dir <path>\x1b[0m          Run daemon with web panel", name);
    println!("  \x1b[36m{}\x1b[0m (no arguments)           Run CLI", name);
    println!();
    println!("Database will be created next to binary: tor_db.sqlite");
    println!("API starts on {}", DEFAULT_API_BIND);
}

#[tokio::main]
async fn main() {
    setup_auto_symlink();

    let args: Vec<String> = env::args().collect();
    let db_path  = db_path_next_to_exe();

    // Load saved settings (if DB exists) to determine bind address/port.
    // Fall back to defaults when settings cannot be read.
    let settings = match config::load_settings(&db_path) {
        Ok(s) => s,
        Err(_) => config::Settings::default(),
    };

    // Use the web panel port as the single shared port for both UI and API
    // when the daemon is started with --web-dir. This makes UI+API share one bind.
    let api_bind = format!("{}:{}", settings.web_bind_address, settings.web_panel_port);
    let has_domain = !settings.domain.as_deref().unwrap_or("").trim().is_empty();
    let scheme = if settings.use_custom_cert || has_domain { "https" } else { "http" };
    
    // For localhost CLI connections, we should use 127.0.0.1 instead of 0.0.0.0
    let mut connect_addr = settings.web_bind_address.clone();
    if connect_addr == "0.0.0.0" {
        connect_addr = "127.0.0.1".to_string();
    }
    // If we have a domain and Auto-SSL is active, we should try to use the domain
    if scheme == "https" {
        connect_addr = settings.domain.clone().unwrap_or(connect_addr);
    }
    
    let mut base_path = settings.web_base_path.trim().trim_end_matches('/').to_string();
    if !base_path.is_empty() && !base_path.starts_with('/') {
        base_path = format!("/{}", base_path);
    }
    
    let api_url = format!("{}://{}:{}{}", scheme, connect_addr, settings.web_panel_port, base_path);

    // No arguments → CLI
    if args.len() == 1 {
        run_cli(&api_url).await;
        return;
    }

    let mut web_dir:  Option<String> = None;
    let mut run_mode: bool           = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--run" => {
                run_mode = true;
            }
            "--web-dir" if i + 1 < args.len() => {
                web_dir = Some(args[i + 1].clone());
                i += 1;
            }
            "--help" | "-h" => {
                print_usage(&args[0]);
                return;
            }
            unknown => {
                eprintln!("\x1b[31m⚠️  Unknown argument: {}\x1b[0m", unknown);
                print_usage(&args[0]);
                std::process::exit(1);
            }
        }
        i += 1;
    }

    if let Some(dir) = web_dir.as_ref() {
        let path = PathBuf::from(dir);
        if !path.exists() {
            eprintln!("\x1b[31m❌ Web directory not found: {}\x1b[0m", path.display());
            std::process::exit(1);
        }
        if !path.is_dir() {
            eprintln!("\x1b[31m❌ Web directory is not a directory: {}\x1b[0m", path.display());
            std::process::exit(1);
        }
        if let Ok(abs_path) = fs::canonicalize(&path) {
            web_dir = Some(abs_path.to_string_lossy().into_owned());
        }
    }

    if run_mode || web_dir.is_some() {
        // Initialize logging
        let mut log_level = env::var("RUST_LOG").unwrap_or_else(|_| settings.log_level.clone());
        if log_level.is_empty() {
            log_level = "info".to_string();
        }
        let filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new(format!("tor_router={},hyper=info,reqwest=info,h2=info", log_level)));
            
        let (filter_layer, reload_handle) = tracing_subscriber::reload::Layer::new(filter);
        
        *crate::daemon::LOG_RELOAD_HANDLE.write() = Some(reload_handle);

        let logger = crate::daemon::AppLogger::new();

        use tracing_subscriber::layer::SubscriberExt;
        use tracing_subscriber::util::SubscriberInitExt;
        use tracing_subscriber::Layer;
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_writer(logger).with_filter(filter_layer))
            .init();
            
        run_daemon(&db_path, &api_bind, web_dir).await;
    } else {
        // No valid flags given → CLI
        run_cli(&api_url).await;
    }
}
