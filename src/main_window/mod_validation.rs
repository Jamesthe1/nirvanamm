use super::mod_data::*;
use crate::utils::stream::*;

use std::collections::{HashMap, HashSet};

pub enum ModCheckResult {
    ModsOk(),
    ModInsecurity(String, String),
    FailedDependency(Vec<String>, Vec<String>),
    FileConflict(String, Vec<String>, Vec<String>),
    InvalidPatchNames(String, Vec<String>)
}

use ModCheckResult::*;

pub fn validate_active_mods(active_mod_files: &Vec<ModFile>) -> ModCheckResult {
    let mut deps_unsatisfied: Vec<String> = vec![];
    let mut mods_blame: Vec<String> = vec![];

    let mut checked_files: HashMap<String, &ModFile> = HashMap::new();

    for mod_file in active_mod_files.iter() {
        let guid = mod_file.metadata.guid.clone();
        let mod_zip = match open_archive(&mod_file.filepath) {
            Err(_) => continue,
            Ok(z) => z
        };

        // Security checks
        for entry in mod_zip.file_names() {
            if entry.ends_with(".exe") || entry.ends_with(".dll") {
                return ModInsecurity(guid, format!("DISALLOWED FILE {}, REPORT IMMEDIATELY", entry));
            }

            if entry == "data.win" {
                return ModInsecurity(guid, "data.win is not allowed to be overridden".to_string());
            }
        }

        // Dependency checks
        let mut failed = false;
        for dep in mod_file.metadata.depends.iter().map(|d| ModMetaData::get_dependency(d).unwrap()) {
            if dep.soft {
                continue;
            }
            if active_mod_files.iter().position(|md| md.metadata.matches_dependency(&dep)).is_none() {
                deps_unsatisfied.push(format!("{} {}", dep.guid, dep.version));
                failed = true;  // Don't care about this being set multiple times, if it's set once it will always be this state
            }
        }
        if failed {
            mods_blame.push(mod_file.metadata.guid.clone());
        }

        // Conflict checks
        let mut conflicts: Vec<String> = mod_zip.file_names().map(String::from).filter(|e| checked_files.contains_key(e)).collect();
        let mut resolved_conflicts: Vec<String> = vec![];
        let mut resolved_mods: Vec<&&ModFile> = vec![];
        for conflict_file in conflicts.iter() {
            let mod_conflict = checked_files.get(conflict_file).unwrap();
            // We can save computation time by checking if the mod was already found in the tree
            if resolved_mods.contains(&mod_conflict) {
                resolved_conflicts.push(conflict_file.clone());
            }

            let mod_deps = mod_file.get_dependency_tree(active_mod_files).unwrap();
            let conflict_deps = mod_conflict.get_dependency_tree(active_mod_files).unwrap();
            if mod_deps.in_dependency_tree(&mod_conflict.metadata.guid) || conflict_deps.in_dependency_tree(&guid) {
                resolved_conflicts.push(conflict_file.clone());
                resolved_mods.push(mod_conflict);
            }
        }
        for resolved in resolved_conflicts {
            let pos = conflicts.iter().position(|s| *s == resolved).unwrap();
            conflicts.remove(pos);
        }

        if conflicts.len() > 0 {
            // Hash sets always have unique data, so no duplicates here
            let conflict_mods: HashSet<String> = conflicts.iter().map(|f| checked_files.get(f).unwrap().metadata.guid.clone()).collect();
            return FileConflict(guid.clone(), conflict_mods.into_iter().collect(), conflicts);
        }
        
        for entry in mod_zip.file_names() {
            if entry == "mod.toml" {
                continue;
            }
            checked_files.insert(entry.to_string(), mod_file);
        }

        // Patch name checks
        let bad_patches: Vec<String> = mod_zip.file_names().map(String::from).filter(|f| f.ends_with(".xdelta") && f != "patch.xdelta").collect();
        if bad_patches.len() > 0 {
            return InvalidPatchNames(guid.clone(), bad_patches);
        }
    }
    
    if deps_unsatisfied.len() > 0 {
        FailedDependency(deps_unsatisfied, mods_blame)
    }
    else {
        ModsOk()
    }
}