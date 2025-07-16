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

/// Fill a rectangle with a vertical gradient from color1 (top) to color2 (bottom)
pub fn fill_gradient(from: Position, to: Position, color1: RGBA, color2: RGBA) {
    unsafe {
        raw::fill_gradient(
            from.x, from.y, to.x, to.y, color1.r, color1.g, color1.b, color1.a, color2.r, color2.g,
            color2.b, color2.a,
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

/// Draw a rounded rectangle with the given position, width, height, radius, and color
pub fn draw_rounded_rectangle(pos: Position, width: i32, height: i32, radius: i32, color: RGBA) {
    unsafe {
        raw::draw_rounded_rectangle(
            pos.x, pos.y, width, height, radius, color.r, color.g, color.b, color.a,
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

/// Draw text at the given position with the given color using a simple bitmap font
pub fn draw_text(pos: Position, text: &str, color: RGBA) {
    let char_width = 8;
    let char_height = 12;

    for (i, c) in text.chars().enumerate() {
        let char_x = pos.x + i as i32 * char_width;
        let char_y = pos.y;

        // Get bitmap for character
        let bitmap = get_char_bitmap(c);

        // Draw character bitmap
        for row in 0..char_height {
            for col in 0..char_width {
                if row < 8 && col < 8 {
                    // Our bitmap is 8x8
                    // Extract the byte for this row (bitmap stored with top row in MSB)
                    let row_byte = ((bitmap >> (8 * (7 - row))) & 0xFF) as u8;
                    // Check the bit for this column (LSB is leftmost pixel to fix horizontal flip)
                    if (row_byte & (1 << col)) != 0 {
                        fill_rectangle(Position::new(char_x + col, char_y + row), 1, 1, color);
                    }
                }
            }
        }
    }
}

/// Get a simple 8x8 bitmap for ASCII characters
fn get_char_bitmap(c: char) -> u64 {
    match c {
        ' ' => 0x0000000000000000,
        '!' => 0x1818181818001800,
        '"' => 0x3636000000000000,
        '#' => 0x36367F367F363600,
        '$' => 0x0C3E031E301F0C00,
        '%' => 0x006333180C664300,
        '&' => 0x1C361C6E3B336E00,
        '\'' => 0x0606030000000000,
        '(' => 0x180C0606060C1800,
        ')' => 0x060C1818180C0600,
        '*' => 0x00663CFF3C660000,
        '+' => 0x000C0C3F0C0C0000,
        ',' => 0x00000000000C0C06,
        '-' => 0x0000003F00000000,
        '.' => 0x00000000000C0C00,
        '/' => 0x6030180C06030100,
        '0' => 0x3E63737B6F673E00,
        '1' => 0x0C0E0C0C0C0C3F00,
        '2' => 0x1E33301C06333F00,
        '3' => 0x1E33301C30331E00,
        '4' => 0x383C363F33303800,
        '5' => 0x3F031F3030331E00,
        '6' => 0x1C06031F33331E00,
        '7' => 0x3F3330180C0C0C00,
        '8' => 0x1E33331E33331E00,
        '9' => 0x1E33333E30180E00,
        ':' => 0x000C0C00000C0C00,
        ';' => 0x000C0C00000C0C06,
        '<' => 0x180C0603060C1800,
        '=' => 0x00003F00003F0000,
        '>' => 0x060C1830180C0600,
        '?' => 0x1E3330180C000C00,
        '@' => 0x3E637B7B7B031E00,
        'A' => 0x0C1E33333F333300,
        'B' => 0x3F66663E66663F00,
        'C' => 0x3C66030303663C00,
        'D' => 0x1F36666666361F00,
        'E' => 0x7F46161E16467F00,
        'F' => 0x7F46161E16060F00,
        'G' => 0x3C66030373663C00,
        'H' => 0x3333333F33333300,
        'I' => 0x1E0C0C0C0C0C1E00,
        'J' => 0x7830303033331E00,
        'K' => 0x6766361E36666700,
        'L' => 0x0F06060646667F00,
        'M' => 0x63777F7F6B636300,
        'N' => 0x63676F7B73636300,
        'O' => 0x1C36636363361C00,
        'P' => 0x3F66663E06060F00,
        'Q' => 0x1E33333B39331E00,
        'R' => 0x3F66663E36666700,
        'S' => 0x1E33070E38331E00,
        'T' => 0x3F2D0C0C0C0C1E00,
        'U' => 0x3333333333331E00,
        'V' => 0x33333333331E0C00,
        'W' => 0x6363636B7F773600,
        'X' => 0x6363361C1C366300,
        'Y' => 0x3333331E0C0C1E00,
        'Z' => 0x7F6331184C667F00,
        '[' => 0x1E06060606061E00,
        '\\' => 0x03060C1830604000,
        ']' => 0x1E18181818181E00,
        '^' => 0x081C366300000000,
        '_' => 0x00000000000000FF,
        '`' => 0x0C0C180000000000,
        'a' => 0x00001E303E333E00,
        'b' => 0x0706063E66663B00,
        'c' => 0x00001E3303331E00,
        'd' => 0x3830303E33333E00,
        'e' => 0x00001E333F031E00,
        'f' => 0x1C36060F06060F00,
        'g' => 0x00003E33333E301F,
        'h' => 0x0706366E66666700,
        'i' => 0x0C000E0C0C0C1E00,
        'j' => 0x300030303033331E,
        'k' => 0x070606663E366700,
        'l' => 0x0E0C0C0C0C0C1E00,
        'm' => 0x0000337F7F6B6300,
        'n' => 0x00001F3333333300,
        'o' => 0x00001E3333331E00,
        'p' => 0x00003B66663E060F,
        'q' => 0x00003E33333E3078,
        'r' => 0x00003B6E66060F00,
        's' => 0x00001E031E301F00,
        't' => 0x080C3E0C0C2C1800,
        'u' => 0x0000333333333E00,
        'v' => 0x00003333331E0C00,
        'w' => 0x0000636B7F7F3600,
        'x' => 0x000033361C366300,
        'y' => 0x00003333333E301F,
        'z' => 0x00003F190C263F00,
        '{' => 0x380C0C070C0C3800,
        '|' => 0x1818181818181800,
        '}' => 0x070C0C380C0C0700,
        '~' => 0x6E3B000000000000,
        _ => 0x0000000000000000, // Default for unknown characters
    }
}

// Keyboard key codes (standard Linux input event codes)
pub const KEY_ESC: i32 = 1;
pub const KEY_1: i32 = 2;
pub const KEY_2: i32 = 3;
pub const KEY_3: i32 = 4;
pub const KEY_4: i32 = 5;
pub const KEY_5: i32 = 6;
pub const KEY_6: i32 = 7;
pub const KEY_7: i32 = 8;
pub const KEY_8: i32 = 9;
pub const KEY_9: i32 = 10;
pub const KEY_0: i32 = 11;
pub const KEY_MINUS: i32 = 12;
pub const KEY_EQUAL: i32 = 13;
pub const KEY_BACKSPACE: i32 = 14;
pub const KEY_TAB: i32 = 15;
pub const KEY_Q: i32 = 16;
pub const KEY_W: i32 = 17;
pub const KEY_E: i32 = 18;
pub const KEY_R: i32 = 19;
pub const KEY_T: i32 = 20;
pub const KEY_Y: i32 = 21;
pub const KEY_U: i32 = 22;
pub const KEY_I: i32 = 23;
pub const KEY_O: i32 = 24;
pub const KEY_P: i32 = 25;
pub const KEY_LEFTBRACE: i32 = 26;
pub const KEY_RIGHTBRACE: i32 = 27;
pub const KEY_ENTER: i32 = 28;
pub const KEY_LEFTCTRL: i32 = 29;
pub const KEY_A: i32 = 30;
pub const KEY_S: i32 = 31;
pub const KEY_D: i32 = 32;
pub const KEY_F: i32 = 33;
pub const KEY_G: i32 = 34;
pub const KEY_H: i32 = 35;
pub const KEY_J: i32 = 36;
pub const KEY_K: i32 = 37;
pub const KEY_L: i32 = 38;
pub const KEY_SEMICOLON: i32 = 39;
pub const KEY_APOSTROPHE: i32 = 40;
pub const KEY_GRAVE: i32 = 41;
pub const KEY_LEFTSHIFT: i32 = 42;
pub const KEY_BACKSLASH: i32 = 43;
pub const KEY_Z: i32 = 44;
pub const KEY_X: i32 = 45;
pub const KEY_C: i32 = 46;
pub const KEY_V: i32 = 47;
pub const KEY_B: i32 = 48;
pub const KEY_N: i32 = 49;
pub const KEY_M: i32 = 50;
pub const KEY_COMMA: i32 = 51;
pub const KEY_DOT: i32 = 52;
pub const KEY_SLASH: i32 = 53;
pub const KEY_RIGHTSHIFT: i32 = 54;
pub const KEY_SPACE: i32 = 57;

/// Check if a key was just pressed this frame
pub fn is_key_pressed(key_code: i32) -> bool {
    unsafe { raw::is_key_pressed(key_code) }
}

/// Check if a key is currently being held down
pub fn is_key_down(key_code: i32) -> bool {
    unsafe { raw::is_key_down(key_code) }
}

/// Check if a key was just released this frame
pub fn is_key_released(key_code: i32) -> bool {
    unsafe { raw::is_key_released(key_code) }
}

/// Get the number of keyboard events in the history buffer
pub fn get_key_history_count() -> i32 {
    unsafe { raw::get_key_history_count() }
}

/// Get a keyboard event from the history buffer (returns key code and pressed state)
pub fn get_key_history_event(index: i32) -> (i32, bool) {
    unsafe {
        let event = raw::get_key_history_event(index);
        let key_code = (event & 0xFFFFFFFF) as i32;
        let pressed = (event >> 32) != 0;
        (key_code, pressed)
    }
}

/// Convert key code to ASCII character (simple mapping)
pub fn key_code_to_char(key_code: i32, shift_pressed: bool) -> Option<char> {
    match key_code {
        KEY_A => Some(if shift_pressed { 'A' } else { 'a' }),
        KEY_B => Some(if shift_pressed { 'B' } else { 'b' }),
        KEY_C => Some(if shift_pressed { 'C' } else { 'c' }),
        KEY_D => Some(if shift_pressed { 'D' } else { 'd' }),
        KEY_E => Some(if shift_pressed { 'E' } else { 'e' }),
        KEY_F => Some(if shift_pressed { 'F' } else { 'f' }),
        KEY_G => Some(if shift_pressed { 'G' } else { 'g' }),
        KEY_H => Some(if shift_pressed { 'H' } else { 'h' }),
        KEY_I => Some(if shift_pressed { 'I' } else { 'i' }),
        KEY_J => Some(if shift_pressed { 'J' } else { 'j' }),
        KEY_K => Some(if shift_pressed { 'K' } else { 'k' }),
        KEY_L => Some(if shift_pressed { 'L' } else { 'l' }),
        KEY_M => Some(if shift_pressed { 'M' } else { 'm' }),
        KEY_N => Some(if shift_pressed { 'N' } else { 'n' }),
        KEY_O => Some(if shift_pressed { 'O' } else { 'o' }),
        KEY_P => Some(if shift_pressed { 'P' } else { 'p' }),
        KEY_Q => Some(if shift_pressed { 'Q' } else { 'q' }),
        KEY_R => Some(if shift_pressed { 'R' } else { 'r' }),
        KEY_S => Some(if shift_pressed { 'S' } else { 's' }),
        KEY_T => Some(if shift_pressed { 'T' } else { 't' }),
        KEY_U => Some(if shift_pressed { 'U' } else { 'u' }),
        KEY_V => Some(if shift_pressed { 'V' } else { 'v' }),
        KEY_W => Some(if shift_pressed { 'W' } else { 'w' }),
        KEY_X => Some(if shift_pressed { 'X' } else { 'x' }),
        KEY_Y => Some(if shift_pressed { 'Y' } else { 'y' }),
        KEY_Z => Some(if shift_pressed { 'Z' } else { 'z' }),
        KEY_1 => Some(if shift_pressed { '!' } else { '1' }),
        KEY_2 => Some(if shift_pressed { '@' } else { '2' }),
        KEY_3 => Some(if shift_pressed { '#' } else { '3' }),
        KEY_4 => Some(if shift_pressed { '$' } else { '4' }),
        KEY_5 => Some(if shift_pressed { '%' } else { '5' }),
        KEY_6 => Some(if shift_pressed { '^' } else { '6' }),
        KEY_7 => Some(if shift_pressed { '&' } else { '7' }),
        KEY_8 => Some(if shift_pressed { '*' } else { '8' }),
        KEY_9 => Some(if shift_pressed { '(' } else { '9' }),
        KEY_0 => Some(if shift_pressed { ')' } else { '0' }),
        KEY_SPACE => Some(' '),
        KEY_MINUS => Some(if shift_pressed { '_' } else { '-' }),
        KEY_EQUAL => Some(if shift_pressed { '+' } else { '=' }),
        KEY_LEFTBRACE => Some(if shift_pressed { '{' } else { '[' }),
        KEY_RIGHTBRACE => Some(if shift_pressed { '}' } else { ']' }),
        KEY_BACKSLASH => Some(if shift_pressed { '|' } else { '\\' }),
        KEY_SEMICOLON => Some(if shift_pressed { ':' } else { ';' }),
        KEY_APOSTROPHE => Some(if shift_pressed { '"' } else { '\'' }),
        KEY_GRAVE => Some(if shift_pressed { '~' } else { '`' }),
        KEY_COMMA => Some(if shift_pressed { '<' } else { ',' }),
        KEY_DOT => Some(if shift_pressed { '>' } else { '.' }),
        KEY_SLASH => Some(if shift_pressed { '?' } else { '/' }),
        _ => None,
    }
}

/// Get current system time in milliseconds
pub fn get_time_ms() -> u64 {
    unsafe { raw::get_time_ms() }
}

/// Grow the WebAssembly memory by the given number of pages (64KB each)
pub fn grow_memory(pages: u64) -> i32 {
    unsafe { raw::grow_memory(pages) }
}
