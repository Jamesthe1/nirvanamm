mod mod_data;
use mod_data::*;

mod config;
use config::*;

use log::{error, info, warn};
use crate::utils::{files::get_appdata_dir, stream::*, xdelta3::XDelta3};

use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

mod asref_winctrl;
use asref_winctrl::*;

mod mod_validation;
use mod_validation::*;
use mod_validation::ModCheckResult::*;

use std::{borrow::Borrow, cell::RefCell, fs, io::{Read, Write}, ops::Index, path::PathBuf, process::Command, thread};

// Prelude automatically imports necessary traits
use winsafe::{co::{BS, LR, SS, SW, WS, WS_EX}, gui::{self, Icon}, msg::bm::SetImage, prelude::*, BmpIcon, WString, HICON, HINSTANCE, HWND, SIZE};
use directories::BaseDirs;

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

enum MenuType {
    ModMenu,
    OptionsMenu
}

// Allows for our enum to specifically be used on the WindowMenu vector
impl Index<MenuType> for Vec<WindowMenu> {
    type Output = WindowMenu;
    
    fn index(&self, index: MenuType) -> &Self::Output {
        &self[index as usize]
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
        let class_icon = match Self::load_shared_icon("res/dorg.ico") {
            Err(e) => panic!("Couldn't load main icon: {}", e),
            Ok(ico) => Icon::Handle(ico)
        };

        let wnd = gui::WindowMain::new(
            gui::WindowMainOpts {
                title: format!("{} Control Panel", Self::APPNAME),
                size: (1024, 768),
                style: WS::CAPTION | WS::SYSMENU | WS::CLIPCHILDREN | WS::BORDER | WS::VISIBLE | WS::MINIMIZEBOX | WS::MAXIMIZEBOX,
                class_icon,
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
                button_style: BS::CENTER | BS::PUSHBUTTON | BS::ICON,
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
                button_style: BS::CENTER | BS::PUSHBUTTON | BS::ICON,
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
            },
            gui::ButtonOpts {   // Cannot create tooltip as there is no built-in one :(
                text: "Reset Origin".to_string(),
                position: (794, 638),
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

    fn get_appdata_dir_default() -> PathBuf {
        match get_appdata_dir(Self::APPNAME) {
            Err(e_msg) => panic!("Failed to get appdata directory: {}", e_msg),
            Ok(dir) => dir
        }
    }

    pub fn get_appdata_dir() -> PathBuf {
        if DirsConfig::cfg_exists() {
            match DirsConfig::get_appdata_dir_cfg(Self::APPNAME) {
                Err(e_msg) => {
                    log::error!("Failed to get app config: {}", e_msg);
                    Self::get_appdata_dir_default() // Using this after instead of storing in a variable, because this will need to be dealt with after we run into config issues
                },
                Ok(dir) => {
                    if dir == PathBuf::from("") {
                        Self::get_appdata_dir_default()
                    }
                    else {
                        dir
                    }
                }
            }
        }
        else {
            Self::get_appdata_dir_default()
        }
    }

    fn get_appcfg() -> AppConfig {
        let cfg_path = Self::get_appdata_dir().join(AppConfig::FILENAME);
        AppConfig::new(cfg_path)
    }

    fn get_all_mod_paths(appdata_dir: PathBuf) -> Result<Vec<PathBuf>, String> {
        let mods_dir = appdata_dir.join(ModFile::SUBDIRECTORY);
        if !mods_dir.exists() {
            if let Err(e) =  fs::create_dir(&mods_dir) {
                return Err(format!("Could not create mods directory in appdata: {}", e.to_string()));
            }
        }

        let mut paths: Vec<PathBuf> = vec![];
        for entry in fs::read_dir(mods_dir).unwrap() {
            if entry.is_err() {
                log::error!("A directory entry could not be read");
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

        Ok(paths)
    }

    fn fill_main_view(main_view: &gui::ListView<ModFile>, config: &AppConfig) {
        let items = main_view.items();
        if items.count() > 0 {
            items.delete_all();
        }

        let filepaths = match Self::get_all_mod_paths(Self::get_appdata_dir()) {
            Err(e) => {
                log::error!("Could not get mod paths: {}", e);
                return;
            },
            Ok(fs) => fs
        };

        for filepath in filepaths.iter() {
            // Making a clone of the filepath so it can exist within ModData
            let mod_file = match ModFile::new(filepath.to_owned()) {
                Err(e_msg) => {
                    log::error!("{}", e_msg);
                    continue;
                },
                Ok(mf) => mf
            };
            let meta = mod_file.metadata.clone();
            let selected = config.data_win.active_mods.contains(&meta.guid);

            let mut hard_mods: Vec<String> = vec![];
            let mut soft_mods: Vec<String> = vec![];
            for dep in meta.depends.iter().map(|d| ModMetaData::get_dependency(d).unwrap()) {
                let guid = format!("{} {}", dep.guid, dep.version);
                if dep.soft {
                    soft_mods.push(format!("[{}]", guid));
                }
                else {
                    hard_mods.push(guid);
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

    fn show_popup(&self, text: String, level: log::Level) {
        self.popup.label.set_text(text.as_str());
        self.set_popup_state(true);
        match level {
            log::Level::Error => error!("{}", text),
            log::Level::Warn => warn!("{}", text),
            log::Level::Info => info!("{}", text),
            _ => ()
        }
    }

    fn show_popup_result<T, U, Ft, Fu>(&self, result: Result<T, U>, ok_text: Ft, err_text: Fu)
        where
            Ft: Fn(T) -> String,
            Fu: Fn(U) -> String
    {
        let (text, level) = match result {
            Err(e) => (err_text(e), log::Level::Error),
            Ok(o) => (ok_text(o), log::Level::Info),
        };
        self.show_popup(text, level);
    }

    fn show_popup_option<T, Ft, F_>(&self, option: Option<T>, some_text: Ft, none_text: F_, level: log::Level)
        where
            Ft: Fn(T) -> String,
            F_: Fn() -> String
    {
        self.show_popup(match option {
            Some(t) => some_text(t),
            None => none_text()
        }, level);
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

        let mods_view = self.menus[MenuType::ModMenu].mods_view.as_ref();
        for it in mods_view.unwrap().items().iter_selected() {
            if let Some(rc_mf) = it.data() {
                let ref_mod_file: &RefCell<ModFile> = rc_mf.borrow();
                let mod_file = ref_mod_file.borrow();
                active_mods.push(mod_file.metadata.guid.to_owned());
                active_mod_files.push(mod_file.clone());
            };
        }

        match validate_active_mods(&active_mod_files) {
            ModsOk() => (),
            ModInsecurity(guid, e_msg) => {
                self.show_popup(format!("Mod security failure: {}\nCaused by: {}", e_msg, guid), log::Level::Error);
                return;
            },
            FailedDependency(deps, mods_blame) => {
                let deps_str = deps.join(", ");
                let blame_str = mods_blame.join(", ");
                self.show_popup(format!("Missing dependencies: {}\nRequired by: {}", deps_str, blame_str), log::Level::Error);
                return;
            },
            FileConflict(guid, mod_conflicts, file_conflicts) => {
                let files_str = file_conflicts.join(", ");
                let mods_str = mod_conflicts.join(", ");
                let patch_name = "patch.xdelta".to_string();
                let text = if file_conflicts.contains(&patch_name) {
                    "Incompatible patches".to_string()
                }
                else {
                    format!("Conflicting files: {}", files_str)
                };
                self.show_popup(format!("Mod {} is incompatible with {}\n{}", guid, mods_str, text), log::Level::Error);
                return;
            },
            // TODO: Maybe warn and give the user the option to continue?
            InvalidPatchNames(guid, bad_patches) => {
                let files_str = bad_patches.join(", ");
                self.show_popup(format!("Mod {} has patches not named patch.xdelta\n{}", guid, files_str), log::Level::Error);
                return;
            }
        }

        let self_clone = self.clone();
        self.show_popup("Applying selected mods...".to_string(), log::Level::Info);
        self.set_popup_button_state(false);
        thread::spawn(move || {
            if let Err((guid, e_msg)) = Self::apply_mod_files(&mut config, active_mod_files) {
                self_clone.show_popup_option(guid,
                    |g| format!("Failed to apply mod {}\nReason: {}", g, e_msg),
                    || format!("Failed to apply mods: {}", e_msg),
                    log::Level::Error
                );
                self_clone.set_popup_button_state(true);
                return;
            }

            self_clone.show_popup_result(
                config.save(),
                |_| "Patches succeeded".to_string(),
                |e| format!("Patches succeeded\nError saving config: {}", e)
            );
            self_clone.set_popup_button_state(true);
        });
    }

    fn use_selected_data(&self, config: AppConfig) {
        let origin_path = Self::get_appdata_dir().join("origin.zip");
        let game_root_clone = config.data_win.game_root.clone();
        let self_clone = self.clone();
        if !origin_path.exists() {
            self.show_popup("Preparing origin (this may take a while...)".to_string(), log::Level::Info);
            self.set_popup_button_state(false);
            // GDI can handle thread safety just fine actually, given it uses the message system with locks
            thread::spawn(move || {
                if let Err(e) = self_clone.prepare_origin(origin_path, game_root_clone) {
                    self_clone.show_popup(format!("Could not prepare origin: {}", e), log::Level::Error);
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

    fn load_shared_icon(filepath: &str) -> Result<HICON, String> {
        let load = LR::LOADFROMFILE | LR::DEFAULTSIZE | LR::SHARED;
        let name = winsafe::IdOicStr::Str(WString::from_str(filepath));
        match HINSTANCE::NULL.LoadImageIcon(name, SIZE::new(0, 0), load) {
            Err(e) => return Err(format!("Load error: {}", e.FormatMessage())),
            Ok(mut ig) => Ok(ig.leak()) // OK to leak because shared icons should not be destroyed
        }
    }

    fn set_icon(hwnd: &HWND, filepath: &str) -> Result<BmpIcon, String> {
        let image = match Self::load_shared_icon(filepath) {
            Err(e) => return Err(e),
            Ok(ico) => BmpIcon::Icon(ico)
        };
        match unsafe { hwnd.SendMessage(SetImage { image }) } {
            Err(e) => Err(format!("Set icon error: {}", e.FormatMessage())),
            Ok(i) => Ok(i)
        }
    }

    fn set_btn_icons(&self) {
        let buttons = &self.menus[MenuType::ModMenu].buttons;
        if let Err(e) = Self::set_icon(buttons[0].hwnd(), "res/refresh.ico") {
            log::error!("Couldn't set refresh icon: {}", e);
        }
        if let Err(e) = Self::set_icon(buttons[2].hwnd(), "res/folder.ico") {
            log::error!("Couldn't set folder icon: {}", e);
        }
    }

    fn set_btn_events(&self) {
        let buttons = &self.menus[MenuType::ModMenu].buttons;
        let self_clone = self.clone();  // Shallow copy, retains the underlying pointer
        buttons[0].on().bn_clicked(move || {
            let appcfg = Self::get_appcfg();    // New app config is loaded each time this button is clicked, just to freshen data
            let mods_view = self_clone.menus[MenuType::ModMenu].mods_view.as_ref().unwrap();
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

        let buttons = &self.menus[MenuType::OptionsMenu].buttons;
        let self_clone = self.clone();
        buttons[0].on().bn_clicked(move || {
            let mut appcfg = Self::get_appcfg();
            let path = PathBuf::from(self_clone.menus[MenuType::OptionsMenu].edits[0].text());
            appcfg.data_win.game_root = path;
            self_clone.show_popup_result(
                appcfg.save(),
                |_| "Save successful".to_string(),
                |e| format!("Failed to save config: {}", e)
            );
            Ok(())
        });

        let self_clone = self.clone();
        buttons[1].on().bn_clicked(move || {
            let self_clone_inner = self_clone.clone();
            self_clone.show_popup("Resetting the game to its original state...".to_string(), log::Level::Debug);
            self_clone.set_popup_button_state(false);
            thread::spawn(move || {
                let mut appcfg = Self::get_appcfg();
                self_clone_inner.show_popup_result(
                    Self::purge_to_origin(&mut appcfg),
                    |_| "Reset successful".to_string(),
                    |e| format!("Failed to reset: {}", e)
                );
                self_clone_inner.set_popup_button_state(true);
            });
            Ok(())
        });

        let buttons = &self.popup.buttons;
        let self_clone = self.clone();
        buttons[0].on().bn_clicked(move || {
            self_clone.hide_popup();
            Ok(())
        });
    }

    fn set_window_ready(&self) {
        let self_clone = self.clone();
        self.wnd.on().wm_create(move |_| {
            let appcfg = Self::get_appcfg();
            let mods_view = self_clone.menus[MenuType::ModMenu].mods_view.as_ref().unwrap();
            Self::fill_main_view(mods_view, &appcfg);
            self_clone.set_btn_icons(); // Button icons must be set after our window is initialized, because SendMessage relies on their HWND's being created (done in run_main).

            Ok(0)
        });

        let self_clone = self.clone();
        self.tabs.on().tcn_sel_change (move || {
            if self_clone.tabs.items().selected().unwrap().index() != 1 {
                return Ok(());
            }

            let appcfg = Self::get_appcfg();
            let menu = self_clone.menus.get(MenuType::OptionsMenu as usize).unwrap();
            Self::fill_options_menu(menu, &appcfg);
            Ok(())
        });
    }
}