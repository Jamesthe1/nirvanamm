//#![windows_subsystem = "windows"]   // Flag to not make the console appear

// Prelude automatically imports necessary traits
use winsafe::{co::{BS, SS}, gui, prelude::*};

fn main() {
    let mywin = MyWindow::new();
    if let Err(e) = mywin.wnd.run_main(None) {
        eprintln!("{}", e);
    }
}

#[derive(Clone)]
pub struct MyWindow {
    wnd:        gui::WindowMain,
    labels:     Vec<gui::Label>,
    buttons:    Vec<gui::Button>,
    main_view:  gui::ListView<String>,  // Each item will contain the filename associated
}

impl MyWindow {
    pub fn new() -> Self {
        let wnd = gui::WindowMain::new(
            gui::WindowMainOpts {
                title: String::from("NirvanaMM Control Panel"),
                size: (1024, 768),
                ..Default::default()    // Makes the rest of the fields default
            }
        );

        let labels: Vec<gui::Label> = vec! {
            gui::Label::new(
                &wnd,
                gui::LabelOpts {
                    text: String::from("NirvanaMM"),
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
                    width: 200,
                    height: 200,
                    button_style: BS::CENTER | BS::PUSHBUTTON,
                    ..Default::default()
                }
            ),
            gui::Button::new(
                &wnd,
                gui::ButtonOpts {
                    text: String::from("&Patch"),
                    position: (794, 728),
                    width: 200,
                    height: 20,
                    button_style: BS::CENTER | BS::PUSHBUTTON,  // Use ICON flag, set icon somehow
                    ..Default::default()
                }
            )
        };

        let main_view: gui::ListView<String> =
            gui::ListView::new(
                &wnd,
                gui::ListViewOpts {
                    position: (20, 50),
                    size: (764, 698),
                    columns: vec! {
                        (String::from("Name"), 300),
                        (String::from("Author"), 200),
                        (String::from("Version"), 100)
                    },
                    ..Default::default()
                }
            );

        let new_self = Self { wnd, labels, buttons, main_view };
        new_self.set_btn_events();
        new_self.set_window_ready();    // Functions such as `text()` or `items()` will fail if the window hasn't spawned yet (done in run_main), so modify them in the window ready event
        new_self
    }

    fn fill_main_view(main_view: &gui::ListView<String>) {
        // TODO: List all .zip files in <app data>/mods directory
        main_view.items().add(
            &[
                "Example",
                "Jamesthe1",
                "v1.0.0"
            ],
            None,
            String::from("example-file-name.zip")
        );
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

    fn get_selected_data(main_view: &gui::ListView<String>) {
        for it in main_view.items().iter_selected() {
            match it.data() {
                Some(filepath) => println!("Filepath of mod is {}", filepath.borrow()),
                None => (),
            };
        }
    }
}