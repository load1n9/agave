use agave_lib::{
    is_key_pressed, is_key_down, key_code_to_char, get_key_history_count, get_key_history_event,
    KEY_ENTER, KEY_BACKSPACE, KEY_LEFTSHIFT, KEY_RIGHTSHIFT, KEY_ESC, 
    KEY_A, KEY_L, KEY_P, KEY_S, KEY_H, KEY_U, KEY_C, KEY_M
};

use crate::types::TerminalApp;
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
                    // Check for Enter key in history
                    if key_code == KEY_ENTER {
                        TERMINAL.process_command();
                        LAST_PROCESSED_COUNT = history_count;
                        return;
                    }
                    
                    // Check for Backspace in history
                    if key_code == KEY_BACKSPACE {
                        if TERMINAL.command_length > 0 {
                            TERMINAL.command_length -= 1;
                            TERMINAL.command_buffer[TERMINAL.command_length] = 0;
                        }
                        continue;
                    }
                    
                    // Check for Escape in history
                    if key_code == KEY_ESC {
                        TERMINAL.command_length = 0;
                        for j in 0..256 {
                            TERMINAL.command_buffer[j] = 0;
                        }
                        continue;
                    }
                    
                    // Check for Up arrow - scroll history or command history
                    if key_code == KEY_UP {
                        if TERMINAL.command_length == 0 {
                            // No command typed, scroll up in output history
                            TERMINAL.scroll_up(3);
                        } else if COMMAND_HISTORY_INDEX > 0 {
                            // Command typed, navigate command history
                            COMMAND_HISTORY_INDEX -= 1;
                            // Load command from history
                            TERMINAL.command_length = 0;
                            for j in 0..256 {
                                if COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j] == 0 {
                                    break;
                                }
                                TERMINAL.command_buffer[j] = COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j];
                                TERMINAL.command_length += 1;
                            }
                        }
                        continue;
                    }
                    
                    // Check for Down arrow - scroll history or command history
                    if key_code == KEY_DOWN {
                        if TERMINAL.command_length == 0 {
                            // No command typed, scroll down in output history
                            TERMINAL.scroll_down(3);
                        } else if COMMAND_HISTORY_INDEX < COMMAND_HISTORY_COUNT {
                            // Command typed, navigate command history
                            COMMAND_HISTORY_INDEX += 1;
                            if COMMAND_HISTORY_INDEX >= COMMAND_HISTORY_COUNT {
                                // Clear command line
                                TERMINAL.command_length = 0;
                                for j in 0..256 {
                                    TERMINAL.command_buffer[j] = 0;
                                }
                            } else {
                                // Load command from history
                                TERMINAL.command_length = 0;
                                for j in 0..256 {
                                    if COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j] == 0 {
                                        break;
                                    }
                                    TERMINAL.command_buffer[j] = COMMAND_HISTORY[COMMAND_HISTORY_INDEX][j];
                                    TERMINAL.command_length += 1;
                                }
                            }
                        }
                        continue;
                    }
                    
                    // Check for Page Up - scroll up faster
                    if key_code == KEY_PAGEUP {
                        TERMINAL.scroll_up(10);
                        continue;
                    }
                    
                    // Check for Page Down - scroll down faster  
                    if key_code == KEY_PAGEDOWN {
                        TERMINAL.scroll_down(10);
                        continue;
                    }
                    
                    // Check for Home - jump to top of history
                    if key_code == KEY_HOME && TERMINAL.command_length == 0 {
                        TERMINAL.scroll_to_top();
                        continue;
                    }
                    
                    // Check for End - jump to bottom of history
                    if key_code == KEY_END && TERMINAL.command_length == 0 {
                        TERMINAL.scroll_to_bottom();
                        continue;
                    }
                    
                    // Handle character input
                    if let Some(ch) = key_code_to_char(key_code, shift_pressed) {
                        // Add character to command buffer with safety checks
                        if TERMINAL.command_length < 250 { // Leave some buffer space
                            TERMINAL.command_buffer[TERMINAL.command_length] = ch as u8;
                            TERMINAL.command_length += 1;
                        } else {
                            // Command too long - auto-clear to prevent issues
                            TERMINAL.add_output_line(b"Command too long - cleared");
                            TERMINAL.command_length = 0;
                            for j in 0..256 {
                                TERMINAL.command_buffer[j] = 0;
                            }
                        }
                    }
                }
            }
            LAST_PROCESSED_COUNT = history_count;
        }
        
        // Handle special keys using is_key_pressed as backup
        if is_key_pressed(KEY_ENTER) {
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_BACKSPACE) {
            if TERMINAL.command_length > 0 {
                TERMINAL.command_length -= 1;
                TERMINAL.command_buffer[TERMINAL.command_length] = 0;
            }
            return;
        }
        
        if is_key_pressed(KEY_ESC) {
            // Clear command line
            TERMINAL.command_length = 0;
            for i in 0..256 {
                TERMINAL.command_buffer[i] = 0;
            }
            return;
        }
        
        // Handle quick shortcuts
        handle_shortcut_keys();
    }
}

fn handle_shortcut_keys() {
    unsafe {
        if is_key_pressed(KEY_L) && TERMINAL.command_length == 0 {
            // Quick 'ls' shortcut
            TERMINAL.command_buffer[0] = b'l';
            TERMINAL.command_buffer[1] = b's';
            TERMINAL.command_length = 2;
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_P) && TERMINAL.command_length == 0 {
            // Quick 'ps' shortcut
            TERMINAL.command_buffer[0] = b'p';
            TERMINAL.command_buffer[1] = b's';
            TERMINAL.command_length = 2;
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_H) && TERMINAL.command_length == 0 {
            // Quick 'help' shortcut
            let help_cmd = b"help";
            for (i, &byte) in help_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = help_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_S) && TERMINAL.command_length == 0 {
            // Quick 'system' shortcut
            let sys_cmd = b"system";
            for (i, &byte) in sys_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = sys_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_U) && TERMINAL.command_length == 0 {
            // Quick 'uname' shortcut
            let uname_cmd = b"uname";
            for (i, &byte) in uname_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = uname_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_C) && TERMINAL.command_length == 0 {
            // Quick 'clear' shortcut
            let clear_cmd = b"clear";
            for (i, &byte) in clear_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = clear_cmd.len();
            TERMINAL.process_command();
            return;
        }
        
        if is_key_pressed(KEY_M) && TERMINAL.command_length == 0 {
            // Quick 'main' shortcut
            let main_cmd = b"main";
            for (i, &byte) in main_cmd.iter().enumerate() {
                TERMINAL.command_buffer[i] = byte;
            }
            TERMINAL.command_length = main_cmd.len();
            TERMINAL.process_command();
            return;
        }
    }
}
