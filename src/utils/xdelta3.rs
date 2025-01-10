use std::{ffi::c_void, mem, path::PathBuf};

#[link(name="xdelta3")]
extern "C" {
    fn xd3_main_cmdline(argc: i32, argv: *const *const u8) -> i32;
}

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

pub fn xd3_decode(in_file: PathBuf, patch_file: PathBuf, out_file: PathBuf) -> Result<(), i32> {
    let params = [
        "-d",   // Decode
        "-f",   // Overwrites output
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