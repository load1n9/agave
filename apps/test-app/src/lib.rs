use agave_lib::{set_pixel, RGBA};

#[no_mangle]
pub extern "C" fn _start() {}

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    unsafe {
        set_pixel(
            mouse_x,
            mouse_y,
            RGBA {
                r: 255,
                g: 0,
                b: 0,
                a: 255,
            },
        );
    }
}
