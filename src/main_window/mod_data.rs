use serde::Deserialize;
use zip::{read::ZipFile, ZipArchive};

use std::{fs, io::{Read, Write}, path::PathBuf};

use super::stream_from_to;

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

impl PartialEq for ModMetaData {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

#[derive(Deserialize, Default, Clone)]
pub struct ModFile {
    pub manifest: i32,
    pub metadata: ModMetaData,
    #[serde(skip_serializing, skip_deserializing)]
    pub filepath: PathBuf
}

impl ModFile {
    pub const SUBDIRECTORY: &str = "mods";

    pub fn new(filepath: PathBuf) -> Result<Self, String> {
        let filepath_str = filepath.to_str().unwrap();
        match Self::open_archive(&filepath) {
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
            Err(e) => Err(e)
        }
    }

    pub fn open_archive(filepath: &PathBuf) -> Result<ZipArchive<fs::File>, String> {
        let filepath_str = filepath.to_str().unwrap();
        match fs::File::open(&filepath) {
            Err(e) => Err(format!("Error reading archive at {}: {}", filepath_str, e.to_string())),
            Ok(file) => {
                match ZipArchive::new(file) {
                    Err(e) => Err(format!("Error reading archive {}: {}", filepath_str, e.to_string())),
                    Ok(archive) => Ok(archive)
                }
            }
        }
    }

    pub fn extract_archive(&self, game_root: &PathBuf) -> Result<(), (String, String)> {
        let guid = self.metadata.guid.clone();
        match Self::open_archive(&self.filepath) {
            Err(e) => return Err((guid, e)),
            Ok(mut archive) => {
                let entries: Vec<String> = archive.file_names().map(String::from).collect();    // Drops the immutable borrow by making a vector of new strings
                for entry in entries.iter() {
                    // TODO: If it's a patch.xdelta, use the xdelta3 library and decode the patch. Write data into a temp file, then overwrite data.win
                    if entry == "mod.toml" || entry == "patch.xdelta" {
                        continue;
                    }
                    let path = game_root.join(entry);
                    let dir = path.parent().unwrap();
                    if !dir.exists() {
                        let _ = fs::create_dir_all(dir);
                    }

                    match fs::File::create(&path) {
                        Err(e) => return Err((guid, format!("Extract output error: {}", e.to_string()))),
                        Ok(mut out) => {
                            match archive.by_name(entry.as_str()) {
                                Err(e) => return Err((guid, format!("Failed to read zip content: {}", e.to_string()))),
                                Ok(mut zip_file) => {
                                    // Better to stream with a buffer than to store the entire file in RAM
                                    stream_from_to::<32768>(|buf| zip_file.read(buf), |buf| out.write(buf));
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn parse_mod_metadata(mut mod_file: ZipFile) -> Result<Self, String> {
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

    pub fn has_dependencies(&self) -> bool {
        let deps = self.metadata.depends.clone();
        deps.is_some_and(|d| d.len() > 0)
    }

    pub fn has_dependency(&self, mod_file: &Self) -> bool {
        let deps = self.metadata.depends.clone().unwrap();
        deps.iter().find(|d| {
            let guid = match d {
                ModDependencyEnum::ImplicitHard(guid) => guid,
                ModDependencyEnum::DependTable(md) => &md.guid
            };
            *guid == mod_file.metadata.guid
        }).is_some()
    }
}

impl PartialEq for ModFile {
    fn eq(&self, other: &Self) -> bool {
        self.metadata == other.metadata
    }
}