mod raw;

/// RGBA color
#[derive(Debug, Clone, Copy)]
pub struct RGBA {
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub a: i32,
}

impl RGBA {
    /// Create a new RGBA color
    pub const fn new(r: i32, g: i32, b: i32, a: i32) -> Self {
        Self { r, g, b, a }
    }

    /// Predefined colors
    pub const RED: RGBA = RGBA::new(255, 0, 0, 255);
    pub const GREEN: RGBA = RGBA::new(0, 255, 0, 255);
    pub const BLUE: RGBA = RGBA::new(0, 0, 255, 255);
    pub const WHITE: RGBA = RGBA::new(255, 255, 255, 255);
    pub const BLACK: RGBA = RGBA::new(0, 0, 0, 255);
    pub const TRANSPARENT: RGBA = RGBA::new(0, 0, 0, 0);
}

/// Position on the screen
#[derive(Debug, Clone, Copy)]
pub struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    /// Create a new position
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Dimensions of the screen
#[derive(Debug, Clone, Copy)]
pub struct Dimensions {
    pub width: i32,
    pub height: i32,
}

/// Input state for mouse and keyboard
pub struct InputState {
    pub mouse_x: i32,
    pub mouse_y: i32,
    pub mouse_left: bool,
    pub mouse_right: bool,
    pub mouse_middle: bool,
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

/// Clear the entire screen with the given color
pub fn clear_screen(color: RGBA) {
    let dim = get_dimensions();
    fill_rectangle(Position::new(0, 0), dim.width, dim.height, color);
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

/// Draw a filled circle with the given center, radius, and color
pub fn fill_circle(center: Position, radius: i32, color: RGBA) {
    unsafe {
        raw::fill_circle(
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

/// Draw a rectangle outline with the given position, width, height, and color
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

/// Draw a triangle with the given vertices and color
pub fn draw_triangle(p1: Position, p2: Position, p3: Position, color: RGBA) {
    unsafe {
        raw::draw_triangle(
            p1.x, p1.y, p2.x, p2.y, p3.x, p3.y, color.r, color.g, color.b, color.a,
        )
    }
}

/// Draw text at the given position with the given color (placeholder)
pub fn draw_text(pos: Position, text: &str, color: RGBA) {
    // Simple text rendering - draw rectangles as placeholders for now
    let char_width = 8;
    let char_height = 12;
    for (i, _c) in text.chars().enumerate() {
        draw_rectangle(
            Position::new(pos.x + i as i32 * char_width, pos.y),
            char_width - 1,
            char_height,
            color,
        );
    }
}

/// Get current system time in milliseconds
pub fn get_time_ms() -> u64 {
    unsafe { raw::get_time_ms() }
}

/// temp function to test stuff
pub fn temp() {
    unsafe {
        raw::temp();
    }
}
