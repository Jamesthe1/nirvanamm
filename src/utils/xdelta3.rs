use std::{ffi::c_void, mem, path::PathBuf, sync::{LazyLock, Mutex}};

use libloading::{Library, Symbol};

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

type XPrintFPtr = *const fn(*mut u8) -> ();
static XD3_MESSAGES: LazyLock<Mutex<Vec<String>>> = LazyLock::new(|| Mutex::new(vec![]));

unsafe fn xd3_message_collect(mut msg: *mut u8) {
    let mut str = String::new();
    let mut lc = msg.read();
    while lc != 0 {
        str.push(char::from(lc));
        msg = msg.offset(1);
        lc = msg.read();
    }
    XD3_MESSAGES.lock().unwrap().push(str);
}

pub struct XDelta3 {
    lib: Library
}

impl XDelta3 {
    pub fn new() -> Result<XDelta3, String> {
        match unsafe {
            Library::new("xdelta3_bridge.dll")
        } {
            Err(e) => Err(e.to_string()),
            Ok(lib) => Ok(XDelta3 { lib })
        }
    }

    pub fn decode(&self, in_file: PathBuf, patch_file: PathBuf, out_file: PathBuf) -> Result<(), String> {
        // We cannot set xprintf_message_func (xdelta's logger function) because Rust does not like mutable global variables :(
        // A workaround would be to compile a small DLL in C with a function that takes in the xprintf function pointer, and sets that global for us
        // TODO: Implement
        let xd3_call: Symbol<unsafe extern "C" fn(i32, *const *const u8, XPrintFPtr) -> i32> = unsafe {self.lib.get(b"xd3_call\0").unwrap()};
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
            let res = xd3_call(c_strings.len().try_into().unwrap(), argv, xd3_message_collect as XPrintFPtr);
    
            // Doing it this way because we want to consume these pointers, as they will have no data attached to them once freed
            for c_str in c_strings {
                deallocate_c_str(c_str);
            }
    
            if res == 0 {
                Ok(())
            }
            else {
                let mut msg_guard = XD3_MESSAGES.lock().unwrap();
                let messages = msg_guard.join("");
                msg_guard.clear();
                Err(messages)
            }
        }
    }
}