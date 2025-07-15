use agave_lib::{
    get_key_history_count, get_key_history_event, is_key_down, key_code_to_char, KEY_BACKSPACE,
    KEY_ENTER, KEY_ESC, KEY_LEFTSHIFT, KEY_RIGHTSHIFT,
};

use crate::state::{COMMAND_HISTORY, COMMAND_HISTORY_COUNT, COMMAND_HISTORY_INDEX, TERMINAL};

// Add arrow keys and page keys for navigation
const KEY_UP: i32 = 103;
const KEY_DOWN: i32 = 108;
const KEY_PAGEUP: i32 = 104;
const KEY_PAGEDOWN: i32 = 109;
const KEY_HOME: i32 = 102;
const KEY_END: i32 = 107;

pub fn handle_keyboard_input() {
    unsafe {
        let shift_pressed = is_key_down(KEY_LEFTSHIFT) || is_key_down(KEY_RIGHTSHIFT);

        // Process keyboard history to catch all events
        let history_count = get_key_history_count();
        static mut LAST_PROCESSED_COUNT: i32 = 0;

        // Process new keyboard events
        if history_count > LAST_PROCESSED_COUNT {
            for i in LAST_PROCESSED_COUNT..history_count {
                let (key_code, pressed) = get_key_history_event(i);

                if pressed {
                    // Handle special keys first
                    if handle_special_key(key_code, shift_pressed) {
                        continue;
                    }

                    // Handle character input
                    if let Some(ch) = key_code_to_char(key_code, shift_pressed) {
                        handle_character_input(ch);
                    }
                }
            }
            LAST_PROCESSED_COUNT = history_count;
        }
    }
}

// Separate function to handle special keys
fn handle_special_key(key_code: i32, _shift_pressed: bool) -> bool {
    unsafe {
        match key_code {
            KEY_ENTER => {
                #[allow(static_mut_refs)]
                TERMINAL.process_command();
                true
            }
            KEY_BACKSPACE => {
                if TERMINAL.command_length > 0 {
                    TERMINAL.command_length -= 1;
                    TERMINAL.command_buffer[TERMINAL.command_length] = 0;
                }
                true
            }
            KEY_ESC => {
                TERMINAL.command_length = 0;
                for j in 0..2048 {
                    TERMINAL.command_buffer[j] = 0;
                }
                true
            }
            KEY_UP => {
                if TERMINAL.command_length == 0 {
                    // No command typed, scroll up in output history
                    #[allow(static_mut_refs)]
                    TERMINAL.scroll_up(3);
                } else if COMMAND_HISTORY_INDEX > 0 {
                    // Command typed, navigate command history
                    COMMAND_HISTORY_INDEX -= 1;
                    // Load command from history
                    TERMINAL.command_length = 0;
                    for j in 0..2048 {
                        if COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j] == 0 {
                            break;
                        }
                        TERMINAL.command_buffer[j] = COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j];
                        TERMINAL.command_length += 1;
                    }
                }
                true
            }
            KEY_DOWN => {
                if TERMINAL.command_length == 0 {
                    // No command typed, scroll down in output history
                    #[allow(static_mut_refs)]
                    TERMINAL.scroll_down(3);
                } else if COMMAND_HISTORY_INDEX < COMMAND_HISTORY_COUNT {
                    // Command typed, navigate command history
                    COMMAND_HISTORY_INDEX += 1;
                    if COMMAND_HISTORY_INDEX >= COMMAND_HISTORY_COUNT {
                        // Clear command line
                        TERMINAL.command_length = 0;
                        for j in 0..2048 {
                            TERMINAL.command_buffer[j] = 0;
                        }
                    } else {
                        // Load command from history
                        TERMINAL.command_length = 0;
                        for j in 0..2048 {
                            if COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j] == 0 {
                                break;
                            }
                            TERMINAL.command_buffer[j] = COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j];
                            TERMINAL.command_length += 1;
                        }
                    }
                }
                true
            }
            KEY_PAGEUP => {
                #[allow(static_mut_refs)]
                TERMINAL.scroll_up(10);
                true
            }
            KEY_PAGEDOWN => {
                #[allow(static_mut_refs)]
                TERMINAL.scroll_down(10);
                true
            }
            KEY_HOME => {
                if TERMINAL.command_length == 0 {
                    #[allow(static_mut_refs)]
                    TERMINAL.scroll_to_top();
                }
                true
            }
            KEY_END => {
                if TERMINAL.command_length == 0 {
                    #[allow(static_mut_refs)]
                    TERMINAL.scroll_to_bottom();
                }
                true
            }
            _ => false,
        }
    }
}

// Separate function to handle character input with better bounds checking
fn handle_character_input(ch: char) {
    unsafe {
        // Validate that the character is printable and safe
        let ch_byte = ch as u8;
        if ch_byte < 32 || ch_byte > 126 {
            return; // Skip non-printable characters
        }

        // Check buffer bounds more conservatively
        if TERMINAL.command_length < 2046 {
            // Updated for new command buffer size - leave more space for safety
            TERMINAL.command_buffer[TERMINAL.command_length] = ch_byte;
            TERMINAL.command_length += 1;
            // Ensure null termination
            TERMINAL.command_buffer[TERMINAL.command_length] = 0;
        } else {
            // Command too long - auto-clear to prevent issues
            #[allow(static_mut_refs)]
            TERMINAL.add_output_line(b"Command too long - cleared");
            TERMINAL.command_length = 0;
            for j in 0..2048 {
                TERMINAL.command_buffer[j] = 0;
            }
        }
    }
}
