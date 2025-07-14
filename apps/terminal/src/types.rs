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
    pub command_buffer: [u8; 256],
    pub command_length: usize,
    pub output_lines: [[u8; 80]; 24],
    pub output_line_count: usize,
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
            command_buffer: [0; 256],
            command_length: 0,
            output_lines: [[0; 80]; 24],
            output_line_count: 0,
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
        if self.output_line_count < 24 {
            let mut line = [0u8; 80];
            let len = text.len().min(79);
            line[..len].copy_from_slice(&text[..len]);
            self.output_lines[self.output_line_count] = line;
            self.output_line_count += 1;
        } else {
            // Scroll up
            for i in 0..23 {
                self.output_lines[i] = self.output_lines[i + 1];
            }
            let mut line = [0u8; 80];
            let len = text.len().min(79);
            line[..len].copy_from_slice(&text[..len]);
            self.output_lines[23] = line;
        }
        
        // Auto-clear output when it gets too long to prevent memory issues
        // Keep only the last 20 lines when we have 24 lines
        if self.output_line_count >= 24 {
            // Every so often, clear some old content to free up memory
            static mut CLEAR_COUNTER: u32 = 0;
            unsafe {
                CLEAR_COUNTER += 1;
                if CLEAR_COUNTER >= 50 { // After 50 scroll operations, do a partial clear
                    // Keep only the last 15 lines
                    for i in 0..15 {
                        self.output_lines[i] = self.output_lines[i + 9];
                    }
                    // Clear the rest
                    for i in 15..24 {
                        self.output_lines[i] = [0u8; 80];
                    }
                    self.output_line_count = 15;
                    CLEAR_COUNTER = 0;
                }
            }
        }
    }
}
