mod raw;

pub struct RGBA {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}
pub fn set_pixel(x: i32, y: i32, color: RGBA) {
    unsafe { raw::set_pixel(x, y, color.r, color.g, color.b, color.a) }
}
