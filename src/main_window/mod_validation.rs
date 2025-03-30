use super::mod_data::*;
use crate::utils::stream::*;

use std::{collections::{HashMap, HashSet}, fs::File};

pub enum ModCheckResult {
    ModsOk(),
    ModInsecurity(String, String),
    FailedDependency(Vec<String>, Vec<String>),
    FileConflict(String, Vec<String>, Vec<String>),
    InvalidPatchNames(String, Vec<String>)
}

use zip::ZipArchive;
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

        if let Err(e_msg) = check_mod_security(&mod_zip) {
            return ModInsecurity(guid, e_msg);
        }

        if let Err(bad_patches) = check_patch_validity(&mod_zip) {
            return InvalidPatchNames(guid.clone(), bad_patches);
        }

        if let Err(mut bad_deps) = check_mod_dependencies(active_mod_files, &mod_file.metadata.depends) {
            deps_unsatisfied.append(&mut bad_deps);
            mods_blame.push(guid);
            continue;
        }

        match check_mod_conflicts(&mut checked_files, active_mod_files, mod_file, &mod_zip) {
            Err((conflict_mods, conflict_files)) => return FileConflict(guid, conflict_mods, conflict_files),
            Ok(valid_files) => {
                for file in valid_files {
                    checked_files.insert(file, &mod_file);
                }
            }
        }
    }
    
    if deps_unsatisfied.len() > 0 {
        FailedDependency(deps_unsatisfied, mods_blame)
    }
    else {
        ModsOk()
    }
}

fn check_mod_security(mod_zip: &ZipArchive<File>) -> Result<(), String> {
    for entry in mod_zip.file_names() {
        if entry.ends_with(".exe") || entry.ends_with(".dll") {
            return Err(format!("DISALLOWED FILE {}, REPORT IMMEDIATELY", entry));
        }

        if entry == "data.win" {
            return Err("data.win is not allowed to be overridden".to_string());
        }
    }
    Ok(())
}

fn check_mod_dependencies(active_mod_files: &Vec<ModFile>, mod_depends: &Vec<ModDependencyEnum>) -> Result<(), Vec<String>> {
    let mut deps_unsatisfied: Vec<String> = vec![];

    for dep in mod_depends.iter().map(|d| ModMetaData::get_dependency(d).unwrap()) {
        if dep.soft {
            continue;
        }
        if active_mod_files.iter().position(|md| md.metadata.matches_dependency(&dep)).is_none() {
            deps_unsatisfied.push(format!("{} {}", dep.guid, dep.version));
        }
    }

    if deps_unsatisfied.len() > 0 {
        Err(deps_unsatisfied)
    }
    else {
        Ok(())
    }
}

fn check_mod_conflicts(checked_files: &mut HashMap<String, &ModFile>, active_mod_files: &Vec<ModFile>, mod_file: &ModFile, mod_zip: &ZipArchive<File>) -> Result<Vec<String>, (Vec<String>, Vec<String>)> {
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
        if mod_deps.in_dependency_tree(&mod_conflict.metadata.guid) || conflict_deps.in_dependency_tree(&mod_file.metadata.guid) {
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
        Err((conflict_mods.into_iter().collect(), conflicts))
    }
    else {
        Ok(mod_zip.file_names().map(String::from).filter(|e| e != "mod.toml").collect())
    }
}

fn check_patch_validity(mod_zip: &ZipArchive<File>) -> Result<(), Vec<String>> {
    let bad_patches: Vec<String> = mod_zip.file_names().map(String::from).filter(|f| f.ends_with(".xdelta") && f != "patch.xdelta").collect();
    if bad_patches.len() > 0 {
        Err(bad_patches)
    }
    else {
        Ok(())
    }
}