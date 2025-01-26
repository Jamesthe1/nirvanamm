use std::{fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::utils::files::get_appdata_dir;

#[derive(Serialize, Deserialize, Clone)]
pub struct DataWinConfig {
    pub game_root: PathBuf,
    pub active_mods: Vec<String>,
    pub replaced_files: Vec<PathBuf>
}

// Custom default because data_path should always be set
impl Default for DataWinConfig {
    fn default() -> Self {
        let game_root = PathBuf::from("C:\\Program Files (x86)\\Steam\\steamapps\\common\\ZeroRanger");
        DataWinConfig { game_root, active_mods: vec![], replaced_files: vec![] }
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
            let mut config = Self { ..Default::default() };
            config.filepath = cfg_path;
            let _ = config.save();
            config
        }
    }

    pub fn load(cfg_path: PathBuf) -> Self {
        let cfg_default = Self { ..Default::default() };
        let mut app_cfg = match fs::read_to_string(&cfg_path) {
            Err(e) => {
                eprintln!("File read error: {}", e.to_string());
                cfg_default
            },
            Ok(c) => {
                match toml::from_str::<Self>(&c) {
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

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct DirsConfig {
    pub appdata: PathBuf
}

impl DirsConfig {
    pub const FILENAME: &str = "dirs.toml";

    pub fn cfg_exists() -> bool {
        let cfg_path = PathBuf::from(Self::FILENAME);
        cfg_path.exists()
    }

    pub fn open(appname: &str) -> Result<Self, String> {
        if Self::cfg_exists() {
            Self::load()
        }
        else {
            let appdata = match get_appdata_dir(appname) {
                Err(e_msg) => return Err(e_msg),
                Ok(a) => a
            };
            Ok(Self { appdata })
        }
    }

    pub fn load() -> Result<Self, String> {
        match fs::read_to_string(PathBuf::from(Self::FILENAME)) {
            Err(e) => {
                Err(format!("File read error: {}", e.to_string()))
            },
            Ok(c) => {
                match toml::from_str::<Self>(&c) {
                    Err(e) => {
                        Err(format!("Parse error: {}", e.to_string()))
                    },
                    Ok(ac) => Ok(ac)
                }
            }
        }
    }

    pub fn get_appdata_dir_cfg(appname: &str) -> Result<PathBuf, String> {
        match Self::open(appname) {
            Err(e_msg) => {
                Err(format!("Could not open dir config: {}", e_msg))
            },
            Ok(dc) => Ok(dc.appdata)
        }
    }
}