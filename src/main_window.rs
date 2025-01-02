mod mod_data;
use mod_data::{ModData, ModMetaData};

use std::{fs, io::Read, path::PathBuf};

// Prelude automatically imports necessary traits
use winsafe::{co::{BS, SS}, gui, prelude::*};
use directories::ProjectDirs;

#[derive(Clone)]
pub struct MyWindow {
    pub wnd:        gui::WindowMain,
    pub labels:     Vec<gui::Label>,
    pub buttons:    Vec<gui::Button>,
    pub main_view:  gui::ListView<PathBuf>, // Each item will contain the filename associated
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
            )
        };

        let buttons: Vec<gui::Button> = vec! {
            gui::Button::new(
                &wnd,
                gui::ButtonOpts {
                    text: String::from("&Refresh"),
                    position: (794, 50),
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

        let main_view: gui::ListView<PathBuf> =
            gui::ListView::new(
                &wnd,
                gui::ListViewOpts {
                    position: (20, 50),
                    size: (764, 698),
                    columns: vec! {
                        (String::from("Name"), 300),
                        (String::from("Version"), 100),
                        (String::from("Author"), 200)
                    },
                    ..Default::default()
                }
            );

        let new_self = Self { wnd, labels, buttons, main_view };
        new_self.set_btn_events();      // Events can only be set before `run_main` is executed
        new_self.set_window_ready();    // Functions such as `text()` or `items()` will fail if the window hasn't spawned yet (done in run_main), so modify them in the window ready event
        new_self
    }

    fn get_all_filepaths() -> Vec<PathBuf> {
        let pdirs = ProjectDirs::from(
            "Jamesthe1",
            "Jamesthe1",
            MyWindow::APPNAME
        ).unwrap();

        let appdata_dir = pdirs.data_dir();
        if !appdata_dir.exists() {
            match fs::create_dir_all(appdata_dir) { // appdata_dir is a borrowed Path, so it does not need to be re-borrowed here
                Err(e) => panic!("Could not create appdata directory: {}", e.to_string()),
                Ok(_) => ()
            }
        }
        
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

    fn fill_main_view(main_view: &gui::ListView<PathBuf>) {
        let items = main_view.items();
        if items.count() > 0 {
            items.delete_all();
        }

        let filepaths = MyWindow::get_all_filepaths();
        for filepath in filepaths.iter() {
            let filepath_str = filepath.to_str().unwrap();
            let file: fs::File;
            match fs::File::open(filepath) {
                Ok(f) => file = f,
                Err(e) => {
                    eprintln!("Error reading archive at {}: {}", filepath_str, e.to_string());
                    continue;
                }
            };

            let mut archive: zip::ZipArchive<fs::File>;
            match zip::ZipArchive::new (file) {
                Ok(z) => archive = z,
                Err(e) => {
                    eprintln!("Error parsing archive {}: {}", filepath_str, e.to_string());
                    continue;
                }
            };

            match archive.by_name("mod.toml") {
                Err(_) => {
                    eprintln!("{} does not contain a mod.toml file", filepath_str)
                },
                Ok(zip_entry) => {
                    match MyWindow::parse_mod_metadata(&items, zip_entry, filepath.to_owned()) {
                        Ok(_) => (),
                        Err(e_msg) => {
                            eprintln!("Failed to parse mod file in {}: {}", filepath_str, e_msg);
                            continue;
                        }
                    }
                }
            };
        }
    }

    fn parse_mod_metadata(view_items: &gui::spec::ListViewItems<'_, PathBuf>, mut mod_file: zip::read::ZipFile, filepath: PathBuf) -> Result<(), String> {
        let mut contents = String::new();
        match mod_file.read_to_string(&mut contents) {
            Ok(_) => {
                match toml::from_str::<ModData>(&contents) {
                    Ok(md) => {
                        let meta = md.metadata;
                        view_items.add(
                            &[
                                meta.name,
                                meta.version,
                                meta.author
                            ],
                            None,
                            filepath
                        );
                        Ok(())
                    },
                    Err(e) => Err(e.to_string())
                }
            },
            Err(e) => Err(e.to_string())
        }
    }

    fn get_selected_data(main_view: &gui::ListView<PathBuf>) {
        for it in main_view.items().iter_selected() {
            match it.data() {
                Some(filepath) => println!("Filepath of mod is {}", filepath.borrow().to_str().unwrap()),
                None => (),
            };
        }
    }

    fn set_btn_events(&self) {
        let self_clone = self.clone();  // Shallow copy, retains the underlying pointer
        self.buttons[0].on().bn_clicked(move || {
            MyWindow::fill_main_view(&self_clone.main_view);
            Ok(())
        });

        let self_clone = self.clone();  // Re-definition because the original clone was moved away
        self.buttons[1].on().bn_clicked(move || {
            MyWindow::get_selected_data(&self_clone.main_view);
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