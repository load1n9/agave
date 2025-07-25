// Global state management
use crate::types::TerminalApp;

// Terminal application state
pub static mut TERMINAL: TerminalApp = TerminalApp::new();
pub static mut LAST_TIME: u64 = 0;
pub static mut ANIMATION_FRAME: u32 = 0;
pub static mut CURSOR_BLINK: bool = false;

// Command history
pub static mut COMMAND_HISTORY: [[u8; 2048]; 100] = [[0; 2048]; 100];
pub static mut COMMAND_HISTORY_COUNT: usize = 0;
pub static mut COMMAND_HISTORY_INDEX: usize = 0;
