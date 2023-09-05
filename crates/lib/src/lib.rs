mod raw;

pub struct RGBA {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

pub struct Position {
    pub x: i32,
    pub y: i32,
}

pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

pub fn set_pixel(position: Position, color: RGBA) {
    unsafe { raw::set_pixel(position.x, position.y, color.r, color.g, color.b, color.a) }
}

pub fn set_pixels(from: Position, to: Position, color: RGBA) {
    unsafe {
        raw::set_pixels_from_to(
            from.x, from.y, to.x, to.y, color.r, color.g, color.b, color.a,
        )
    }
}

pub fn get_dimensions() -> Dimensions {
    Dimensions {
        width: unsafe { raw::get_width() },
        height: unsafe { raw::get_height() },
    }
}
