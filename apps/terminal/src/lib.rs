mod commands;
mod display;
mod input;
mod state;
mod themes;
mod types;

use agave_lib::get_time_ms;
use display::draw_terminal;
use input::handle_keyboard_input;
use state::{ANIMATION_FRAME, CURSOR_BLINK, LAST_TIME, TERMINAL};

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
