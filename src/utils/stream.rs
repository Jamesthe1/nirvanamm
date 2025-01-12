use std::{collections::HashSet, fs, hash::Hash, io, path::PathBuf};
use zip::ZipArchive;

pub fn stream_from_to<const N: usize>(mut read: impl FnMut(&mut [u8]) -> io::Result<usize>, mut write: impl FnMut(&[u8]) -> io::Result<usize>) {
    let mut buf = [0u8; N];
    while let Ok(count) = read(&mut buf) {
        if count == 0 {
            break;
        }
        let _ = write(&buf[..count]);
    }
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

/// Consumes the vector to deduplicate it
pub fn dedup<T>(vec: Vec<T>) -> Vec<T>
    where T: Eq + Hash
{
    let set: HashSet<T> = vec.into_iter().collect();  // Since every item in a set is unique, we can use this as a way to deduplicate data
    set.into_iter().collect()
}