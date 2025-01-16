//#![windows_subsystem = "windows"]   // Flag to not make the console appear

mod utils;

mod main_window;
use log::LevelFilter;
use main_window::MyWindow;

fn main() {
    let logpath = MyWindow::get_appdata_dir().join("latest.log");
    let _ = simple_logging::log_to_file(logpath.to_str().unwrap(), LevelFilter::Info);
    log::info!("Logger initialized");

    let mywin = MyWindow::new();
    if let Err(e) = mywin.wnd.run_main(None) {
        log::error!("Window error: {}", e);
    }
}