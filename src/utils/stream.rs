use std::io;

pub fn stream_from_to<const N: usize>(mut read: impl FnMut(&mut [u8]) -> io::Result<usize>, mut write: impl FnMut(&[u8]) -> io::Result<usize>) {
    let mut buf = [0u8; N];
    while let Ok(count) = read(&mut buf) {
        if count == 0 {
            break;
        }
        let _ = write(&buf[..count]);
    }
}