use std::{ffi::c_void, io::Read, iter, mem, path::PathBuf};

use libloading::{Library, Symbol};
use zip::read::ZipFile;
use base64::prelude::*;

unsafe fn allocate_c_str(input: &str) -> *mut u8 {
    let data = Vec::from(input);
    let len = data.len() + 1;   // Gap for null terminator
    let memsize = len * mem::size_of::<u8>();
    let ptr = libc::malloc(memsize) as *mut u8;
    for i in 0..len {
        let ch =
            if i < data.len() {
                data[i]
            } else {
                b'\0'
            };
        ptr.offset(i.try_into().unwrap()).write(ch);
    }
    return ptr;
}

unsafe fn deallocate_c_str(ptr: *mut u8) {
    libc::free(ptr as *mut c_void);
}

pub struct XDelta3 {
    lib: Library
}

pub struct XDeltaPatch {
    pub version: u8,
    pub use_second_compression: bool,
    pub code_table_length: usize,
    pub data_length: usize,
    pub description: String
}

impl XDelta3 {
    pub fn new() -> Result<XDelta3, String> {
        match unsafe {
            Library::new("libxdelta3.dll")
        } {
            Err(e) => Err(e.to_string()),
            Ok(lib) => Ok(XDelta3 { lib })
        }
    }

    fn decode_var_length(zip_file: &mut ZipFile) -> usize {
        let mut length: usize = 0;
        let mut buf = [0x80u8];
        while (buf[0] & 0x80u8) != 0u8 {
            length <<= 7;
            let _ = zip_file.read(&mut buf);
            length |= usize::from(buf[0] & 0x7fu8);
        }
        length
    }

    pub fn extract_patch_header(zip_file: &mut ZipFile) -> Result<XDeltaPatch, String> {
        let header = [0xd6u8, 0xc3u8, 0xc4u8];
        let mut buf = [0u8; 3];

        let _ = zip_file.read(&mut buf);
        if buf != header {
            return Err(format!("Patch file does not start with magic bytes, starts with 0x{:X} 0x{:X} 0x{:X}", buf[0], buf[1], buf[2]));
        }

        let mut buf = [0u8];
        let _ = zip_file.read(&mut buf);
        let version = buf[0];
        if version != 0 {
            return Err(String::from("Invalid version"));
        }
        
        let _ = zip_file.read(&mut buf);
        let flags = buf[0];
        if (flags & 0x04u8) == 0u8 {
            return Err(String::from("No data present at all"));
        }

        let use_second_compression = (flags & 0x01u8) != 0;
        if use_second_compression {
            let mut skip = [0u8];
            let _ = zip_file.read(&mut skip);
        }

        let mut code_table_length: usize = 0;
        if (flags & 0x02u8) != 0 {
            code_table_length = Self::decode_var_length(zip_file);
            let mut skip: Vec<u8> = iter::repeat_n(0u8, code_table_length).collect();
            let _ = zip_file.read(&mut skip);
        }

        let data_length = Self::decode_var_length(zip_file);
        if data_length < 2 {
            return Err(String::from("Not enough data"));
        }

        let mut buf: Vec<u8> = iter::repeat_n(0u8, data_length).collect();
        let _ = zip_file.read(&mut buf);
        let description = if buf[..1] != [b'^', b'*'] {
                String::new()
            }
            else {
                let mut description = match String::from_utf8(buf) {
                    Err(e) => return Err(format!("UTF-8 encoding error: {}", e.to_string())),
                    Ok(d) => d
                };
                description = description.split_off(1);
                description = match BASE64_STANDARD.decode(description) {
                    Err(e) => return Err(format!("base64 not decodable: {}", e.to_string())),
                    Ok(b64) => {
                        match String::from_utf8(b64) {
                            Err(e) => return Err(format!("UTF-8 encoding error (post-decode base64): {}", e.to_string())),
                            Ok(utf) => utf
                        }
                    }
                };
                // All must be uniform with UNIX newlines
                description = description.replace("\r\n", "\n");
                description = description.replace("\r", "\n");
                description
            };

        Ok(XDeltaPatch { version, use_second_compression, code_table_length, data_length, description })
    }

    pub fn decode(&self, in_file: PathBuf, patch_file: PathBuf, out_file: PathBuf) -> Result<(), i32> {
        let xd3_main_cmdline: Symbol<unsafe extern "C" fn(i32, *const *const u8) -> i32> = unsafe {self.lib.get(b"xd3_main_cmdline\0").unwrap()};
        let params = [
            "xdelta3",  // Dummy name
            "-d",       // Decode
            "-f",       // Overwrites output
            "-s", in_file.to_str().unwrap(),
            patch_file.to_str().unwrap(),
            out_file.to_str().unwrap()
        ];
    
        // Not emplacing in its own function, lest the pointer would end up outliving the vector
        unsafe {
            let c_strings: Vec<*mut u8> = params.iter().map(|s| allocate_c_str(s)).collect();
            let argv = c_strings.as_ptr() as *const *const u8;
            let res = xd3_main_cmdline(c_strings.len().try_into().unwrap(), argv);  // I wish we could interface with xdelta3's inner workings without having to re-write the entire xd3_stream struct in Rust
    
            // Doing it this way because we want to consume these pointers, as they will have no data attached to them once freed
            for c_str in c_strings {
                deallocate_c_str(c_str);
            }
    
            if res == 0 {
                Ok(())
            }
            else {
                Err(res)
            }
        }
    }
}