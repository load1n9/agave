use std::path::Path;
use std::fs::{self, read_dir};
/// Terminal screen types
#[derive(Clone, Copy, PartialEq)]
pub enum Screen {
    Main,
    Files,
    Processes,
    System,
    Help,
}

/// Color theme types
#[derive(Clone, Copy, PartialEq)]
pub enum Theme {
    Default,
    Dark,
    Light,
    Ocean,
    Forest,
    Sunset,
    Neon,
    Retro,
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Default => "Default",
            Theme::Dark => "Dark",
            Theme::Light => "Light",
            Theme::Ocean => "Ocean",
            Theme::Forest => "Forest",
            Theme::Sunset => "Sunset",
            Theme::Neon => "Neon",
            Theme::Retro => "Retro",
        }
    }

    pub fn all_themes() -> [Theme; 8] {
        [
            Theme::Default,
            Theme::Dark,
            Theme::Light,
            Theme::Ocean,
            Theme::Forest,
            Theme::Sunset,
            Theme::Neon,
            Theme::Retro,
        ]
    }

    pub fn next_theme(&self) -> Theme {
        let themes = Self::all_themes();
        let current_index = themes.iter().position(|&t| t == *self).unwrap_or(0);
        themes[(current_index + 1) % themes.len()]
    }

    pub fn prev_theme(&self) -> Theme {
        let themes = Self::all_themes();
        let current_index = themes.iter().position(|&t| t == *self).unwrap_or(0);
        themes[(current_index + themes.len() - 1) % themes.len()]
    }
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
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub struct FileEntry {
    pub name: &'static str,
    pub size: u32,
    pub is_directory: bool,
}

/// Terminal application state
pub struct TerminalApp {
    pub current_screen: Screen,
    pub current_theme: Theme,
    pub command_buffer: [u8; 2048], // Increased from 512 to 2048 for much longer commands
    pub command_length: usize,
    pub output_lines: [[u8; 200]; 2000], // Increased from 120x500 to 200x2000 for much more history and wider lines
    pub output_line_count: usize,
    pub scroll_offset: usize, // Current scroll position (0 = bottom/latest)
    pub uptime: u64,
    // Only keep minimal demo processes
    pub processes: [Process; 4],
    pub process_count: usize,
    pub current_directory: &'static str,
    pub files_scroll_offset: usize, // Scroll offset for file browser
}

impl TerminalApp {
    /// Scroll up in the file list
    pub fn files_scroll_up(&mut self, lines: usize) {
        let path = Path::new(self.current_directory);
        let total_files = match read_dir(path) {
            Ok(read_dir) => read_dir.count(),
            Err(_) => 0,
        };
        let max_scroll = if total_files > 15 {
            total_files - 15
        } else {
            0
        };
        self.files_scroll_offset = (self.files_scroll_offset + lines).min(max_scroll);
    }

    /// Scroll down in the file list
    pub fn files_scroll_down(&mut self, lines: usize) {
        self.files_scroll_offset = self.files_scroll_offset.saturating_sub(lines);
    }

    /// Jump to top of file list
    pub fn files_scroll_to_top(&mut self) {
        let path = Path::new(self.current_directory);
        let total_files = match fs::read_dir(path) {
            Ok(read_dir) => read_dir.count(),
            Err(_) => 0,
        };
        let max_scroll = if total_files > 15 {
            total_files - 15
        } else {
            0
        };
        self.files_scroll_offset = max_scroll;
    }

    /// Jump to bottom of file list
    pub fn files_scroll_to_bottom(&mut self) {
        self.files_scroll_offset = 0;
    }
    pub const fn new() -> Self {
        Self {
            current_screen: Screen::Main,
            current_theme: Theme::Default,
            command_buffer: [0; 2048],
            command_length: 0,
            output_lines: [[0; 200]; 2000],
            output_line_count: 0,
            scroll_offset: 0, // Initialize scroll at bottom
            uptime: 0,
            processes: [
                Process { pid: 1, name: "init", status: "running", memory: 2048 },
                Process { pid: 2, name: "kernel", status: "running", memory: 8192 },
                Process { pid: 3, name: "terminal-app", status: "running", memory: 4096 },
                Process { pid: 4, name: "filesystem", status: "running", memory: 2048 },
            ],
            process_count: 4,
            current_directory: "/home/user",
            files_scroll_offset: 0,
        }
    }

    pub fn version() -> &'static str {
        "Agave v0.1.3"
    }

    pub fn add_output_line(&mut self, text: &[u8]) {
        // When adding new content, reset scroll to bottom to show latest
        self.scroll_offset = 0;

        if self.output_line_count < 2000 {
            let mut line = [0u8; 200];
            let len = text.len().min(199);
            line[..len].copy_from_slice(&text[..len]);
            self.output_lines[self.output_line_count] = line;
            self.output_line_count += 1;
        } else {
            for i in 0..1999 {
                self.output_lines[i] = self.output_lines[i + 1];
            }
            let mut line = [0u8; 200];
            let len = text.len().min(199);
            line[..len].copy_from_slice(&text[..len]);
            self.output_lines[1999] = line;
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

    /// List files in the current directory using std::fs
    pub fn list_files(&mut self) {
        let path = Path::new(self.current_directory);
        match read_dir(path) {
            Ok(entries) => {
                self.add_output_line(b"Files and directories:");
                for entry in entries {
                    if let Ok(entry) = entry {
                        let file_name = entry.file_name();
                        let name_lossy = file_name.to_string_lossy();
                        let name_bytes = name_lossy.as_bytes();
                        let mut line = [0u8; 200];
                        let len = name_bytes.len().min(199);
                        line[..len].copy_from_slice(&name_bytes[..len]);
                        self.add_output_line(&line[..len]);
                    }
                }
            }
            Err(_) => self.add_output_line(b"Error: Could not list directory"),
        }
    }
}
