#[link(wasm_import_module = "agave")]
extern "C" {
    pub fn set_pixel(x: i32, y: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn set_pixels_from_to(x0: i32, y0: i32, x1: i32, y1: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn get_width() -> i32;
    pub fn get_height() -> i32;
    pub fn draw_circle(x: i32, y: i32, radius: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn fill_rectangle(x: i32, y: i32, width: i32, height: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn draw_rectangle(x: i32, y: i32, width: i32, height: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn temp();
}
