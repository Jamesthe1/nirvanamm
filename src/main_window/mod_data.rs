use serde::Deserialize;
use zip::read::ZipFile;

use std::{fs, io::{Read, Write}, path::PathBuf};

use crate::utils::stream::*;
use crate::utils::xdelta3::*;

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
        match open_archive(&filepath) {
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

    pub fn extract_archive(&self, xd3: &XDelta3, game_root: &PathBuf, temp_dir: &PathBuf, replaced_files: &mut Vec<PathBuf>) -> Result<(), (String, String)> {
        let guid = self.metadata.guid.clone();

        match open_archive(&self.filepath) {
            Err(e) => return Err((guid, e)),
            Ok(mut archive) => {
                let entries: Vec<String> = archive.file_names().map(String::from).collect();    // Drops the immutable borrow by making a vector of new strings
                for entry in entries.iter() {
                    if entry == "mod.toml" {
                        continue;
                    }
                    let is_patch = entry == "patch.xdelta";
                    let entry_path = PathBuf::from(entry);

                    let path =
                        if is_patch {
                            temp_dir.join(entry)
                        } else {
                            game_root.join(entry)
                        };
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

                    if is_patch {
                        let data_win = PathBuf::from("data.win");
                        let data_out = game_root.join(&data_win);
                        let data_in = temp_dir.join (&data_win);

                        if let Err(_) = fs::rename(&data_out, &data_in) {
                            // Rename only works if they are in the same file system, so we should catch cases that aren't like this
                            match fs::copy(&data_out, &data_in) {
                                Err(e) => return Err((guid, format!("Could not move old data.win to temp: {}", e.to_string()))),
                                Ok(_) => {let _ = fs::remove_file(&data_out);}
                            }
                        }

                        match xd3.decode(data_in, path, data_out) {
                            Ok(_) => (),
                            Err(i) => return Err((guid, format!("Failed to patch (xdelta3 exit code {})", i)))
                        }

                        if !replaced_files.contains(&data_win) {
                            replaced_files.push(data_win);
                        }
                    }
                    else {
                        replaced_files.push(entry_path);
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