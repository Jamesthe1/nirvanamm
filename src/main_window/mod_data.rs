use serde::Deserialize;

use std::{fs, io::Read, path::PathBuf};

#[derive(Deserialize, Default, Clone)]
pub struct ModDependency {
    pub guid: String,
    pub soft: bool
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]  // Lets serde know that this shouldn't look for one of the names here
pub enum ModDependencyEnum {
    ImplicitHard(String),
    DependTable(ModDependency)
}

#[derive(Deserialize, Default, Clone)]
pub struct ModMetaData {
    pub name: String,
    pub guid: String,                       // Useful to have a display name (for end users) and a GUID (for mod developers)
    pub author: String,
    pub version: String,
    pub depends: Option<Vec<ModDependencyEnum>> // Must be another mod GUID if defined
}

#[derive(Deserialize, Default, Clone)]
pub struct ModData {
    pub metadata: ModMetaData,
    #[serde(skip_serializing, skip_deserializing)]
    pub filepath: PathBuf
}

impl ModData {
    pub fn new(filepath: PathBuf) -> Result<Self, String> {
        let filepath_str = filepath.to_str().unwrap();
        match fs::File::open(&filepath) {
            Ok(file) => {
                match zip::ZipArchive::new (file) {
                    Ok(mut archive) => {
                        match archive.by_name("mod.toml") {
                            Err(_) => Err(format!("{} does not contain a mod.toml file", filepath_str)),
                            Ok(mod_file) => {
                                match Self::parse_mod_metadata(mod_file) {
                                    Ok(mut md) => {
                                        md.filepath = filepath;
                                        Ok(md)
                                    },
                                    Err(e_msg) => Err(format!("Failed to parse mod file in {}: {}", filepath_str, e_msg))
                                }
                            }
                        }
                    }
                    Err(e) => Err(format!("Error parsing archive {}: {}", filepath_str, e.to_string()))
                }
            }
            Err(e) => Err(format!("Error reading archive at {}: {}", filepath_str, e.to_string()))
        }
    }

    fn parse_mod_metadata(mut mod_file: zip::read::ZipFile) -> Result<Self, String> {
        let mut contents = String::new();
        match mod_file.read_to_string(&mut contents) {
            Err(e) => Err(e.to_string()),
            Ok(_) => {
                match toml::from_str::<Self>(&contents) {
                    Err(e) => Err(e.to_string()),
                    Ok(md) => Ok(md)
                }
            }
        }
    }
}