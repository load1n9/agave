mod raw;

pub struct RGBA {
    r: i32,
    g: i32,
    b: i32,
    a: i32,
}
pub fn set_pixel(x: i32, y: i32, color: RGBA) {
    unsafe { raw::set_pixel(x, y, color.r, color.g, color.b, color.a) }
}
