use dirs::config_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    transfers: u32,
    zip: bool,
    exclude: String,
    overwrite: bool,
    multicast: String,
    ip: String,
    relay: String,
    relay6: String,
    pass: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            transfers: 8,
            zip: false,
            exclude: "".to_string(),
            overwrite: false,
            multicast: "239.255.255.250".to_string(),
            ip: "".to_string(),
            relay: "".to_string(),
            relay6: "".to_string(),
            pass: "".to_string(),
        }
    }
}

fn get_config_dir() -> PathBuf {
    let mut dir = config_dir().expect("Failed to get config dir");
    dir.push("croc-gui");
    fs::create_dir_all(&dir).expect("Failed to create config dir");
    println!("Config Path:{:?}", dir);
    dir
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
