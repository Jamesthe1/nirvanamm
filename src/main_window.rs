mod mod_data;
use mod_data::*;

mod config;
use config::*;

use crate::utils::{stream::*, xdelta3::XDelta3};

use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

mod asref_winctrl;
use asref_winctrl::*;

use std::{borrow::Borrow, cell::RefCell, collections::{HashMap, HashSet}, fs, io::{Read, Write}, path::PathBuf, process::Command, thread};

// Prelude automatically imports necessary traits
use winsafe::{co::{BS, SS, SW, WS, WS_EX}, gui, prelude::*};
use directories::{BaseDirs, ProjectDirs};

#[derive(Clone)]
struct WindowMenu {
    title:          String,
    control:        WindowControlWrapper,
    labels:         Vec<gui::Label>,
    buttons:        Vec<gui::Button>,
    edits:          Vec<gui::Edit>,
    mods_view:      Option<gui::ListView<ModFile>>  // Each item will contain the filename associated
}

impl WindowMenu {
    fn new(
            parent: &impl GuiParent,
            title: String,
            control_opts: gui::WindowControlOpts,
            label_opts: Vec<gui::LabelOpts>,
            button_opts: Vec<gui::ButtonOpts>,
            edit_opts: Vec<gui::EditOpts>,
            list_view_opts: Option<gui::ListViewOpts>
        ) -> Self {
        let control = gui::WindowControl::new(parent, control_opts);
        let control = WindowControlWrapper::new(control);
        let labels: Vec<gui::Label> = label_opts.into_iter().map(|o| gui::Label::new(control.as_ref(), o)).collect();
        let buttons: Vec<gui::Button> = button_opts.into_iter().map(|o| gui::Button::new(control.as_ref(), o)).collect();
        let edits: Vec<gui::Edit> = edit_opts.into_iter().map(|o| gui::Edit::new(control.as_ref(), o)).collect();
        let mods_view = match list_view_opts {
            None => None,
            Some(o) => Some(gui::ListView::new(control.as_ref(), o))
        };
        Self { title, control, labels, buttons, edits, mods_view }
    }
}

#[derive(Clone)]
struct PopupWindow {
    control:    gui::WindowControl,
    label:      gui::Label,
    buttons:    Vec<gui::Button>
}

impl PopupWindow {
    fn new(
        parent: &impl GuiParent,
        control_opts: gui::WindowControlOpts,
        label_opts: gui::LabelOpts,
        button_opts: Vec<gui::ButtonOpts>
    ) -> Self {
        let control = gui::WindowControl::new(parent, control_opts);
        let label = gui::Label::new(&control, label_opts);
        let buttons: Vec<gui::Button> = button_opts.into_iter().map(|o| gui::Button::new(&control, o)).collect();
        Self { control, label, buttons }
    }
}

#[derive(Clone)]
pub struct MyWindow {
    pub wnd:    gui::WindowMain,
    tabs:       gui::Tab,
    menus:      Vec<WindowMenu>,
    popup:      PopupWindow
}

unsafe impl Send for MyWindow {}

impl MyWindow {
    const APPNAME: &str = "NirvanaMM";
    const BUFSIZE: usize = 524228;
    const POPUP_SZ: (u32, u32) = (600, 200);

    pub fn new() -> Self {
        let wnd = gui::WindowMain::new(
            gui::WindowMainOpts {
                title: format!("{} Control Panel", Self::APPNAME),
                size: (1024, 768),
                style: WS::CAPTION | WS::SYSMENU | WS::CLIPCHILDREN | WS::BORDER | WS::VISIBLE | WS::SIZEBOX | WS::MINIMIZEBOX | WS::MAXIMIZEBOX,
                ..Default::default()    // Makes the rest of the fields default
            }
        );

        let control_opts =
            gui::WindowControlOpts {
                position: (212, 334),
                size: Self::POPUP_SZ,
                style: WS::CHILD | WS::CLIPSIBLINGS | WS::DLGFRAME,
                ..Default::default()
            };
        let label_opts =
            gui::LabelOpts {
                text: "Placeholder".to_string(),
                position: (10, 10),
                size: (Self::POPUP_SZ.0 - 20, Self::POPUP_SZ.1 - 60),
                ..Default::default()
            };
        let button_opts = vec! {
            gui::ButtonOpts {
                text: "&Ok".to_string(),
                position: ((Self::POPUP_SZ.0 - 70).try_into().unwrap(), (Self::POPUP_SZ.1 - 40).try_into().unwrap()),
                width: 60,
                height: 30,
                button_style: BS::CENTER | BS::PUSHBUTTON,
                ..Default::default()
            }
        };
        let popup = PopupWindow::new(&wnd, control_opts, label_opts, button_opts);

        let mut menus = vec![];

        let control_opts =
            gui::WindowControlOpts {
                position: (0, 20),
                size: (1024, 748),
                style: WS::CHILD | WS::CLIPSIBLINGS,
                ex_style: WS_EX::LEFT | WS_EX::CONTROLPARENT,
                ..Default::default()
            };
        let label_opts = vec! {
            gui::LabelOpts {
                text: Self::APPNAME.to_string(),
                position: (20, 20),
                size: (984, 20),
                label_style: SS::CENTER,
                ..Default::default()
            },
            gui::LabelOpts {
                text: "Click on the mod you wish to apply (shift-click for more than one), then click \"Patch\" (or press Alt-P)".to_string(),
                position: (20, 50),
                size: (984, 20),
                ..Default::default()
            }
        };
        let button_opts = vec! {
            gui::ButtonOpts {
                text: "&Refresh".to_string(),
                position: (794, 80),
                width: 40,
                height: 40,
                button_style: BS::CENTER | BS::PUSHBUTTON,  // Use ICON flag, set icon somehow
                ..Default::default()
            },
            gui::ButtonOpts {
                text: "&Patch".to_string(),
                position: (794, 688),
                width: 200,
                height: 40,
                button_style: BS::CENTER | BS::PUSHBUTTON,
                ..Default::default()
            },
            gui::ButtonOpts {
                text: "&Mods".to_string(),
                position: (844, 80),
                width: 40,
                height: 40,
                button_style: BS::CENTER | BS::PUSHBUTTON,  // Use ICON flag, set icon somehow
                ..Default::default()
            },
            gui::ButtonOpts {
                text: "&Reset".to_string(),
                position: (794, 638),
                width: 200,
                height: 40,
                button_style: BS::CENTER | BS::PUSHBUTTON,
                ..Default::default()
            }
        };
        let edit_opts = vec![];
        let list_view_opts =
            gui::ListViewOpts {
                position: (20, 80),
                size: (764, 648),
                columns: vec! {
                    ("Name".to_string(), 200),
                    ("GUID".to_string(), 200),
                    ("Version".to_string(), 100),
                    ("Author".to_string(), 150),
                    ("Depends on".to_string(), 400)
                },
                ..Default::default()
            };
        let list_view_opts = Some(list_view_opts);
        let title = "Mods".to_string();
        menus.push(WindowMenu::new(&wnd, title, control_opts, label_opts, button_opts, edit_opts, list_view_opts));

        let control_opts =
            gui::WindowControlOpts {
                position: (0, 20),
                size: (1024, 748),
                style: WS::CHILD | WS::CLIPSIBLINGS,
                ex_style: WS_EX::LEFT | WS_EX::CONTROLPARENT,
                ..Default::default()
            };
        let label_opts = vec! {
            gui::LabelOpts {
                text: Self::APPNAME.to_string(),
                position: (20, 20),
                size: (984, 20),
                label_style: SS::CENTER,
                ..Default::default()
            },
            gui::LabelOpts {
                text: "Game directory:".to_string(),
                position: (20, 50),
                size: (497, 20),
                ..Default::default()
            }
        };
        let button_opts = vec! {
            gui::ButtonOpts {
                text: "&Save".to_string(),
                position: (794, 688),
                width: 200,
                height: 40,
                button_style: BS::CENTER | BS::PUSHBUTTON,
                ..Default::default()
            }
        };
        let edit_opts = vec! {
            gui::EditOpts {
                text: String::new(),
                position: (497, 50),
                width: 507,
                height: 20,
                ..Default::default()
            }
        };
        let list_view_opts = None;
        let title = "Options".to_string();
        menus.push(WindowMenu::new(&wnd, title, control_opts, label_opts, button_opts, edit_opts, list_view_opts));

        let tabs = gui::Tab::new(
            &wnd,
            gui::TabOpts {
                position: (0, 0),
                size: (1024, 768),
                items: menus.iter().map(|wm| {
                    let wc: Box<dyn AsRef<gui::WindowControl>> = Box::new(wm.control.clone());
                    (wm.title.clone(), wc)
                }).collect(),
                ..Default::default()
            }
        );

        let new_self = Self { wnd, tabs, menus, popup };
        new_self.set_btn_events();      // Events can only be set before `run_main` is executed
        new_self.set_window_ready();    // Functions such as `text()` or `items()` will fail if the window hasn't spawned yet (done in run_main), so modify them in the window ready event
        new_self
    }

    fn get_appdata_dir() -> PathBuf {
        let pdirs = ProjectDirs::from(
            "",
            "Jamesthe1",
            Self::APPNAME
        ).unwrap();

        let appdata_dir = pdirs.data_dir();
        if !appdata_dir.exists() {
            if let Err(e) = fs::create_dir_all(appdata_dir) {
                panic!("Could not create appdata directory: {}", e.to_string());
            }
        }
        appdata_dir.to_path_buf()
    }

    fn get_appcfg() -> AppConfig {
        let cfg_path = Self::get_appdata_dir().join(AppConfig::FILENAME);
        AppConfig::new(cfg_path)
    }

    fn get_all_mod_paths(appdata_dir: PathBuf) -> Vec<PathBuf> {
        let mods_dir = appdata_dir.join(ModFile::SUBDIRECTORY);
        if !mods_dir.exists() {
            match fs::create_dir(&mods_dir) {
                Err(e) => panic!("Could not create mods directory in appdata: {}", e.to_string()),
                Ok(_) => ()
            }
        }

        let mut paths: Vec<PathBuf> = vec![];
        for entry in fs::read_dir(mods_dir).unwrap() {
            if entry.is_err() {
                eprintln!("A directory entry could not be read");
                continue;
            }
            
            let path = entry.unwrap().path();
            if !path.is_file() {
                continue;
            }

            match path.extension() {
                Some(ext) if ext == "zip" => paths.push(path),
                _ => continue
            }
        }

        paths
    }

    fn fill_main_view(main_view: &gui::ListView<ModFile>, config: &AppConfig) {
        let items = main_view.items();
        if items.count() > 0 {
            items.delete_all();
        }

        let filepaths = Self::get_all_mod_paths(Self::get_appdata_dir());
        for filepath in filepaths.iter() {
            // Making a clone of the filepath so it can exist within ModData
            let mod_file = match ModFile::new(filepath.to_owned()) {
                Err(e_msg) => {
                    eprintln!("{}", e_msg);
                    continue;
                },
                Ok(mf) => mf
            };
            let meta = mod_file.metadata.clone();
            let selected = config.data_win.active_mods.contains(&meta.guid);

            let mut hard_mods: Vec<String> = vec![];
            let mut soft_mods: Vec<String> = vec![];
            for d in meta.depends.iter() {
                match d {
                    ModDependencyEnum::ImplicitHard(guid) => {
                        hard_mods.push(guid.to_owned());
                    }
                    ModDependencyEnum::DependTable(md) => {
                        let guid = md.guid.clone();
                        if md.soft {
                            soft_mods.push(format!("[{}]", guid));
                        }
                        else {
                            hard_mods.push(guid);
                        }
                    }
                }
            }

            // Done this way because hard dependencies must go first
            let sep = ", ".to_string();
            let mut depend_str = hard_mods.join(&sep);
            if hard_mods.len() > 0 && soft_mods.len() > 0 {
                depend_str.push_str(&sep);
            }
            depend_str.push_str(&soft_mods.join(&sep));

            items.add(
                &[
                    meta.name,
                    meta.guid,
                    meta.version,
                    meta.author,
                    depend_str
                ],
                None,
                mod_file
            ).select(selected);
        }
    }

    fn fill_options_menu(menu: &WindowMenu, config: &AppConfig) {
        menu.edits[0].set_text(config.data_win.game_root.to_str().unwrap());
    }

    fn prepare_origin(&self, origin_path: PathBuf, game_root: PathBuf) -> Result<(), String> {
        let foptions = SimpleFileOptions::default();

        let mut origin_zip = match fs::File::create(origin_path) {
            Err(e) => return Err(e.to_string()),
            Ok(f) => ZipWriter::new(f)
        };
        for entry_rslt in WalkDir::new(&game_root) {
            // If unwrapped without a definition to hold it, it would be dropped and the compiler recognizes that. So we must do it this way.
            if let Ok(entry) = entry_rslt {
                let path = entry.path();
                let rel_path = path.strip_prefix(&game_root).unwrap();

                if path.is_dir() {
                    let _ = origin_zip.add_directory_from_path(rel_path, foptions);
                }
                else if path.is_file() {
                    let _ = origin_zip.start_file_from_path(rel_path, foptions);
                    if let Ok(mut f) = fs::File::open(path) {
                        if let Err(e) = stream_from_to::<{Self::BUFSIZE}>(|buf| f.read(buf), |buf| origin_zip.write_all(buf)) {
                            return Err(format!("Failed to backup file {}: {}", rel_path.to_str().unwrap(), e));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn reset_to_origin(config: &mut AppConfig) -> Result<(), String> {
        let origin_path = Self::get_appdata_dir().join("origin.zip");
        let mut origin_zip = match open_archive(&origin_path) {
            Err(e) => return Err(format!("Failed to open origin.zip: {}", e.to_string())),
            Ok(z) => z
        };
        for entry in config.data_win.replaced_files.iter() {
            let out_path = config.data_win.game_root.join(entry);
            let mut in_file = match origin_zip.by_name(entry.to_str().unwrap()) {
                Err(_) => {
                    let _ = fs::remove_file(out_path);
                    continue;
                },
                Ok(z) => z
            };
            match fs::File::create(out_path) {
                Err(e) => return Err(format!("Failed to extract origin file {}: {}", entry.to_str().unwrap(), e.to_string())),
                Ok(mut out_file) => {
                    if let Err(e) = stream_from_to::<{Self::BUFSIZE}>(|buf| in_file.read(buf), |buf| out_file.write_all(buf)) {
                        return Err(format!("Failed to reset file {}: {}", entry.to_str().unwrap(), e));
                    }
                }
            }
        }

        // Cleanup on leftover, empty folders (unless they're a part of the .zip)
        for entry in WalkDir::new(&config.data_win.game_root) {
            if entry.is_err() {
                continue;
            }

            let entry = entry.unwrap();
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            if fs::read_dir(path).unwrap().count() > 0 {
                continue;
            }
            let pathname = format!("{}/", path.strip_prefix(&config.data_win.game_root).unwrap().to_str().unwrap());
            if let Ok(_) = origin_zip.by_name(&pathname) {
                continue;
            }

            let _ = fs::remove_dir(path);
        }

        config.data_win.replaced_files.clear();
        Ok(())
    }

    fn purge_to_origin(config: &mut AppConfig) -> Result<(), String> {
        let origin_path = Self::get_appdata_dir().join("origin.zip");
        if !origin_path.exists() {
            return Err("Origin not initialized".to_string());
        }

        if let Err(e) = Self::reset_to_origin(config) {
            return Err(e);
        }
        
        config.data_win.active_mods.clear();
        config.data_win.replaced_files.clear();
        if let Err(e) = config.save() {
            return Err(format!("Error saving config: {}", e));
        }

        let _ = fs::remove_file(origin_path);

        Ok(())
    }

    fn get_current_menu(&self) -> &WindowMenu {
        let index: usize = self.tabs.items().selected().unwrap().index().try_into().unwrap();
        self.menus.get(index).unwrap()
    }

    fn set_popup_state(&self, state: bool) {
        self.tabs.hwnd().EnableWindow(!state);
        self.get_current_menu().control.as_ref().hwnd().EnableWindow(!state);
        let _ = self.popup.control.hwnd().ShowWindow(if state {
            SW::SHOW
        }
        else {
            SW::HIDE
        });
    }

    fn show_popup(&self, text: String) {
        self.popup.label.set_text(text.as_str());
        self.set_popup_state(true);
    }

    fn show_popup_result<T, U, Ft, Fu>(&self, result: Result<T, U>, ok_text: Ft, err_text: Fu)
        where
            Ft: Fn(T) -> String,
            Fu: Fn(U) -> String
    {
        self.show_popup(match result {
            Err(e) => err_text(e),
            Ok(o) => ok_text(o)
        });
    }

    fn show_popup_option<T, Ft, F_>(&self, option: Option<T>, some_text: Ft, none_text: F_)
        where
            Ft: Fn(T) -> String,
            F_: Fn() -> String
    {
        self.show_popup(match option {
            Some(t) => some_text(t),
            None => none_text()
        });
    }

    fn hide_popup(&self) {
        self.set_popup_state(false);
    }

    fn set_popup_button_state(&self, state: bool) {
        self.popup.buttons[0].hwnd().ShowWindow(if state {
            SW::SHOW
        }
        else {
            SW::HIDE
        });
    }

    fn use_selected_data_noprep(&self, mut config: AppConfig) {
        let active_mods = &mut config.data_win.active_mods;
        let mut active_mod_files: Vec<ModFile> = vec![];
        if active_mods.len() > 0 {
            active_mods.clear();
        }

        let mods_view = self.menus[0].mods_view.as_ref();
        for it in mods_view.unwrap().items().iter_selected() {
            if let Some(rc_mf) = it.data() {
                let ref_mod_file: &RefCell<ModFile> = rc_mf.borrow();
                let mod_file = ref_mod_file.borrow();
                active_mods.push(mod_file.metadata.guid.to_owned());
                active_mod_files.push(mod_file.clone());
            };
        }

        if let Err((deps_unsatisfied, mods_blame)) = Self::validate_mod_selection(&active_mod_files) {
            let deps_str = deps_unsatisfied.join(", ");
            let blame_str = mods_blame.join(", ");
            self.show_popup(format!("Missing dependencies: {}\nRequired by: {}", deps_str, blame_str));
            return;
        }

        if let Err((guid, mod_conflicts, file_conflicts)) = Self::check_file_conflicts(&active_mod_files) {
            let files_str = file_conflicts.join(", ");
            let mods_str = mod_conflicts.join(", ");
            self.show_popup(format!("Mod {} is incompatible with {}\nConflicting files: {}", guid, mods_str, files_str));
            return;
        }

        if let Err((guid, e_msg)) = Self::apply_mod_files(&mut config, active_mod_files) {
            self.show_popup_option(guid,
                |g| format!("Failed to apply mod {}\nReason: {}", g, e_msg),
                || format!("Failed to apply mods: {}", e_msg)
            );
            return;
        }

        self.show_popup_result(
            config.save(),
            |_| "Patches succeeded".to_string(),
            |e| format!("Patches succeeded\nError saving config: {}", e)
        );
    }

    fn use_selected_data(&self, config: AppConfig) {
        let origin_path = Self::get_appdata_dir().join("origin.zip");
        let game_root_clone = config.data_win.game_root.clone();
        let self_clone = self.clone();
        if !origin_path.exists() {
            self.set_popup_button_state(false);
            self.show_popup("Preparing origin (this may take a while...)".to_string());
            // THIS IS NOT THREAD SAFE!!! But this is a structural problem with GDI, as we can't make the code contain mutexes without making our own implementation.
            // So, too bad.
            thread::spawn(move || {
                if let Err(e) = self_clone.prepare_origin(origin_path, game_root_clone) {
                    self_clone.show_popup(format!("Could not prepare origin: {}", e));
                }
                else {
                    self_clone.use_selected_data_noprep(config);
                }
                self_clone.set_popup_button_state(true);
            });
        }
        else {
            self.use_selected_data_noprep(config);
        }
    }

    fn validate_mod_selection(active_mod_files: &Vec<ModFile>) -> Result<(), (Vec<String>, Vec<String>)> {
        let mut deps_unsatisfied: Vec<String> = vec![];
        let mut mods_blame: Vec<String> = vec![];
        for mod_file in active_mod_files.iter() {
            if !mod_file.metadata.has_dependencies() {
                continue;
            }
            
            let mut failed = false;
            for dep in mod_file.metadata.depends.iter() {
                let hard_guid = match dep {
                    ModDependencyEnum::ImplicitHard(guid) => guid,
                    ModDependencyEnum::DependTable(md) => {
                        if !md.soft {
                            &md.guid
                        }
                        else {
                            continue;   // Shouldn't have to care about soft dependencies
                        }
                    }
                };
                if active_mod_files.iter().position(|md| md.metadata.guid == *hard_guid).is_none() {
                    deps_unsatisfied.push(hard_guid.clone());
                    failed = true;
                }
            }
            if failed {
                mods_blame.push(mod_file.metadata.guid.clone());
            }
        }

        if deps_unsatisfied.len() > 0 {
            Err((deps_unsatisfied, mods_blame))
        }
        else {
            Ok(())
        }
    }

    fn check_file_conflicts(active_mod_files: &Vec<ModFile>) -> Result<(), (String, Vec<String>, Vec<String>)> {
        let mut files: HashMap<String, &ModFile> = HashMap::new();

        for mod_file in active_mod_files.iter() {
            let guid = &mod_file.metadata.guid;
            let mod_zip = match open_archive(&mod_file.filepath) {
                Err(_) => continue,
                Ok(z) => z
            };
            
            let mut conflicts: Vec<String> = mod_zip.file_names().map(String::from).filter(|e| files.contains_key(e)).collect();
            let mut resolved_conflicts: Vec<String> = vec![];
            let mut resolved_mods: Vec<&&ModFile> = vec![];
            for conflict_file in conflicts.iter() {
                let mod_conflict = files.get(conflict_file).unwrap();
                // We can save computation time by checking if the mod was already found in the tree
                if resolved_mods.contains(&mod_conflict) {
                    resolved_conflicts.push(conflict_file.clone());
                }

                let mod_deps = mod_file.get_dependency_tree(active_mod_files).unwrap();
                let conflict_deps = mod_conflict.get_dependency_tree(active_mod_files).unwrap();
                if mod_deps.in_dependency_tree(&mod_conflict.metadata.guid) || conflict_deps.in_dependency_tree(guid) {
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
                let conflict_mods: HashSet<String> = conflicts.iter().map(|f| files.get(f).unwrap().metadata.guid.clone()).collect();
                return Err((guid.clone(), conflict_mods.into_iter().collect(), conflicts));
            }
            
            for entry in mod_zip.file_names() {
                if entry == "mod.toml" || entry == "patch.xdelta" {
                    continue;
                }
                files.insert(entry.to_string(), mod_file);
            }
        }
        Ok(())
    }

    fn apply_mod_files(config: &mut AppConfig, active_mod_files: Vec<ModFile>) -> Result<(), (Option<String>, String)> {
        let xd3: XDelta3;
        match XDelta3::new() {
            Err(e) => return Err((None, format!("Issue with xdelta3 library: {}", e.to_string()))),
            Ok(x) => xd3 = x
        }

        if let Err(e) = Self::reset_to_origin(config) {
            return Err((None, format!("Failed to reset origin: {}", e.to_string())));
        }
        
        let mut chain: Vec<&ModFile> = vec![];
        for mod_file in active_mod_files.iter() {
            // Init
            if chain.len() == 0 {
                chain.push(mod_file);
                continue;
            }

            // Depended upon by anything in the chain, should go to first hit
            let chain_pos = chain.iter().position(|m| {
                if !m.metadata.has_dependencies() {
                    false
                }
                else {
                    m.metadata.has_dependency(&mod_file.metadata)
                }
            });
            if chain_pos.is_some() {
                chain.insert(chain_pos.unwrap(), mod_file);
                continue;
            }
            
            // Depends upon nothing, should be among the first
            if !mod_file.metadata.has_dependencies() {
                chain.insert(0, mod_file);
                continue;
            }

            let dep_pos = chain.iter()
                .filter(|m| mod_file.metadata.has_dependency(&m.metadata))
                .map(|m| chain.iter().position(|cm| cm == m).unwrap());

            // None of the dependencies exist, should be among the last as we expect them to come later
            if dep_pos.clone().count() == 0 {
                chain.push(mod_file);
                continue;
            }

            // Any of our dependencies exist
            chain.insert(dep_pos.last().unwrap() + 1, mod_file);
        }

        let bdirs = BaseDirs::new().unwrap();
        let temp_dir = bdirs.data_local_dir().join("Temp");
        // Now that we're sorted, let's extract the contents
        for mod_file in chain {
            if let Err(mut e) = mod_file.extract_archive(&xd3, &config.data_win.game_root, &temp_dir, &mut config.data_win.replaced_files) {
                if let Err(origin_err) = Self::reset_to_origin(config) {
                    e.1.push_str(format!("\nFailed to reset origin: {}", origin_err).as_str());
                }
                return Err((Some(e.0), e.1));
            }
        }

        Ok(())
    }

    fn set_btn_events(&self) {
        let buttons = &self.menus[0].buttons;
        let self_clone = self.clone();  // Shallow copy, retains the underlying pointer
        buttons[0].on().bn_clicked(move || {
            let appcfg = Self::get_appcfg();    // New app config is loaded each time this button is clicked, just to freshen data
            let mods_view = self_clone.menus[0].mods_view.as_ref().unwrap();
            Self::fill_main_view(mods_view, &appcfg);
            Ok(())
        });

        let self_clone = self.clone();  // Re-definition because the original clone was moved away
        buttons[1].on().bn_clicked(move || {
            let appcfg = Self::get_appcfg();
            self_clone.use_selected_data(appcfg);
            Ok(())
        });

        buttons[2].on().bn_clicked(move || {
            let _ = Command::new("explorer")
            .arg(Self::get_appdata_dir().join("mods").as_os_str())
            .spawn();
            Ok(())
        });

        let self_clone = self.clone();
        buttons[3].on().bn_clicked(move || {
            let mut appcfg = Self::get_appcfg();
            self_clone.show_popup_result(
                Self::purge_to_origin(&mut appcfg),
                |_| "Reset successful".to_string(),
                |e| format!("Failed to reset: {}", e)
            );
            Ok(())
        });

        let buttons = &self.menus[1].buttons;
        let self_clone = self.clone();
        buttons[0].on().bn_clicked(move || {
            let mut appcfg = Self::get_appcfg();
            let path = PathBuf::from(self_clone.menus[1].edits[0].text());
            appcfg.data_win.game_root = path;
            self_clone.show_popup_result(
                appcfg.save(),
                |_| "Save successful".to_string(),
                |e| format!("Failed to save config: {}", e)
            );
            Ok(())
        });

        let buttons = &self.popup.buttons;
        let self_clone = self.clone();
        buttons[0].on().bn_clicked(move || {
            self_clone.hide_popup();
            Ok(())
        });
    }

    fn set_window_ready (&self) {
        let self_clone = self.clone();
        self.wnd.on().wm_create(move |_| {
            let appcfg = Self::get_appcfg();
            let mods_view = self_clone.menus[0].mods_view.as_ref().unwrap();
            Self::fill_main_view(mods_view, &appcfg);
            Ok(0)
        });

        let self_clone = self.clone();
        self.tabs.on().tcn_sel_change (move || {
            if self_clone.tabs.items().selected().unwrap().index() != 1 {
                return Ok(());
            }

            let appcfg = Self::get_appcfg();
            let menu = self_clone.menus.get(1).unwrap();
            Self::fill_options_menu(menu, &appcfg);
            Ok(())
        });
    }
}