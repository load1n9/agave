use agave_lib::{get_dimensions, set_pixels, Position, RGBA};

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    let dim = get_dimensions();

    set_pixels(
        Position {
            x: mouse_x,
            y: mouse_y,
        },
        Position {
            x: if mouse_x + 5 > dim.width {
                mouse_x
            } else {
                mouse_x + 5
            },
            y: if mouse_y + 5 > dim.height {
                mouse_y
            } else {
                mouse_y + 5
            },
        },
        RGBA {
            r: if mouse_x >= dim.width / 2 { 50 } else { 255 },
            g: 0,
            b: 255,
            a: 255,
        },
    );
}
