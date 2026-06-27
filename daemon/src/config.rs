use serde::{Deserialize, Serialize};
use rusqlite::{Connection, Result, params};

// ─── RouteConfig (با id برای CRUD) ───────────────────────────────────────────

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct RouteConfig {
    pub id: i64,
    pub name: String,
    pub bind_address: Option<String>,
    pub input_port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub country_code: String,
    pub swap_interval_hours: Option<u64>,
    pub test_interval_minutes: Option<u64>,
    pub restart_trigger: Option<String>,
    pub tor_ip: Option<String>,
    pub last_checked_at: Option<String>,
}

impl PartialEq for RouteConfig {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.bind_address == other.bind_address
            && self.input_port == other.input_port
            && self.username == other.username
            && self.password == other.password
            && self.country_code == other.country_code
            && self.swap_interval_hours == other.swap_interval_hours
            && self.test_interval_minutes == other.test_interval_minutes
            && self.restart_trigger == other.restart_trigger
    }
}

impl Eq for RouteConfig {}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Config {
    pub routes: Vec<RouteConfig>,
}

// ─── Settings ────────────────────────────────────────────────────────────────

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Settings {
    pub web_panel_port:   u16,
    pub web_bind_address: String,
    pub api_port:         u16,
    pub admin_username:   String,
    pub admin_password:   String,
    pub domain:           Option<String>,
    pub use_custom_cert:  bool,
    pub custom_cert_path: Option<String>,
    pub custom_key_path:  Option<String>,
    pub web_base_path:    String,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            web_panel_port:   3000,
            web_bind_address: "0.0.0.0".to_string(),
            api_port:         9090,
            admin_username:   "admin".to_string(),
            admin_password:   "admin".to_string(),
            domain:           None,
            use_custom_cert:  false,
            custom_cert_path: None,
            custom_key_path:  None,
            web_base_path:    "".to_string(),
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SettingsUpdate {
    pub web_panel_port:   Option<u16>,
    pub web_bind_address: Option<String>,
    pub api_port:         Option<u16>,
    pub admin_username:   Option<String>,
    pub admin_password:   Option<String>,
    pub domain:           Option<String>,
    pub use_custom_cert:  Option<bool>,
    pub custom_cert_path: Option<String>,
    pub custom_key_path:  Option<String>,
    pub web_base_path:    Option<String>,
}

// ─── Bootstrap schema ────────────────────────────────────────────────────────

pub fn init_db(db_path: &str) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch("
        CREATE TABLE IF NOT EXISTS routes (
            id                    INTEGER PRIMARY KEY AUTOINCREMENT,
            name                  TEXT    NOT NULL UNIQUE,
            bind_address          TEXT    NOT NULL DEFAULT '0.0.0.0',
            input_port            INTEGER NOT NULL,
            username              TEXT,
            password              TEXT,
            country_code          TEXT    NOT NULL,
            swap_interval_hours   INTEGER NOT NULL DEFAULT 24,
            test_interval_minutes INTEGER NOT NULL DEFAULT 15,
            tor_ip                TEXT,
            last_checked_at       TEXT
        );
        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
    ")?;

    // Migrate existing DBs that may lack the new columns.
    let mut stmt = conn.prepare("PRAGMA table_info(routes)")?;
    let cols: Vec<String> = stmt.query_map([], |row| row.get(1))?
        .flatten()
        .collect();

    if !cols.contains(&"tor_ip".to_string()) {
        conn.execute("ALTER TABLE routes ADD COLUMN tor_ip TEXT", [])?;
    }
    if !cols.contains(&"last_checked_at".to_string()) {
        conn.execute("ALTER TABLE routes ADD COLUMN last_checked_at TEXT", [])?;
    }
    if !cols.contains(&"restart_trigger".to_string()) {
        conn.execute("ALTER TABLE routes ADD COLUMN restart_trigger TEXT", [])?;
    }

    Ok(())
}

// ─── Route CRUD ──────────────────────────────────────────────────────────────

pub fn load_from_db(db_path: &str) -> Result<Config> {
    let conn = Connection::open(db_path)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, bind_address, input_port, username, password, country_code, swap_interval_hours, test_interval_minutes, tor_ip, last_checked_at, restart_trigger FROM routes"
    )?;

    let route_iter = stmt.query_map([], |row| {
        let username: Option<String> = row.get(4)?;
        let password: Option<String> = row.get(5)?;
        Ok(RouteConfig {
            id:                    row.get(0)?,
            name:                  row.get(1)?,
            bind_address:          Some(row.get::<_, String>(2)?),
            input_port:            row.get(3)?,
            username:              username.filter(|s| !s.is_empty()),
            password:              password.filter(|s| !s.is_empty()),
            country_code:          row.get(6)?,
            swap_interval_hours:   Some(row.get::<_, i64>(7)? as u64),
            test_interval_minutes: Some(row.get::<_, i64>(8)? as u64),
            tor_ip:                row.get(9)?,
            last_checked_at:       row.get(10)?,
            restart_trigger:       row.get(11)?,
        })
    })?;

    let mut routes = Vec::new();
    for r in route_iter.flatten() {
        routes.push(r);
    }
    Ok(Config { routes })
}

pub fn get_route_by_id(db_path: &str, id: i64) -> Result<RouteConfig> {
    let conn = Connection::open(db_path)?;
    conn.query_row(
        "SELECT id, name, bind_address, input_port, username, password,
                country_code, swap_interval_hours, test_interval_minutes, tor_ip, last_checked_at, restart_trigger
         FROM routes WHERE id=?1",
        params![id],
        |row| {
            let username: Option<String> = row.get(4)?;
            let password: Option<String> = row.get(5)?;
            Ok(RouteConfig {
                id:                    row.get(0)?,
                name:                  row.get(1)?,
                bind_address:          Some(row.get::<_, String>(2)?),
                input_port:            row.get(3)?,
                username:              username.filter(|s| !s.is_empty()),
                password:              password.filter(|s| !s.is_empty()),
                country_code:          row.get(6)?,
                swap_interval_hours:   Some(row.get::<_, i64>(7)? as u64),
                test_interval_minutes: Some(row.get::<_, i64>(8)? as u64),
                tor_ip:                row.get(9)?,
                last_checked_at:       row.get(10)?,
                restart_trigger:       row.get(11)?,
            })
        },
    )
}

pub fn create_route(db_path: &str, route: &RouteConfig) -> Result<i64> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "INSERT INTO routes
            (name, bind_address, input_port, username, password,
             country_code, swap_interval_hours, test_interval_minutes, tor_ip, last_checked_at, restart_trigger)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
        params![
            route.name,
            route.bind_address.as_deref().unwrap_or("0.0.0.0"),
            route.input_port,
            route.username,
            route.password,
            route.country_code,
            route.swap_interval_hours.unwrap_or(24) as i64,
            route.test_interval_minutes.unwrap_or(15) as i64,
            route.tor_ip,
            route.last_checked_at,
            route.restart_trigger,
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn update_route(db_path: &str, id: i64, route: &RouteConfig) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "UPDATE routes SET
            name=?1, bind_address=?2, input_port=?3, username=?4, password=?5,
            country_code=?6, swap_interval_hours=?7, test_interval_minutes=?8, restart_trigger=?9
         WHERE id=?10",
        params![
            route.name,
            route.bind_address.as_deref().unwrap_or("0.0.0.0"),
            route.input_port,
            route.username,
            route.password,
            route.country_code,
            route.swap_interval_hours.unwrap_or(24) as i64,
            route.test_interval_minutes.unwrap_or(15) as i64,
            route.restart_trigger,
            id,
        ],
    )?;
    Ok(())
}

pub fn update_route_state_by_name(db_path: &str, name: &str, tor_ip: Option<&str>, last_checked_at: Option<&str>) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "UPDATE routes SET tor_ip = ?1, last_checked_at = ?2 WHERE name = ?3",
        params![tor_ip, last_checked_at, name],
    )?;
    Ok(())
}

pub fn delete_route(db_path: &str, id: i64) -> Result<()> {
    let conn = Connection::open(db_path)?;
    conn.execute("DELETE FROM routes WHERE id=?1", params![id])?;
    Ok(())
}

// ─── Settings CRUD ───────────────────────────────────────────────────────────

pub fn load_settings(db_path: &str) -> Result<Settings> {
    let conn = Connection::open(db_path)?;
    let mut settings = Settings::default();

    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    for row in rows.flatten() {
        match row.0.as_str() {
            "web_panel_port"   => settings.web_panel_port   = row.1.parse().unwrap_or(3000),
            "web_bind_address" => settings.web_bind_address = row.1,
            "api_port"         => settings.api_port         = row.1.parse().unwrap_or(9090),
            "admin_username"   => settings.admin_username   = row.1,
            "admin_password"   => settings.admin_password   = row.1,
            "domain"           => settings.domain           = if row.1.is_empty() { None } else { Some(row.1) },
            "use_custom_cert"  => settings.use_custom_cert  = row.1 == "true",
            "custom_cert_path" => settings.custom_cert_path = if row.1.is_empty() { None } else { Some(row.1) },
            "custom_key_path"  => settings.custom_key_path  = if row.1.is_empty() { None } else { Some(row.1) },
            "web_base_path"    => settings.web_base_path    = row.1,
            _ => {}
        }
    }
    Ok(settings)
}

pub fn save_settings(db_path: &str, s: &Settings) -> Result<()> {
    let conn = Connection::open(db_path)?;
    let pairs: &[(&str, String)] = &[
        ("web_panel_port",   s.web_panel_port.to_string()),
        ("web_bind_address", s.web_bind_address.clone()),
        ("api_port",         s.api_port.to_string()),
        ("admin_username",   s.admin_username.clone()),
        ("admin_password",   s.admin_password.clone()),
        ("domain",           s.domain.clone().unwrap_or_default()),
        ("use_custom_cert",  if s.use_custom_cert { "true".to_string() } else { "false".to_string() }),
        ("custom_cert_path", s.custom_cert_path.clone().unwrap_or_default()),
        ("custom_key_path",  s.custom_key_path.clone().unwrap_or_default()),
        ("web_base_path",    s.web_base_path.clone()),
    ];
    for (k, v) in pairs {
        conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![k, v],
        )?;
    }
    Ok(())
}
