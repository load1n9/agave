/// Terminal screen types
#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Main,
    Files,
    Processes,
    System,
    Help,
}

/// Process information
#[derive(Clone, Copy)]
pub struct Process {
    pub pid: u32,
    pub name: &'static str,
    pub status: &'static str,
    pub memory: u32,
}

/// File system entry
#[derive(Clone, Copy)]
pub struct FileEntry {
    pub name: &'static str,
    pub size: u32,
    pub is_directory: bool,
}

/// Terminal application state
pub struct TerminalApp {
    pub current_screen: Screen,
    pub command_buffer: [u8; 512], // Increased from 256 to 512
    pub command_length: usize,
    pub output_lines: [[u8; 120]; 500], // Increased from 80x200 to 120x500 for more history and wider lines
    pub output_line_count: usize,
    pub scroll_offset: usize, // Current scroll position (0 = bottom/latest)
    pub uptime: u64,
    pub processes: [Process; 8],
    pub process_count: usize,
    pub current_directory: &'static str,
    pub file_system: [FileEntry; 16],
    pub file_count: usize,
}

impl TerminalApp {
    pub const fn new() -> Self {
        Self {
            current_screen: Screen::Main,
            command_buffer: [0; 512],
            command_length: 0,
            output_lines: [[0; 120]; 500], // Increased buffer size
            output_line_count: 0,
            scroll_offset: 0, // Initialize scroll at bottom
            uptime: 0,
            processes: [
                Process { pid: 1, name: "init", status: "running", memory: 1024 },
                Process { pid: 2, name: "kernel", status: "running", memory: 4096 },
                Process { pid: 3, name: "virtio-input", status: "running", memory: 512 },
                Process { pid: 4, name: "wasm-runtime", status: "running", memory: 8192 },
                Process { pid: 5, name: "framebuffer", status: "running", memory: 2048 },
                Process { pid: 6, name: "memory-mgr", status: "running", memory: 1536 },
                Process { pid: 7, name: "task-executor", status: "running", memory: 1024 },
                Process { pid: 8, name: "terminal-app", status: "running", memory: 2048 },
            ],
            process_count: 8,
            current_directory: "/home/user",
            file_system: [
                FileEntry { name: "bin", size: 0, is_directory: true },
                FileEntry { name: "etc", size: 0, is_directory: true },
                FileEntry { name: "home", size: 0, is_directory: true },
                FileEntry { name: "usr", size: 0, is_directory: true },
                FileEntry { name: "var", size: 0, is_directory: true },
                FileEntry { name: "tmp", size: 0, is_directory: true },
                FileEntry { name: "hello.wasm", size: 70221, is_directory: false },
                FileEntry { name: "config.json", size: 512, is_directory: false },
                FileEntry { name: "readme.md", size: 1024, is_directory: false },
                FileEntry { name: "system.log", size: 4096, is_directory: false },
                FileEntry { name: "agave-kernel", size: 2048000, is_directory: false },
                FileEntry { name: "bootloader.img", size: 512000, is_directory: false },
                FileEntry { name: ".bashrc", size: 256, is_directory: false },
                FileEntry { name: "documents", size: 0, is_directory: true },
                FileEntry { name: "downloads", size: 0, is_directory: true },
                FileEntry { name: "desktop", size: 0, is_directory: true },
            ],
            file_count: 16,
        }
    }

    pub fn add_output_line(&mut self, text: &[u8]) {
        // When adding new content, reset scroll to bottom to show latest
        self.scroll_offset = 0;
        
        if self.output_line_count < 500 {
            // Still have space in buffer
            let mut line = [0u8; 120];
            let len = text.len().min(119);
            line[..len].copy_from_slice(&text[..len]);
            self.output_lines[self.output_line_count] = line;
            self.output_line_count += 1;
        } else {
            // Buffer is full, scroll up (shift all lines up by 1)
            for i in 0..499 {
                self.output_lines[i] = self.output_lines[i + 1];
            }
            // Add new line at the end
            let mut line = [0u8; 120];
            let len = text.len().min(119);
            line[..len].copy_from_slice(&text[..len]);
            self.output_lines[499] = line;
            // Keep count at 500 (buffer is full)
        }
    }
    
    /// Scroll up in the output history
    pub fn scroll_up(&mut self, lines: usize) {
        let max_scroll = if self.output_line_count > 15 {
            self.output_line_count - 15 // Keep at least 15 lines visible
        } else {
            0
        };
        self.scroll_offset = (self.scroll_offset + lines).min(max_scroll);
    }
    
    /// Scroll down in the output history
    pub fn scroll_down(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }
    
    /// Jump to top of history
    pub fn scroll_to_top(&mut self) {
        let max_scroll = if self.output_line_count > 15 {
            self.output_line_count - 15
        } else {
            0
        };
        self.scroll_offset = max_scroll;
    }
    
    /// Jump to bottom of history (latest output)
    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = 0;
    }
}
