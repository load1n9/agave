use agave_lib::{
    is_key_pressed, is_key_down, key_code_to_char, get_key_history_count, get_key_history_event,
    KEY_ENTER, KEY_BACKSPACE, KEY_LEFTSHIFT, KEY_RIGHTSHIFT, KEY_ESC, 
    KEY_L, KEY_P, KEY_S, KEY_H, KEY_U, KEY_C, KEY_M, KEY_T
};

use crate::state::{TERMINAL, COMMAND_HISTORY, COMMAND_HISTORY_COUNT, COMMAND_HISTORY_INDEX};

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
        
        // Handle quick shortcuts only when no character input is being processed
        handle_shortcut_keys();
    }
}

// Separate function to handle special keys
fn handle_special_key(key_code: i32, shift_pressed: bool) -> bool {
    unsafe {
        match key_code {
            KEY_ENTER => {
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
                for j in 0..512 {
                    TERMINAL.command_buffer[j] = 0;
                }
                true
            }
            KEY_UP => {
                if TERMINAL.command_length == 0 {
                    // No command typed, scroll up in output history
                    TERMINAL.scroll_up(3);
                } else if COMMAND_HISTORY_INDEX > 0 {
                    // Command typed, navigate command history
                    COMMAND_HISTORY_INDEX -= 1;
                    // Load command from history
                    TERMINAL.command_length = 0;
                    for j in 0..512 {
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
                    TERMINAL.scroll_down(3);
                } else if COMMAND_HISTORY_INDEX < COMMAND_HISTORY_COUNT {
                    // Command typed, navigate command history
                    COMMAND_HISTORY_INDEX += 1;
                    if COMMAND_HISTORY_INDEX >= COMMAND_HISTORY_COUNT {
                        // Clear command line
                        TERMINAL.command_length = 0;
                        for j in 0..512 {
                            TERMINAL.command_buffer[j] = 0;
                        }
                    } else {
                        // Load command from history
                        TERMINAL.command_length = 0;
                        for j in 0..512 {
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
                TERMINAL.scroll_up(10);
                true
            }
            KEY_PAGEDOWN => {
                TERMINAL.scroll_down(10);
                true
            }
            KEY_HOME => {
                if TERMINAL.command_length == 0 {
                    TERMINAL.scroll_to_top();
                }
                true
            }
            KEY_END => {
                if TERMINAL.command_length == 0 {
                    TERMINAL.scroll_to_bottom();
                }
                true
            }
            _ => false
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
        if TERMINAL.command_length < 510 { // Leave more space for safety
            TERMINAL.command_buffer[TERMINAL.command_length] = ch_byte;
            TERMINAL.command_length += 1;
            // Ensure null termination
            TERMINAL.command_buffer[TERMINAL.command_length] = 0;
        } else {
            // Command too long - auto-clear to prevent issues
            TERMINAL.add_output_line(b"Command too long - cleared");
            TERMINAL.command_length = 0;
            for j in 0..512 {
                TERMINAL.command_buffer[j] = 0;
            }
        }
    }
}

fn handle_shortcut_keys() {
    unsafe {
        // Only handle shortcuts when command line is empty to avoid conflicts
        if TERMINAL.command_length > 0 {
            return;
        }
        
        if is_key_pressed(KEY_L) {
            // Quick 'ls' shortcut
            TERMINAL.command_buffer[0] = b'l';
            TERMINAL.command_buffer[1] = b's';
            TERMINAL.command_length = 2;
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_P) {
            // Quick 'ps' shortcut
            TERMINAL.command_buffer[0] = b'p';
            TERMINAL.command_buffer[1] = b's';
            TERMINAL.command_length = 2;
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_H) {
            // Quick 'help' shortcut
            let help_cmd = b"help";
            for (i, &byte) in help_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = help_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_S) {
            // Quick 'system' shortcut
            let sys_cmd = b"system";
            for (i, &byte) in sys_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = sys_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_U) {
            // Quick 'uname' shortcut
            let uname_cmd = b"uname";
            for (i, &byte) in uname_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = uname_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_C) {
            // Quick 'clear' shortcut
            let clear_cmd = b"clear";
            for (i, &byte) in clear_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = clear_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_M) {
            // Quick 'main' shortcut
            let main_cmd = b"main";
            for (i, &byte) in main_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = main_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_T) {
            // Quick 'theme next' shortcut
            TERMINAL.current_theme = TERMINAL.current_theme.next_theme();
            let theme_name = TERMINAL.current_theme.name();
            
            let mut response = [0u8; 120];
            let prefix = b"Switched to theme: ";
            let mut pos = 0;
            
            for &byte in prefix {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            for &byte in theme_name.as_bytes() {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            
            TERMINAL.add_output_line(&response);
            return;
        }
    }
}
