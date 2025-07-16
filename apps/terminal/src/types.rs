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
    pub processes: [Process; 32], // Increased from 8 to 32 processes
    pub process_count: usize,
    #[allow(dead_code)]
    pub current_directory: &'static str,
    pub file_system: [FileEntry; 80], // Increased from 16 to 80 file entries to accommodate all entries
    pub file_count: usize,
}

impl TerminalApp {
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
                Process {
                    pid: 1,
                    name: "init",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 2,
                    name: "kernel",
                    status: "running",
                    memory: 8192,
                },
                Process {
                    pid: 3,
                    name: "virtio-input",
                    status: "running",
                    memory: 1024,
                },
                Process {
                    pid: 4,
                    name: "wasm-runtime",
                    status: "running",
                    memory: 16384,
                },
                Process {
                    pid: 5,
                    name: "framebuffer",
                    status: "running",
                    memory: 4096,
                },
                Process {
                    pid: 6,
                    name: "memory-mgr",
                    status: "running",
                    memory: 3072,
                },
                Process {
                    pid: 7,
                    name: "task-executor",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 8,
                    name: "terminal-app",
                    status: "running",
                    memory: 4096,
                },
                Process {
                    pid: 9,
                    name: "filesystem",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 10,
                    name: "network-stack",
                    status: "running",
                    memory: 3072,
                },
                Process {
                    pid: 11,
                    name: "audio-driver",
                    status: "running",
                    memory: 1536,
                },
                Process {
                    pid: 12,
                    name: "graphics-accel",
                    status: "running",
                    memory: 6144,
                },
                Process {
                    pid: 13,
                    name: "security-mgr",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 14,
                    name: "power-mgr",
                    status: "running",
                    memory: 1024,
                },
                Process {
                    pid: 15,
                    name: "interrupt-hdl",
                    status: "running",
                    memory: 512,
                },
                Process {
                    pid: 16,
                    name: "scheduler",
                    status: "running",
                    memory: 1536,
                },
                Process {
                    pid: 17,
                    name: "device-mgr",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 18,
                    name: "io-subsystem",
                    status: "running",
                    memory: 1024,
                },
                Process {
                    pid: 19,
                    name: "cache-mgr",
                    status: "running",
                    memory: 4096,
                },
                Process {
                    pid: 20,
                    name: "crypto-engine",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 21,
                    name: "vm-manager",
                    status: "running",
                    memory: 8192,
                },
                Process {
                    pid: 22,
                    name: "backup-daemon",
                    status: "idle",
                    memory: 1024,
                },
                Process {
                    pid: 23,
                    name: "log-collector",
                    status: "running",
                    memory: 1536,
                },
                Process {
                    pid: 24,
                    name: "perf-monitor",
                    status: "running",
                    memory: 1024,
                },
                Process {
                    pid: 25,
                    name: "user-session",
                    status: "running",
                    memory: 2048,
                },
                Process {
                    pid: 26,
                    name: "service-mgr",
                    status: "running",
                    memory: 1536,
                },
                Process {
                    pid: 27,
                    name: "event-loop",
                    status: "running",
                    memory: 1024,
                },
                Process {
                    pid: 28,
                    name: "debug-agent",
                    status: "idle",
                    memory: 2048,
                },
                Process {
                    pid: 29,
                    name: "thermal-ctrl",
                    status: "running",
                    memory: 512,
                },
                Process {
                    pid: 30,
                    name: "watchdog",
                    status: "running",
                    memory: 256,
                },
                Process {
                    pid: 31,
                    name: "profiler",
                    status: "idle",
                    memory: 1024,
                },
                Process {
                    pid: 32,
                    name: "health-check",
                    status: "running",
                    memory: 768,
                },
            ],
            process_count: 32,
            current_directory: "/home/user",
            file_system: [
                FileEntry {
                    name: "bin",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "etc",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "home",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "usr",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "var",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "tmp",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "opt",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "lib",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "dev",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "proc",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "sys",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "boot",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "media",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "mnt",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "srv",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "root",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "hello.wasm",
                    size: 70221,
                    is_directory: false,
                },
                FileEntry {
                    name: "config.json",
                    size: 512,
                    is_directory: false,
                },
                FileEntry {
                    name: "readme.md",
                    size: 1024,
                    is_directory: false,
                },
                FileEntry {
                    name: "system.log",
                    size: 4096,
                    is_directory: false,
                },
                FileEntry {
                    name: "agave-kernel",
                    size: 2048000,
                    is_directory: false,
                },
                FileEntry {
                    name: "bootloader.img",
                    size: 512000,
                    is_directory: false,
                },
                FileEntry {
                    name: ".bashrc",
                    size: 256,
                    is_directory: false,
                },
                FileEntry {
                    name: ".profile",
                    size: 512,
                    is_directory: false,
                },
                FileEntry {
                    name: "environment",
                    size: 1024,
                    is_directory: false,
                },
                FileEntry {
                    name: "motd",
                    size: 256,
                    is_directory: false,
                },
                FileEntry {
                    name: "hosts",
                    size: 128,
                    is_directory: false,
                },
                FileEntry {
                    name: "passwd",
                    size: 1024,
                    is_directory: false,
                },
                FileEntry {
                    name: "group",
                    size: 512,
                    is_directory: false,
                },
                FileEntry {
                    name: "fstab",
                    size: 256,
                    is_directory: false,
                },
                FileEntry {
                    name: "sudoers",
                    size: 512,
                    is_directory: false,
                },
                FileEntry {
                    name: "crontab",
                    size: 256,
                    is_directory: false,
                },
                FileEntry {
                    name: "locale.conf",
                    size: 128,
                    is_directory: false,
                },
                FileEntry {
                    name: "timezone",
                    size: 64,
                    is_directory: false,
                },
                FileEntry {
                    name: "documents",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "downloads",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "desktop",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "pictures",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "music",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "videos",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "templates",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "public",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "applications",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "games",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "logs",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "backup",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "cache",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "runtime",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "scripts",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "tools",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "workspace",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "projects",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "archives",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "fonts",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "themes",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "plugins",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "extensions",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "packages",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "libs",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "includes",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "resources",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "assets",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "data",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "config",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "settings",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "preferences",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "profiles",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "sessions",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "local",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "shared",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "security",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "network",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "system",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "drivers",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "modules",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "firmware",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "hardware",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "software",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "memory",
                    size: 0,
                    is_directory: true,
                },
                FileEntry {
                    name: "storage",
                    size: 0,
                    is_directory: true,
                },
            ],
            file_count: 80,
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
}
