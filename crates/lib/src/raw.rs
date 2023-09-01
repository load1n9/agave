#[link(wasm_import_module = "agave")]
extern "C" {
    pub fn set_pixel(x: i32, y: i32, r: i32, g: i32, b: i32, a: i32);
}