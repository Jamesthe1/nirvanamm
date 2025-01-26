use std::{fs, path::PathBuf};

use directories::ProjectDirs;

pub fn get_appdata_dir(appname: &str) -> Result<PathBuf, String> {
    let pdirs = ProjectDirs::from(
        "",
        "Jamesthe1",
        appname
    ).unwrap();

    let appdata_dir = pdirs.data_dir();
    if !appdata_dir.exists() {
        if let Err(e) = fs::create_dir_all(appdata_dir) {
            return Err(format!("Could not create appdata directory: {}", e.to_string()));
        }
    }
    
    Ok(appdata_dir.to_path_buf())
}