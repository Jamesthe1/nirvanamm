mod mod_data;
use mod_data::*;

mod config;
use config::*;

use std::{borrow::Borrow, cell::RefCell, fs, path::PathBuf};

// Prelude automatically imports necessary traits
use winsafe::{co::{BS, SS}, gui, prelude::*};
use directories::ProjectDirs;

#[derive(Clone)]
pub struct MyWindow {
    pub wnd:        gui::WindowMain,
    pub labels:     Vec<gui::Label>,
    pub buttons:    Vec<gui::Button>,
    pub main_view:  gui::ListView<ModData>, // Each item will contain the filename associated
}

impl MyWindow {
    const APPNAME: &str = "NirvanaMM";

    pub fn new() -> Self {
        let wnd = gui::WindowMain::new(
            gui::WindowMainOpts {
                title: format!("{} Control Panel", Self::APPNAME),
                size: (1024, 768),
                ..Default::default()    // Makes the rest of the fields default
            }
        );

        let labels: Vec<gui::Label> = vec! {
            gui::Label::new(
                &wnd,
                gui::LabelOpts {
                    text: String::from(Self::APPNAME),
                    position: (20, 20),
                    size: (984, 20),
                    label_style: SS::CENTER,
                    ..Default::default()
                }
            ),
            gui::Label::new(
                &wnd,
                gui::LabelOpts {
                    text: String::from("Click on the mod you wish to apply (shift-click for more than one), then click \"Patch\" (or press Alt-P)"),
                    position: (20, 50),
                    size: (984, 20),
                    ..Default::default()
                }
            )
        };

        let buttons: Vec<gui::Button> = vec! {
            gui::Button::new(
                &wnd,
                gui::ButtonOpts {
                    text: String::from("&Refresh"),
                    position: (794, 80),
                    width: 40,
                    height: 40,
                    button_style: BS::CENTER | BS::PUSHBUTTON,
                    ..Default::default()
                }
            ),
            gui::Button::new(
                &wnd,
                gui::ButtonOpts {
                    text: String::from("&Patch"),
                    position: (794, 708),
                    width: 200,
                    height: 40,
                    button_style: BS::CENTER | BS::PUSHBUTTON,  // Use ICON flag, set icon somehow
                    ..Default::default()
                }
            )
        };

        let main_view: gui::ListView<ModData> =
            gui::ListView::new(
                &wnd,
                gui::ListViewOpts {
                    position: (20, 80),
                    size: (764, 668),
                    columns: vec! {
                        (String::from("Name"), 200),
                        (String::from("GUID"), 200),
                        (String::from("Version"), 100),
                        (String::from("Author"), 150),
                        (String::from("Depends on"), 200)
                    },
                    ..Default::default()
                }
            );

        let new_self = Self { wnd, labels, buttons, main_view };
        new_self.set_btn_events();      // Events can only be set before `run_main` is executed
        new_self.set_window_ready();    // Functions such as `text()` or `items()` will fail if the window hasn't spawned yet (done in run_main), so modify them in the window ready event
        new_self
    }

    fn get_appdata_dir() -> PathBuf {
        let pdirs = ProjectDirs::from(
            "Jamesthe1",
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
        let mods_dir = appdata_dir.join("mods");
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

    fn fill_main_view(main_view: &gui::ListView<ModData>, config: &AppConfig) {
        let items = main_view.items();
        if items.count() > 0 {
            items.delete_all();
        }

        let filepaths = Self::get_all_mod_paths(Self::get_appdata_dir());
        for filepath in filepaths.iter() {
            // Making a clone of the filepath so it can exist within ModData
            match ModData::new(filepath.to_owned()) {
                Err(e_msg) => eprintln!("{}", e_msg),
                Ok(mf) => {
                    let meta = mf.metadata.clone();
                    let selected = config.data_win.active_mods.contains(&meta.guid);
                    let mut depend_str = String::new();

                    for d in meta.depends.unwrap_or_default().iter() {
                        if depend_str.len() > 0 {
                            depend_str.push_str(", ");
                        }
                        match d {
                            ModDependencyEnum::ImplicitHard(guid) => {
                                depend_str.push_str(guid);
                            }
                            ModDependencyEnum::DependTable(md) => {
                                let mut guid = md.guid.clone();
                                if md.soft {
                                    guid.insert_str(0, "[");
                                    guid.push(']');
                                }
                                depend_str.push_str(&guid);
                            }
                        }
                    }

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

    fn use_selected_data(main_view: &gui::ListView<ModData>, config: &mut AppConfig) {
        let active_mods = &mut config.data_win.active_mods;
        if active_mods.len() > 0 {
            active_mods.clear();
        }

        // TODO: Copy data.win to appdata directory
        let appdata_dir = Self::get_appdata_dir();
        for it in main_view.items().iter_selected() {
            match it.data() {
                Some(rc_mf) => {
                    let ref_mod_file: &RefCell<ModData> = rc_mf.borrow();
                    let mod_file = ref_mod_file.borrow();
                    active_mods.push(mod_file.metadata.guid.to_owned());
                    println!("Pushed {} to active mods", active_mods.last().unwrap())
                },
                None => (),
            };
        }

        match config.save() {
            Err(e) => eprintln!("Error saving config: {}", e),
            Ok(_) => {
                // TODO: Patch here
                println!("Patch success");
            }
        }
    }

    fn set_btn_events(&self) {
        let self_clone = self.clone();  // Shallow copy, retains the underlying pointer
        self.buttons[0].on().bn_clicked(move || {
            let appcfg = Self::get_appcfg();    // New app config is loaded each time this button is clicked, just to freshen data
            Self::fill_main_view(&self_clone.main_view, &appcfg);
            Ok(())
        });

        let self_clone = self.clone();  // Re-definition because the original clone was moved away
        self.buttons[1].on().bn_clicked(move || {
            let mut appcfg = Self::get_appcfg();
            Self::use_selected_data(&self_clone.main_view, &mut appcfg);
            Ok(())
        });
    }

    fn set_window_ready (&self) {
        let self_clone = self.clone();
        self.wnd.on().wm_create(move |_| {
            let appcfg = Self::get_appcfg();
            MyWindow::fill_main_view(&self_clone.main_view, &appcfg);
            Ok(0)
        });
    }
}