use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DataWinConfig {
    pub data_path: String,
    pub active_mods: Vec<String>
}

// Custom default because data_path should always be set
impl Default for DataWinConfig {
    fn default() -> Self {
        let data_path = String::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\ZeroRanger\\data.win");
        let active_mods: Vec<String> = vec![];
        DataWinConfig { data_path, active_mods }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub data_win: DataWinConfig
}

impl AppConfig {
    pub const FILENAME: &str = "config.toml";

    pub fn new(cfg_path: PathBuf) -> Self {
        if cfg_path.exists() {
            Self::load(cfg_path)
        }
        else {
            let config = AppConfig { ..Default::default() };
            let _ = config.save(cfg_path);
            config
        }
    }

    pub fn load(cfg_path: PathBuf) -> Self {
        let cfg_default = AppConfig { ..Default::default() };
        match fs::read_to_string(cfg_path) {
            Err(e) => {
                eprintln!("File read error: {}", e.to_string());
                cfg_default
            },
            Ok(c) => {
                match toml::from_str::<AppConfig>(&c) {
                    Err(e) => {
                        eprintln!("Parse error: {}", e.to_string());
                        cfg_default
                    },
                    Ok(ac) => ac
                }
            }
        }
    }

    pub fn save(&self, cfg_path: PathBuf) -> Result<(), String> {
        match toml::to_string(self) {
            Err(e) => Err(format!("Serialize error: {}", e.to_string())),
            Ok(c) => {
                match fs::write(cfg_path, c) {
                    Err(e) => Err(format!("File write error: {}", e.to_string())),
                    Ok(_) => Ok(())
                }
            }
        }
    }
}