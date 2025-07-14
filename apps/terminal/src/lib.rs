mod types;
mod state;
mod commands;
mod input;
mod display;

use agave_lib::get_time_ms;
use state::{TERMINAL, LAST_TIME, ANIMATION_FRAME, CURSOR_BLINK};
use input::handle_keyboard_input;
use display::draw_terminal;

#[no_mangle]
pub extern "C" fn update(_mouse_x: i32, _mouse_y: i32) {
    unsafe {
        let current_time = get_time_ms();
        TERMINAL.uptime = current_time;
        
        // Update cursor blink and animation frame
        if current_time - LAST_TIME >= 500 {
            CURSOR_BLINK = !CURSOR_BLINK;
            LAST_TIME = current_time;
        }
        
        ANIMATION_FRAME = (ANIMATION_FRAME + 1) % 60;
        
        // Handle real keyboard input
        handle_keyboard_input();
    }

    draw_terminal();
}
