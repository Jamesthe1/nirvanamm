//#![windows_subsystem = "windows"]   // Flag to not make the console appear

mod utils;

mod main_window;
use main_window::MyWindow;

fn main() {
    let mywin = MyWindow::new();
    if let Err(e) = mywin.wnd.run_main(None) {
        eprintln!("{}", e);
    }
}