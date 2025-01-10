mod mod_data;
use mod_data::*;

mod config;
use config::*;

use crate::utils::{stream::*, xdelta3::XDelta3};

use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipArchive, ZipWriter};

mod asref_winctrl;
use asref_winctrl::*;

use std::{borrow::Borrow, cell::RefCell, fs::{self, File}, io::{Read, Write}, path::PathBuf};

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

#[derive(Clone)]
struct PopupWindow {
    control:    gui::WindowControl,
    labels:     Vec<gui::Label>,
    buttons:    Vec<gui::Button>
}

#[derive(Clone)]
pub struct MyWindow {
    pub wnd:    gui::WindowMain,
    tabs:       gui::Tab,
    menus:      Vec<WindowMenu>,
    popup:      PopupWindow
}

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

        let control = gui::WindowControl::new(
            &wnd,
            gui::WindowControlOpts {
                position: (212, 334),
                size: Self::POPUP_SZ,
                style: WS::CHILD | WS::CLIPSIBLINGS | WS::DLGFRAME,
                ..Default::default()
            }
        );
        let labels = vec! {
            gui::Label::new(
                &control,
                gui::LabelOpts {
                    text: String::from("Placeholder"),
                    position: (10, 10),
                    size: (Self::POPUP_SZ.0 - 20, Self::POPUP_SZ.1 - 60),
                    ..Default::default()
                }
            )
        };
        let buttons = vec! {
            gui::Button::new(
                &control,
                gui::ButtonOpts {
                    text: String::from("&Ok"),
                    position: ((Self::POPUP_SZ.0 - 70).try_into().unwrap(), (Self::POPUP_SZ.1 - 40).try_into().unwrap()),
                    width: 60,
                    height: 30,
                    button_style: BS::CENTER | BS::PUSHBUTTON,
                    ..Default::default()
                }
            )
        };
        let popup = PopupWindow { control, labels, buttons };

        let mut menus = vec![];

        let control = gui::WindowControl::new(
            &wnd,
            gui::WindowControlOpts {
                position: (0, 20),
                size: (1024, 748),
                style: WS::CHILD | WS::CLIPSIBLINGS,
                ex_style: WS_EX::LEFT | WS_EX::CONTROLPARENT,
                ..Default::default()
            }
        );
        let control = WindowControlWrapper::new(control);
        let labels = vec! {
            gui::Label::new(
                control.as_ref(),
                gui::LabelOpts {
                    text: String::from(Self::APPNAME),
                    position: (20, 20),
                    size: (984, 20),
                    label_style: SS::CENTER,
                    ..Default::default()
                }
            ),
            gui::Label::new(
                control.as_ref(),
                gui::LabelOpts {
                    text: String::from("Click on the mod you wish to apply (shift-click for more than one), then click \"Patch\" (or press Alt-P)"),
                    position: (20, 50),
                    size: (984, 20),
                    ..Default::default()
                }
            )
        };
        // TODO: Add button to main window that opens explorer to the mods directory
        let buttons = vec! {
            gui::Button::new(
                control.as_ref(),
                gui::ButtonOpts {
                    text: String::from("&Refresh"),
                    position: (794, 80),
                    width: 40,
                    height: 40,
                    button_style: BS::CENTER | BS::PUSHBUTTON,  // Use ICON flag, set icon somehow
                    ..Default::default()
                }
            ),
            gui::Button::new(
                control.as_ref(),
                gui::ButtonOpts {
                    text: String::from("&Patch"),
                    position: (794, 688),
                    width: 200,
                    height: 40,
                    button_style: BS::CENTER | BS::PUSHBUTTON,
                    ..Default::default()
                }
            )
        };
        let edits = vec![];
        let mods_view =
            gui::ListView::new(
                control.as_ref(),
                gui::ListViewOpts {
                    position: (20, 80),
                    size: (764, 648),
                    columns: vec! {
                        (String::from("Name"), 200),
                        (String::from("GUID"), 200),
                        (String::from("Version"), 100),
                        (String::from("Author"), 150),
                        (String::from("Depends on"), 400)
                    },
                    ..Default::default()
                }
            );
        let mods_view = Some(mods_view);
        let title = String::from("Mods");
        menus.push(WindowMenu { title, control, labels, buttons, edits, mods_view });

        let control = gui::WindowControl::new(
            &wnd,
            gui::WindowControlOpts {
                position: (0, 20),
                size: (1024, 748),
                style: WS::CHILD | WS::CLIPSIBLINGS,
                ex_style: WS_EX::LEFT | WS_EX::CONTROLPARENT,
                ..Default::default()
            }
        );
        let control = WindowControlWrapper::new(control);
        let labels = vec! {
            gui::Label::new(
                control.as_ref(),
                gui::LabelOpts {
                    text: String::from(Self::APPNAME),
                    position: (20, 20),
                    size: (984, 20),
                    label_style: SS::CENTER,
                    ..Default::default()
                }
            ),
            gui::Label::new(
                control.as_ref(),
                gui::LabelOpts {
                    text: String::from("Game directory:"),
                    position: (20, 50),
                    size: (497, 20),
                    ..Default::default()
                }
            )
        };
        let buttons = vec! {
            gui::Button::new(
                control.as_ref(),
                gui::ButtonOpts {
                    text: String::from("&Save"),
                    position: (794, 688),
                    width: 200,
                    height: 40,
                    button_style: BS::CENTER | BS::PUSHBUTTON,
                    ..Default::default()
                }
            )
        };
        let edits = vec! {
            gui::Edit::new(
                control.as_ref(),
                gui::EditOpts {
                    text: String::new(),
                    position: (497, 50),
                    width: 507,
                    height: 20,
                    ..Default::default()
                }
            )
        };
        let mods_view = None;
        let title = String::from("Options");
        menus.push(WindowMenu { title, control, labels, buttons, edits, mods_view });

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
            match fs::create_dir_all(appdata_dir) { // appdata_dir is a borrowed Path, so it does not need to be re-borrowed here
                Err(e) => panic!("Could not create appdata directory: {}", e.to_string()),
                Ok(_) => ()
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
            match ModFile::new(filepath.to_owned()) {
                Err(e_msg) => eprintln!("{}", e_msg),
                Ok(mf) => {
                    let meta = mf.metadata.clone();
                    let selected = config.data_win.active_mods.contains(&meta.guid);

                    let mut hard_mods: Vec<String> = vec![];
                    let mut soft_mods: Vec<String> = vec![];
                    for d in meta.depends.unwrap_or_default().iter() {
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
                    let sep = String::from (", ");
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
                        mf
                    ).select(selected);
                }
            };
        }
    }

    fn fill_options_menu(menu: &WindowMenu, config: &AppConfig) {
        menu.edits[0].set_text(config.data_win.game_root.to_str().unwrap());
    }

    fn prepare_origin(&self, origin_path: PathBuf, game_root: PathBuf) -> Result<(), String> {
        let foptions = SimpleFileOptions::default();

        match fs::File::create(origin_path) {
            Err(e) => Err(e.to_string()),
            Ok(zf) => {
                let mut zip_file = ZipWriter::new(zf);
                for entry_rslt in WalkDir::new(&game_root) {
                    // If unwrapped without a definition to hold it, it would be dropped and the compiler recognizes that. So we must do it this way.
                    if let Ok(entry) = entry_rslt {
                        let path = entry.path();
                        let rel_path = path.strip_prefix(&game_root).unwrap();

                        if path.is_dir() {
                            let _ = zip_file.add_directory_from_path(rel_path, foptions);
                        }
                        else if path.is_file() {
                            let _ = zip_file.start_file_from_path(rel_path, foptions);
                            // TODO: Move to seperate thread and wait for result
                            // Update message box to inform this is zipping
                            if let Ok(mut f) = File::open(path) {
                                stream_from_to::<{Self::BUFSIZE}>(|buf| f.read(buf), |buf| zip_file.write(buf));
                            }
                        }
                    }
                }
                Ok(())
            }
        }
        // TODO: Add button called "Reset" that clears the game root, extracts the origin's contents, and purges the origin file and the active mods vector.
    }

    fn reset_to_origin(config: &mut AppConfig) -> Result<(), String> {
        let origin_path = Self::get_appdata_dir().join("origin.zip");
        match fs::File::open(origin_path) {
            Err(e) => Err(format!("Failed to open origin.zip: {}", e.to_string())),
            Ok(f) => {
                match ZipArchive::new(f) {
                    Err(e) => Err(format!("Failed to read origin as zip: {}", e.to_string())),
                    Ok(mut origin) => {
                        for entry in config.data_win.replaced_files.iter() {
                            let out_path = config.data_win.game_root.join(entry);
                            match origin.by_name(entry.to_str().unwrap()) {
                                Err(_) => {
                                    let _ = fs::remove_file(out_path);
                                },
                                Ok(mut in_file) => {
                                    match fs::File::create(out_path) {
                                        Err(e) => return Err(format!("Failed to extract origin file {}: {}", entry.to_str().unwrap(), e.to_string())),
                                        Ok(mut out_file) => {
                                            stream_from_to::<{Self::BUFSIZE}>(|buf| in_file.read(buf), |buf| out_file.write(buf));
                                        }
                                    }
                                }
                            }
                        }
                        config.data_win.replaced_files = vec![];
                        Ok(())
                    }
                }
            }
        }
    }

    fn use_selected_data(&self, config: &mut AppConfig) {
        let active_mods = &mut config.data_win.active_mods;
        let mut active_mod_files: Vec<ModFile> = vec![];
        if active_mods.len() > 0 {
            active_mods.clear();
        }

        let origin_path = Self::get_appdata_dir().join("origin.zip");
        if !origin_path.exists() {
            let _ = self.prepare_origin(origin_path, config.data_win.game_root.clone());
        }
        let mods_view = self.menus[0].mods_view.as_ref();
        for it in mods_view.unwrap().items().iter_selected() {
            match it.data() {
                Some(rc_mf) => {
                    let ref_mod_file: &RefCell<ModFile> = rc_mf.borrow();
                    let mod_file = ref_mod_file.borrow();
                    active_mods.push(mod_file.metadata.guid.to_owned());
                    active_mod_files.push(mod_file.clone());
                },
                None => (),
            };
        }

        match config.save() {
            Err(e) => eprintln!("Error saving config: {}", e),
            Ok(_) => {
                // TODO: Validate before saving the config
                // TODO: Check for file conflicts (that aren't mod.toml and patch.xdelta) before validating the dependencies
                let msg =
                    match Self::validate_mod_selection(&active_mod_files) {
                        Ok(_) => {
                            match Self::apply_mod_files(config, active_mod_files) {
                                Ok(()) => String::from("Patch success"),
                                Err((guid, e_msg)) => match guid {
                                    Some(g) => format!("Failed to apply mod {}\nReason: {}", g, e_msg),
                                    None => format!("Failed to apply mods: {}", e_msg)
                                }
                            }
                        },
                        Err((deps_unsatisfied, mods_blame)) => {
                            let deps_str = deps_unsatisfied.join(", ");
                            let blame_str = mods_blame.join(", ");
                            format!("Missing dependencies: {}\nRequired by: {}", deps_str, blame_str)
                        }
                    };
                self.popup.labels[0].set_text(msg.as_str());
                self.popup.control.hwnd().ShowWindow(SW::SHOW);
            }
        }
    }

    fn validate_mod_selection(active_mod_files: &Vec<ModFile>) -> Result<(), (Vec<String>, Vec<String>)> {
        let blank_str = String::new();
        let mut deps_unsatisfied: Vec<String> = vec![];
        let mut mods_blame: Vec<String> = vec![];
        for mod_file in active_mod_files.iter() {
            if !mod_file.has_dependencies() {
                continue;
            }
            
            let deps = mod_file.metadata.depends.clone().unwrap();  // Clone because unwrap also consumes
            let mut failed = false;
            for dep in deps.iter() {
                let hard_guid = match dep {
                    ModDependencyEnum::ImplicitHard(guid) => guid,
                    ModDependencyEnum::DependTable(md) => {
                        if !md.soft {
                            &md.guid
                        }
                        else {
                            &blank_str
                        }
                    }
                };
                if *hard_guid == blank_str {
                    continue;
                }
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
                if !m.has_dependencies() {
                    false
                }
                else {
                    m.has_dependency(&mod_file)
                }
            });
            if chain_pos.is_some() {
                chain.insert(chain_pos.unwrap(), mod_file);
                continue;
            }
            
            // Depends upon nothing, should be among the first
            if !mod_file.has_dependencies() {
                chain.insert(0, mod_file);
                continue;
            }

            let dep_pos = chain.iter()
                .filter(|m| mod_file.has_dependency(m))
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

        let _ = config.save();
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
            let mut appcfg = Self::get_appcfg();
            self_clone.use_selected_data(&mut appcfg);
            Ok(())
        });

        let buttons = &self.menus[1].buttons;
        let self_clone = self.clone();
        buttons[0].on().bn_clicked(move || {
            let mut appcfg = Self::get_appcfg();
            let path = PathBuf::from(self_clone.menus[1].edits[0].text());
            appcfg.data_win.game_root = path;
            let _ = appcfg.save();
            Ok(())
        });

        let buttons = &self.popup.buttons;
        let self_clone = self.clone();
        buttons[0].on().bn_clicked(move || {
            self_clone.popup.control.hwnd().ShowWindow(SW::HIDE);
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