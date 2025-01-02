mod mod_data;
use mod_data::*;

mod config;
use config::*;

use std::{fs, path::PathBuf};

// Prelude automatically imports necessary traits
use winsafe::{co::{BS, SS}, gui, prelude::*};
use directories::ProjectDirs;

#[derive(Clone)]
pub struct MyWindow {
    pub wnd:        gui::WindowMain,
    pub labels:     Vec<gui::Label>,
    pub buttons:    Vec<gui::Button>,
    pub main_view:  gui::ListView<ModFile>, // Each item will contain the filename associated

    pub config: AppConfig
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

        let main_view: gui::ListView<ModFile> =
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
                        (String::from("Depends"), 200)
                    },
                    ..Default::default()
                }
            );

        let cfg_path = Self::get_appdata_dir().join(AppConfig::FILENAME);
        let config = AppConfig::new(cfg_path);

        let new_self = Self { wnd, labels, buttons, main_view, config };
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

    fn get_all_filepaths(appdata_dir: PathBuf) -> Vec<PathBuf> {
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

    fn fill_main_view(main_view: &gui::ListView<ModFile>) {
        let items = main_view.items();
        if items.count() > 0 {
            items.delete_all();
        }

        let filepaths = Self::get_all_filepaths(Self::get_appdata_dir());
        for filepath in filepaths.iter() {
            match ModFile::new(filepath.to_owned()) {
                Err(e_msg) => eprintln!("{}", e_msg),
                Ok(mf) => {
                    let meta = mf.data.metadata.clone();
                    let depends = meta.depends.unwrap_or_default();
                    items.add(
                        &[
                            meta.name,
                            meta.guid,
                            meta.version,
                            meta.author,
                            depends
                        ],
                        None,
                        mf
                    );
                }
            };
        }
    }

    fn use_selected_data(main_view: &gui::ListView<ModFile>, config: &mut AppConfig) {
        let active_mods = &mut config.data_win.active_mods;
        if active_mods.len() > 0 {
            active_mods.clear();
        }

        // TODO: Copy data.win to appdata directory
        let appdata_dir = Self::get_appdata_dir();
        for it in main_view.items().iter_selected() {
            match it.data() {
                Some(ref_mf) => {
                    let mod_file = ref_mf.borrow();
                    active_mods.push(mod_file.data.metadata.guid.to_owned());
                    println!("Pushed {} to active mods", active_mods.last().unwrap())
                },
                None => (),
            };
        }

        let cfg_path = appdata_dir.join(AppConfig::FILENAME);
        match config.save(cfg_path) {
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
            Self::fill_main_view(&self_clone.main_view);
            Ok(())
        });

        let self_clone = self.clone();  // Re-definition because the original clone was moved away
        self.buttons[1].on().bn_clicked(move || {
            let mut cfg_clone = self_clone.config.clone();
            Self::use_selected_data(&self_clone.main_view, &mut cfg_clone);
            Ok(())
        });
    }

    fn set_window_ready (&self) {
        let self_clone = self.clone();
        self.wnd.on().wm_create(move |_| {
            MyWindow::fill_main_view(&self_clone.main_view);
            Ok(0)
        });
    }
}