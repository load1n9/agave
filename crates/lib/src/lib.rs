mod raw;

/// RGBA color
pub struct RGBA {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

/// Position on the screen
pub struct Position {
    pub x: i32,
    pub y: i32,
}

/// Dimensions of the screen
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

/// Set the pixel at the given position to the given color
pub fn set_pixel(position: Position, color: RGBA) {
    unsafe { raw::set_pixel(position.x, position.y, color.r, color.g, color.b, color.a) }
}

/// Set all pixels in the rectangle from the first position to the second position to the given color
pub fn set_pixels(from: Position, to: Position, color: RGBA) {
    unsafe {
        raw::set_pixels_from_to(
            from.x, from.y, to.x, to.y, color.r, color.g, color.b, color.a,
        )
    }
}

/// Get the dimensions of the screen
pub fn get_dimensions() -> Dimensions {
    Dimensions {
        width: unsafe { raw::get_width() },
        height: unsafe { raw::get_height() },
    }
}

/// Draw a circle with the given center, radius, and color
pub fn draw_circle(center: Position, radius: i32, color: RGBA) {
    unsafe {
        raw::draw_circle(
            center.x, center.y, radius, color.r, color.g, color.b, color.a,
        )
    }
}

/// Fill a rectangle with the given position, width, height, and color
pub fn fill_rectangle(pos: Position, width: i32, height: i32, color: RGBA) {
    unsafe {
        raw::fill_rectangle(
            pos.x, pos.y, width, height, color.r, color.g, color.b, color.a,
        )
    }
}

/// Draw a rectangle with the given position, width, height, and color
pub fn draw_rectangle(pos: Position, width: i32, height: i32, color: RGBA) {
    unsafe {
        raw::draw_rectangle(
            pos.x, pos.y, width, height, color.r, color.g, color.b, color.a,
        )
    }
}

/// Draw a line from the first position to the second position with the given color
pub fn draw_line(start: Position, end: Position, color: RGBA) {
    unsafe {
        raw::draw_line(
            start.x, start.y, end.x, end.y, color.r, color.g, color.b, color.a,
        )
    }
}
