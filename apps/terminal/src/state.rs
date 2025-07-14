// Global state management
use crate::types::TerminalApp;

// Terminal application state
pub static mut TERMINAL: TerminalApp = TerminalApp::new();
pub static mut LAST_TIME: u64 = 0;
pub static mut ANIMATION_FRAME: u32 = 0;
pub static mut CURSOR_BLINK: bool = false;

// Command history
pub static mut COMMAND_HISTORY: [[u8; 256]; 10] = [[0; 256]; 10];
pub static mut COMMAND_HISTORY_COUNT: usize = 0;
pub static mut COMMAND_HISTORY_INDEX: usize = 0;
