use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use tauri::State;

pub struct ConfigState(pub RwLock<AppConfig>);

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub transfers: Option<u32>, // 多少个port同时传输
    // pub zip: bool,              // 发送前是否先打包压缩
    // pub exclude: String,        // 排除哪些文件，以逗号分隔
    pub overwrite: bool,      // 是否自动覆盖或续传
    pub multicast: String,    // 局域网广播范围 默认：239.255.255.250
    pub ip: String,           // 本机IP,如果有
    pub local: bool,          // Force local connections
    pub relay: String,        // IP v4中继
    pub relay6: String,       // IP v6中继
    pub relay_passwd: String, // 中继密码
    pub proxy_socks5: String,
    pub proxy_http: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            transfers: Some(4),
            // zip: false,
            // exclude: "".to_string(),
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
pub fn load_config(state: State<ConfigState>) -> AppConfig {
    state.0.read().unwrap().clone()
    // let mut path = get_config_dir();
    // path.push("config.json");
    //
    // if Path::new(&path).exists() {
    //     let content = fs::read_to_string(&path).unwrap_or_default();
    //     serde_json::from_str(&content).unwrap_or_default()
    // } else {
    //     AppConfig::default()
    // }
}

#[tauri::command]
pub fn save_config(state: State<ConfigState>, cfg: AppConfig) -> Result<(), String> {
    let mut path = get_config_dir();
    path.push("config.json");

    let json = serde_json::to_string_pretty(&cfg).unwrap();
    fs::write(&path, json).unwrap();
    *state.0.write().unwrap() = cfg;
    Ok(())
}

pub fn global_args(cfg: AppConfig) -> Vec<String> {
    let mut croc_args = vec!["--yes".to_string()];
    if cfg.local {
        croc_args.push("--local".to_string());
    }
    if cfg.overwrite {
        croc_args.push("--overwrite".to_string());
    }
    if !cfg.multicast.trim().is_empty() {
        croc_args.push("--multicast".to_string());
        croc_args.push(cfg.multicast.clone());
    }
    if !cfg.ip.trim().is_empty() {
        croc_args.push("--ip".to_string());
        croc_args.push(cfg.ip.clone());
    }
    if !cfg.relay.trim().is_empty() {
        croc_args.push("--relay".to_string());
        croc_args.push(cfg.relay.clone());
    }
    if !cfg.relay6.trim().is_empty() {
        croc_args.push("--relay6".to_string());
        croc_args.push(cfg.relay6.clone());
    }
    if !cfg.relay_passwd.trim().is_empty() {
        croc_args.push("--pass".to_string());
        croc_args.push(cfg.relay_passwd.clone());
    }
    if !cfg.proxy_socks5.trim().is_empty() {
        croc_args.push("--socks5".to_string());
        croc_args.push(cfg.proxy_socks5.clone());
    }
    if !cfg.proxy_http.trim().is_empty() {
        croc_args.push("--connect".to_string());
        croc_args.push(cfg.proxy_http.clone());
    }

    croc_args
}
