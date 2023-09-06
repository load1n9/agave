use agave_lib::{draw_circle, get_dimensions, temp, Position, RGBA};

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    let dim = get_dimensions();

    draw_circle(
        Position {
            x: mouse_x,
            y: mouse_y,
        },
        10,
        RGBA {
            r: if mouse_x >= dim.width / 2 { 50 } else { 255 },
            g: 0,
            b: 255,
            a: 255,
        },
    );
    temp();
}
