#[link(wasm_import_module = "agave")]
unsafe extern "C" {
    pub fn set_pixel(x: i32, y: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn set_pixels_from_to(x0: i32, y0: i32, x1: i32, y1: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn get_width() -> i32;
    pub fn get_height() -> i32;
    pub fn draw_circle(x: i32, y: i32, radius: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn fill_circle(x: i32, y: i32, radius: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn fill_gradient(
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        r1: i32,
        g1: i32,
        b1: i32,
        a1: i32,
        r2: i32,
        g2: i32,
        b2: i32,
        a2: i32,
    );
    pub fn draw_triangle(
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        r: i32,
        g: i32,
        b: i32,
        a: i32,
    );
    pub fn fill_rectangle(x: i32, y: i32, width: i32, height: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn draw_rectangle(x: i32, y: i32, width: i32, height: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn draw_rounded_rectangle(
        x: i32,
        y: i32,
        width: i32,
        height: i32,
        radius: i32,
        r: i32,
        g: i32,
        b: i32,
        a: i32,
    );
    pub fn draw_line(x0: i32, y0: i32, x1: i32, y1: i32, r: i32, g: i32, b: i32, a: i32);
    pub fn get_time_ms() -> u64;

    pub fn is_key_pressed(key_code: i32) -> bool;
    pub fn is_key_down(key_code: i32) -> bool;
    pub fn is_key_released(key_code: i32) -> bool;
    pub fn get_key_history_count() -> i32;
    pub fn get_key_history_event(index: i32) -> i64;
}
