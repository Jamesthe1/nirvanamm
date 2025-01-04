use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct DataWinConfig {
    pub game_root: PathBuf,
    pub active_mods: Vec<String>
}

// Custom default because data_path should always be set
impl Default for DataWinConfig {
    fn default() -> Self {
        let game_root = PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\ZeroRanger");
        let active_mods: Vec<String> = vec![];
        DataWinConfig { game_root, active_mods }
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AppConfig {
    pub data_win: DataWinConfig,
    #[serde(skip_serializing, skip_deserializing)]
    pub filepath: PathBuf
}

impl AppConfig {
    pub const FILENAME: &str = "config.toml";

    pub fn new(cfg_path: PathBuf) -> Self {
        if cfg_path.exists() {
            Self::load(cfg_path)
        }
        else {
            let mut config = AppConfig { ..Default::default() };
            config.filepath = cfg_path;
            let _ = config.save();
            config
        }
    }

    pub fn load(cfg_path: PathBuf) -> Self {
        let cfg_default = AppConfig { ..Default::default() };
        let mut app_cfg = match fs::read_to_string(&cfg_path) {
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
        };
        app_cfg.filepath = cfg_path;
        app_cfg
    }

    pub fn save(&self) -> Result<(), String> {
        match toml::to_string(self) {
            Err(e) => Err(format!("Serialize error: {}", e.to_string())),
            Ok(c) => {
                match fs::write(&self.filepath, c) {
                    Err(e) => Err(format!("File write error: {}", e.to_string())),
                    Ok(_) => Ok(())
                }
            }
        }
    }
}