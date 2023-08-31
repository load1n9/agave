#[link(wasm_import_module = "temp")]
extern "C" {
    pub fn hello(arg: i32);
}

#[no_mangle]
pub extern "C" fn _start() {
    unsafe {
        hello(42);
    }
}
