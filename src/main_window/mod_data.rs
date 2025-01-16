use semver::VersionReq;
use serde::Deserialize;
use zip::read::ZipFile;

use std::{fs, io::{Read, Write}, path::PathBuf};

use crate::utils::stream::*;
use crate::utils::xdelta3::*;

#[derive(Deserialize, Default, Clone)]
pub struct ModDependency {
    pub guid: String,
    #[serde(default)]
    pub soft: bool,
    pub version: String
}

#[derive(Deserialize, Clone)]
#[serde(untagged)]  // Lets serde know that this shouldn't look for one of the names here
pub enum ModDependencyEnum {
    ImplicitHard(String),   // This will format as GUID:version (use semver package)
    DependTable(ModDependency)
}

#[derive(Deserialize, Default, Clone)]
pub struct ModMetaData {
    pub name: String,
    pub guid: String,                       // Useful to have a display name (for end users) and a GUID (for mod developers)
    pub author: String,
    pub version: String,
    #[serde(default)]
    pub depends: Vec<ModDependencyEnum>     // Must be another mod GUID if defined
}

impl PartialEq for ModMetaData {
    fn eq(&self, other: &Self) -> bool {
        self.guid == other.guid
    }
}

impl Eq for ModMetaData {}

impl ModMetaData {
    pub fn validate_semantics(&self) -> Result<(), String> {
        if let Err(e) = semver::Version::parse(&self.version) {
            return Err(format!("Version is not cargo-like semantic: {}", e.to_string()));
        }

        for d in self.depends.iter() {
            let dep = match Self::get_dependency(d) {
                Err(e) => {
                    return if let ModDependencyEnum::ImplicitHard(s) = d {
                        Err(format!("{} ({})", e, s))
                    }
                    else {
                        Err(e)
                    };
                },
                Ok(md) => md
            };
            if let Err(e) = VersionReq::parse(&dep.version) {
                return Err(format!("Version of requirement {} is not cargo-like semantic: {}", dep.guid, e.to_string()));
            }
        }

        Ok(())
    }

    pub fn get_dependency(dep: &ModDependencyEnum) -> Result<ModDependency, String> {
        match dep {
            ModDependencyEnum::ImplicitHard(s) => {
                let (guid, version) = match s.split_once(':') {
                    None => return Err("No colon found in hard dependency string".to_string()),
                    Some((g, v)) => (g.to_string(), v.to_string())
                };
                Ok(ModDependency { guid, soft: false, version })
            },
            ModDependencyEnum::DependTable(dt) => Ok(dt.to_owned())
        }
    }

    pub fn has_dependencies(&self) -> bool {
        let deps = self.depends.clone();
        deps.len() > 0
    }

    pub fn has_dependency(&self, mod_meta: &Self) -> bool {
        let deps = self.depends.clone();
        deps.iter().find(|d| {
            let dep = Self::get_dependency(d.to_owned()).unwrap();
            mod_meta.matches_dependency(&dep)
        }).is_some()
    }

    pub fn matches_dependency(&self, dep: &ModDependency) -> bool {
        let ver = semver::Version::parse(&self.version).unwrap();
        let req = VersionReq::parse(&dep.version).unwrap();
        dep.guid == self.guid && req.matches(&ver)
    }

    /// Will try and build a dependency tree. If a dependency is not satisfied, it will return an Err with the missing GUID.
    pub fn get_dependency_tree(&self, mod_metas: &Vec<Self>) -> Result<DependencyNode, String> {
        let guid = self.guid.clone();
        let version = self.version.clone();
        if self.depends.is_empty() {
            return Ok(DependencyNode { guid, version, deps: None });
        }
        let mut deps = vec![];
        for d in self.depends.iter() {
            let dep = Self::get_dependency(d).unwrap();
            match mod_metas.iter().find(|m| m.matches_dependency(&dep)) {
                None => if !dep.soft { return Err(dep.guid.clone()) },
                Some(mod_file) => {
                    match mod_file.get_dependency_tree(mod_metas) {
                        Err(g) => return Err(g),
                        Ok(n) => deps.push(n),
                    }
                }
            }
        }
        Ok(DependencyNode { guid, version, deps: Some(deps) })
    }
}

#[derive(Clone)]
pub struct DependencyNode {
    pub guid: String,
    pub version: String,
    pub deps: Option<Vec<DependencyNode>>
}

impl DependencyNode {
    pub fn in_dependency_tree(&self, guid: &String) -> bool {
        if self.guid == *guid {
            return true;
        }
        match &self.deps {
            None => false,
            Some(d) => d.iter().position(|d| d.in_dependency_tree(guid)).is_some()
        }
    }
}

#[derive(Deserialize, Default, Clone)]
pub struct ModFile {
    pub manifest: i32,
    pub metadata: ModMetaData,
    #[serde(skip_serializing, skip_deserializing)]
    pub filepath: PathBuf
}

impl PartialEq for ModFile {
    fn eq(&self, other: &Self) -> bool {
        self.metadata == other.metadata
    }
}

impl ModFile {
    pub const SUBDIRECTORY: &str = "mods";

    pub fn new(filepath: PathBuf) -> Result<Self, String> {
        let filepath_str = filepath.to_str().unwrap();
        let mut archive = match open_archive(&filepath) {
            Ok(a) => a,
            Err(e) => return Err(e)
        };
        let mod_zip_file = match archive.by_name("mod.toml") {
            Err(_) => return Err(format!("{} does not contain a mod.toml file", filepath_str)),
            Ok(z) => z
        };
        let mod_file = match Self::parse_mod_metadata(mod_zip_file) {
            Ok(mut md) => {
                md.filepath = filepath;
                md
            },
            Err(e_msg) => return Err(format!("Failed to parse mod file in {}: {}", filepath_str, e_msg))
        };
        
        match mod_file.metadata.validate_semantics() {
            Err(e) => Err(format!("{}'s semantics check failed: {}", mod_file.metadata.guid, e)),
            Ok(_) => Ok(mod_file)
        }
    }

    pub fn extract_archive(&self, xd3: &XDelta3, game_root: &PathBuf, temp_dir: &PathBuf, replaced_files: &mut Vec<PathBuf>) -> Result<(), (String, String)> {
        let guid = self.metadata.guid.clone();
        let data_win = PathBuf::from("data.win");

        let mut archive = match open_archive(&self.filepath) {
            Err(e) => return Err((guid, e)),
            Ok(z) => z
        };
        let entries: Vec<String> = archive.file_names().map(String::from).collect();    // Drops the immutable borrow by making a vector of new strings
        for entry in entries.iter() {
            if entry == "mod.toml" {
                continue;
            }

            let is_patch = entry == "patch.xdelta";
            let entry_path = PathBuf::from(entry);
            if is_patch && !replaced_files.contains(&data_win) {
                replaced_files.push(data_win.clone());
            }
            else {
                replaced_files.push(entry_path);
            }

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

            let mut out_file = match fs::File::create(&path) {
                Err(e) => return Err((guid, format!("Extract output error: {}", e.to_string()))),
                Ok(f) => f
            };
            match archive.by_name(entry.as_str()) {
                Err(e) => return Err((guid, format!("Failed to read zip content: {}", e.to_string()))),
                Ok(mut zip_file) => {
                    // Better to stream with a buffer than to store the entire file in RAM
                    if let Err(e) = stream_from_to::<32768>(|buf| zip_file.read(buf), |buf| out_file.write_all(buf)) {
                        return Err((guid, format!("Failed to extract file {}: {}", entry, e)));
                    }
                }
            }
            drop(out_file);  // Drop must happen here or else xdelta will complain the file is still open

            if is_patch {
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
                    Err(e_msg) => return Err((guid, format!("Failed to patch due to an issue encountered by xdelta3.\n\n{}", e_msg)))
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

    pub fn get_dependency_tree(&self, mod_files: &Vec<Self>) -> Result<DependencyNode, String> {
        let mod_metas = mod_files.iter().map(|m| m.metadata.clone()).collect();
        self.metadata.get_dependency_tree(&mod_metas)
    }
}