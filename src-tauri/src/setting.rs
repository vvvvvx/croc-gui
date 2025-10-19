use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    transfers: u32,       // 多少个port同时传输
    zip: bool,            // 发送前是否先打包压缩
    exclude: String,      // 排除哪些文件，以逗号分隔
    overwrite: bool,      // 是否自动覆盖或续传
    multicast: String,    // 局域网广播范围 默认：239.255.255.250
    ip: String,           // 本机IP,如果有
    local: bool,          // Force local connections
    relay: String,        // IP v4中继
    relay6: String,       // IP v6中继
    relay_passwd: String, // 中继密码
    proxy_socks5: String,
    proxy_http: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            transfers: 8,
            zip: false,
            exclude: "".to_string(),
            overwrite: false,
            multicast: "".to_string(),
            ip: "".to_string(),
            local: false,
            relay: "".to_string(),
            relay6: "".to_string(),
            relay_passwd: "".to_string(),
            proxy_socks5: "".to_string(),
            proxy_http: "".to_string(),
        }
    }
}

fn get_config_dir() -> PathBuf {
    let mut dir = config_dir().expect("Failed to get config dir");
    dir.push("croc-gui");
    fs::create_dir_all(&dir).expect("Failed to create config dir");
    println!("Config Path:{dir:?}");
    dir
}

pub fn load_config_internal() -> AppConfig {
    let mut path = get_config_dir();
    path.push("config.json");

    if Path::new(&path).exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}
#[tauri::command]
pub fn load_config() -> AppConfig {
    let mut path = get_config_dir();
    path.push("config.json");

    if Path::new(&path).exists() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        AppConfig::default()
    }
}

#[tauri::command]
pub fn save_config(cfg: AppConfig) -> Result<(), String> {
    let mut path = get_config_dir();
    path.push("config.json");

    let json = serde_json::to_string_pretty(&cfg).unwrap();
    fs::write(&path, json).unwrap();

    Ok(())
}
