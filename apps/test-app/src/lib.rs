#[link(wasm_import_module = "agave")]
extern "C" {
    pub fn set_pixel(x: i32, y: i32, r: i32, g: i32, b: i32, a: i32);
}

#[no_mangle]
pub extern "C" fn _start() {

}

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    unsafe {
        set_pixel(mouse_x, mouse_y, 255, 0, 0, 0);
    }
}