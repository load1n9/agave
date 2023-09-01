use agave_lib::{set_pixel, RGBA};

#[no_mangle]
pub extern "C" fn _start() {}

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    set_pixel(
        mouse_x,
        mouse_y,
        RGBA {
            r: if mouse_x >= 255 { 255 } else { 0 },
            g: 0,
            b: 255,
            a: 255,
        },
    );
}
