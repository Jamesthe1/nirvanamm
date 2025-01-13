use std::{fs, io, path::PathBuf};
use zip::ZipArchive;

pub fn stream_from_to<const N: usize>(mut read: impl FnMut(&mut [u8]) -> io::Result<usize>, mut write: impl FnMut(&[u8]) -> io::Result<()>) -> Result<usize, String> {
    let mut buf = [0u8; N];
    let mut total = 0;
    while let Ok(count) = read(&mut buf) {
        if count == 0 {
            break;
        }
        match write(&buf[..count]) {
            Ok(_) => total += count,
            Err(e) => return Err(format!("Failed to write: {}", e.to_string()))
        }
    }
    Ok(total)
}

pub fn open_archive(filepath: &PathBuf) -> Result<ZipArchive<fs::File>, String> {
    let filepath_str = filepath.to_str().unwrap();
    match fs::File::open(&filepath) {
        Err(e) => Err(format!("Error reading archive at {}: {}", filepath_str, e.to_string())),
        Ok(file) => {
            match ZipArchive::new(file) {
                Err(e) => Err(format!("Error reading archive {}: {}", filepath_str, e.to_string())),
                Ok(archive) => Ok(archive)
            }
        }
    }
}