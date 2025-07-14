use agave_lib::{
    clear_screen, draw_rectangle, fill_rectangle, get_dimensions, get_time_ms, draw_text, Position, RGBA,
    is_key_pressed, is_key_down, key_code_to_char, get_key_history_count, get_key_history_event,
    KEY_ENTER, KEY_BACKSPACE, KEY_LEFTSHIFT, KEY_RIGHTSHIFT, KEY_ESC, KEY_TAB, KEY_A, KEY_L, KEY_P, KEY_S, KEY_H, KEY_U, KEY_C, KEY_M
};

// Terminal application state
static mut TERMINAL: TerminalApp = TerminalApp::new();
static mut LAST_TIME: u64 = 0;
static mut ANIMATION_FRAME: u32 = 0;
static mut CURSOR_BLINK: bool = false;

// Terminal App Structure
struct TerminalApp {
    current_screen: Screen,
    command_buffer: [u8; 256],
    command_length: usize,
    output_lines: [[u8; 80]; 24],
    output_line_count: usize,
    uptime: u64,
    processes: [Process; 8],
    process_count: usize,
    current_directory: &'static str,
    file_system: [FileEntry; 16],
    file_count: usize,
}

#[derive(Clone, Copy, PartialEq)]
enum Screen {
    Main,
    Files,
    Processes,
    System,
    Help,
}

#[derive(Clone, Copy)]
struct Process {
    pid: u32,
    name: &'static str,
    status: &'static str,
    memory: u32,
}

#[derive(Clone, Copy)]
struct FileEntry {
    name: &'static str,
    size: u32,
    is_directory: bool,
}

impl TerminalApp {
    const fn new() -> Self {
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

    fn add_output_line(&mut self, text: &[u8]) {
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
    }

    fn process_command(&mut self) {
        if self.command_length == 0 {
            self.add_output_line(b"");
            return;
        }
        
        // Show command in output (echo)
        let mut prompt_line = [0u8; 80];
        prompt_line[0] = b'$';
        prompt_line[1] = b' ';
        for i in 0..self.command_length.min(76) {
            prompt_line[i + 2] = self.command_buffer[i];
        }
        self.add_output_line(&prompt_line);
        
        // Process the command
        if self.command_length == 2 && self.command_buffer[0] == b'l' && self.command_buffer[1] == b's' {
            self.add_output_line(b"total 8");
            self.add_output_line(b"drwxr-xr-x  2 root  root  4096 Jan  1 00:00 bin");
            self.add_output_line(b"drwxr-xr-x  2 root  root  4096 Jan  1 00:00 usr");
            self.add_output_line(b"drwxr-xr-x  2 root  root  4096 Jan  1 00:00 var");
            self.add_output_line(b"-rw-r--r--  1 root  root  1024 Jan  1 00:00 test.wasm");
            self.add_output_line(b"-rw-r--r--  1 root  root   512 Jan  1 00:00 readme.txt");
            self.current_screen = Screen::Files;
        } else if self.command_length == 2 && self.command_buffer[0] == b'p' && self.command_buffer[1] == b's' {
            self.add_output_line(b"PID  PPID CMD");
            self.add_output_line(b"  1     0 init");
            self.add_output_line(b"  2     1 kernel_thread");
            self.add_output_line(b"  3     1 wasm_runtime");
            self.add_output_line(b"  4     3 terminal_app");
            self.current_screen = Screen::Processes;
        } else if self.command_length == 5 && &self.command_buffer[0..5] == b"uname" {
            self.add_output_line(b"Agave OS 0.1.0 x86_64");
        } else if self.command_length == 6 && &self.command_buffer[0..6] == b"uptime" {
            let uptime_seconds = self.uptime / 1000;
            let hours = uptime_seconds / 3600;
            let minutes = (uptime_seconds % 3600) / 60;
            let seconds = uptime_seconds % 60;
            
            // Format uptime string manually (no std format!)
            let mut uptime_line = [0u8; 80];
            let mut pos = 0;
            
            // "up "
            uptime_line[pos] = b'u'; pos += 1;
            uptime_line[pos] = b'p'; pos += 1;
            uptime_line[pos] = b' '; pos += 1;
            
            // hours
            if hours > 0 {
                if hours >= 10 { uptime_line[pos] = b'0' + (hours / 10) as u8; pos += 1; }
                uptime_line[pos] = b'0' + (hours % 10) as u8; pos += 1;
                uptime_line[pos] = b'h'; pos += 1;
                uptime_line[pos] = b' '; pos += 1;
            }
            
            // minutes
            if minutes > 0 || hours > 0 {
                if minutes >= 10 { uptime_line[pos] = b'0' + (minutes / 10) as u8; pos += 1; }
                uptime_line[pos] = b'0' + (minutes % 10) as u8; pos += 1;
                uptime_line[pos] = b'm'; pos += 1;
                uptime_line[pos] = b' '; pos += 1;
            }
            
            // seconds
            if seconds >= 10 { uptime_line[pos] = b'0' + (seconds / 10) as u8; pos += 1; }
            uptime_line[pos] = b'0' + (seconds % 10) as u8; pos += 1;
            uptime_line[pos] = b's'; pos += 1;
            
            self.add_output_line(&uptime_line);
        } else if self.command_length == 4 && &self.command_buffer[0..4] == b"help" {
            self.add_output_line(b"Available commands:");
            self.add_output_line(b"  ls        - List files and directories");
            self.add_output_line(b"  ps        - List running processes");
            self.add_output_line(b"  system    - Show system information");
            self.add_output_line(b"  uname     - System name and version");
            self.add_output_line(b"  uptime    - System uptime");
            self.add_output_line(b"  clear     - Clear the screen");
            self.add_output_line(b"  main      - Return to main screen");
            self.add_output_line(b"  exit      - Exit the terminal");
            self.add_output_line(b"");
            self.add_output_line(b"Quick shortcuts (when empty):");
            self.add_output_line(b"  L-ls  H-help  S-system  P-ps");
            self.add_output_line(b"  U-uname  C-clear  M-main");
            self.current_screen = Screen::Help;
        } else if self.command_length == 6 && &self.command_buffer[0..6] == b"system" {
            self.add_output_line(b"System Information:");
            self.add_output_line(b"  OS: Agave OS v0.1.0");
            self.add_output_line(b"  Kernel: Rust-based microkernel");
            self.add_output_line(b"  Runtime: WASM execution environment");
            self.add_output_line(b"  Architecture: x86_64");
            self.add_output_line(b"  Memory: 128MB available");
            self.add_output_line(b"  Graphics: VirtIO-GPU framebuffer");
            self.current_screen = Screen::System;
        } else if self.command_length == 4 && &self.command_buffer[0..4] == b"main" {
            self.add_output_line(b"Switching to main screen...");
            self.current_screen = Screen::Main;
        } else if self.command_length == 5 && &self.command_buffer[0..5] == b"clear" {
            self.output_line_count = 0;
            for i in 0..24 {
                for j in 0..80 {
                    self.output_lines[i][j] = 0;
                }
            }
        } else if self.command_length == 4 && &self.command_buffer[0..4] == b"exit" {
            self.add_output_line(b"Goodbye!");
        } else {
            self.add_output_line(b"Command not found. Type 'help' for available commands.");
        }

        // Clear command buffer
        self.command_length = 0;
        for i in 0..256 {
            self.command_buffer[i] = 0;
        }
    }
}

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
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

fn handle_keyboard_input() {
    unsafe {
        let shift_pressed = is_key_down(KEY_LEFTSHIFT) || is_key_down(KEY_RIGHTSHIFT);
        
        // Handle special keys
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
        
        // Process keyboard history for character input
        let history_count = get_key_history_count();
        static mut LAST_PROCESSED_COUNT: i32 = 0;
        
        // Process new keyboard events
        if history_count > LAST_PROCESSED_COUNT {
            for i in LAST_PROCESSED_COUNT..history_count {
                let (key_code, pressed) = get_key_history_event(i);
                
                if pressed {
                    if let Some(ch) = key_code_to_char(key_code, shift_pressed) {
                        // Add character to command buffer
                        if TERMINAL.command_length < 255 {
                            TERMINAL.command_buffer[TERMINAL.command_length] = ch as u8;
                            TERMINAL.command_length += 1;
                        }
                    }
                }
            }
            LAST_PROCESSED_COUNT = history_count;
        }
    }
}

fn draw_terminal() {
    let dim = get_dimensions();
    
    // Clear screen with terminal background
    clear_screen(RGBA::new(12, 12, 12, 255));
    
    unsafe {
        match TERMINAL.current_screen {
            Screen::Main => draw_main_screen(),
            Screen::Files => draw_files_screen(),
            Screen::Processes => draw_processes_screen(),
            Screen::System => draw_system_screen(),
            Screen::Help => draw_help_screen(),
        }
    }
    
    // Draw status bar at bottom
    draw_status_bar(dim);
}

fn draw_main_screen() {
    unsafe {
        // ASCII Art Header
        let header_lines = [
            "    _                         ___  ____",
            "   / \\   __ _  __ ___   _____/ _ \\/ ___|",
            "  / _ \\ / _` |/ _` \\ \\ / / _ \\ | | \\___ \\",
            " / ___ \\ (_| | (_| |\\ V /  __/ |_| |___) |",
            "/_/   \\_\\__, |\\__,_| \\_/ \\___|\\___/|____/",
            "        |___/",
            "",
            "Welcome to Agave OS Terminal",
            "Type 'help' for available commands",
            "",
        ];
        
        for (i, line) in header_lines.iter().enumerate() {
            draw_text(
                Position::new(50, 50 + i as i32 * 20),
                line,
                RGBA::new(0, 255, 100, 255) // Green text
            );
        }
        
        // Terminal output
        let start_y = 250;
        for i in 0..TERMINAL.output_line_count {
            let line_str = core::str::from_utf8(&TERMINAL.output_lines[i]).unwrap_or("???");
            draw_text(
                Position::new(50, start_y + i as i32 * 15),
                line_str.trim_end_matches('\0'),
                RGBA::WHITE
            );
        }
        
        // Command prompt
        let prompt_y = start_y + TERMINAL.output_line_count as i32 * 15 + 20;
        draw_text(
            Position::new(50, prompt_y),
            "user@agave:~$ ",
            RGBA::new(100, 200, 255, 255)
        );
        
        // Current command
        if TERMINAL.command_length > 0 {
            let cmd_str = core::str::from_utf8(&TERMINAL.command_buffer[..TERMINAL.command_length]).unwrap_or("???");
            draw_text(
                Position::new(150, prompt_y),
                cmd_str,
                RGBA::WHITE
            );
        }
        
        // Blinking cursor
        if CURSOR_BLINK {
            let cursor_x = 150 + TERMINAL.command_length as i32 * 8;
            draw_text(
                Position::new(cursor_x, prompt_y),
                "_",
                RGBA::WHITE
            );
        }
        
        // Instructions
        draw_text(
            Position::new(50, prompt_y + 40),
            "Type commands and press Enter. Use single letter shortcuts when empty.",
            RGBA::new(128, 128, 128, 255)
        );
    }
}

fn draw_files_screen() {
    unsafe {
        // Header
        draw_text(
            Position::new(50, 50),
            "File System Browser",
            RGBA::new(255, 255, 0, 255)
        );
        
        draw_text(
            Position::new(50, 80),
            "Current Directory: /home/user",
            RGBA::new(100, 200, 255, 255)
        );
        
        // Column headers
        draw_text(Position::new(50, 120), "Name", RGBA::WHITE);
        draw_text(Position::new(300, 120), "Size", RGBA::WHITE);
        draw_text(Position::new(400, 120), "Type", RGBA::WHITE);
        
        // File listing
        for i in 0..TERMINAL.file_count {
            let file = &TERMINAL.file_system[i];
            let y = 150 + i as i32 * 20;
            
            let color = if file.is_directory {
                RGBA::new(100, 150, 255, 255) // Blue for directories
            } else {
                RGBA::WHITE
            };
            
            draw_text(Position::new(50, y), file.name, color);
            
            if !file.is_directory {
                let size_str = if file.size > 1024 * 1024 {
                    "large"
                } else if file.size > 1024 {
                    "medium"
                } else {
                    "small"
                };
                draw_text(Position::new(300, y), size_str, RGBA::WHITE);
                draw_text(Position::new(400, y), "file", RGBA::WHITE);
            } else {
                draw_text(Position::new(400, y), "dir", RGBA::new(100, 150, 255, 255));
            }
        }
        
        // Instructions
        draw_text(
            Position::new(50, 500),
            "Type 'main' to return to main screen",
            RGBA::new(128, 128, 128, 255)
        );
    }
}

fn draw_processes_screen() {
    unsafe {
        // Header
        draw_text(
            Position::new(50, 50),
            "Process Manager",
            RGBA::new(255, 100, 100, 255)
        );
        
        // Column headers
        draw_text(Position::new(50, 100), "PID", RGBA::WHITE);
        draw_text(Position::new(100, 100), "Name", RGBA::WHITE);
        draw_text(Position::new(250, 100), "Status", RGBA::WHITE);
        draw_text(Position::new(350, 100), "Memory", RGBA::WHITE);
        
        // Process listing
        for i in 0..TERMINAL.process_count {
            let proc = &TERMINAL.processes[i];
            let y = 130 + i as i32 * 25;
            
            // PID
            let pid_str = match proc.pid {
                1 => "1",
                2 => "2", 
                3 => "3",
                4 => "4",
                5 => "5",
                6 => "6",
                7 => "7",
                8 => "8",
                _ => "?",
            };
            draw_text(Position::new(50, y), pid_str, RGBA::WHITE);
            
            // Name
            draw_text(Position::new(100, y), proc.name, RGBA::new(100, 255, 100, 255));
            
            // Status
            let status_color = if proc.status == "running" {
                RGBA::new(0, 255, 0, 255)
            } else {
                RGBA::new(255, 255, 0, 255)
            };
            draw_text(Position::new(250, y), proc.status, status_color);
            
            // Memory usage
            let mem_str = if proc.memory > 4096 {
                "high"
            } else if proc.memory > 1024 {
                "med"
            } else {
                "low"
            };
            draw_text(Position::new(350, y), mem_str, RGBA::WHITE);
        }
        
        // Instructions
        draw_text(
            Position::new(50, 450),
            "Type 'main' to return to main screen",
            RGBA::new(128, 128, 128, 255)
        );
    }
}

fn draw_system_screen() {
    unsafe {
        // Header
        draw_text(
            Position::new(50, 50),
            "System Information",
            RGBA::new(255, 150, 0, 255)
        );
        
        let info_lines = [
            ("OS:", "Agave OS v1.0.0"),
            ("Architecture:", "x86_64"),
            ("Kernel:", "Custom Rust Kernel"),
            ("Runtime:", "WASM + Native"),
            ("Memory:", "100 MB Heap"),
            ("Graphics:", "Direct Framebuffer"),
            ("Input:", "VirtIO Mouse/Keyboard"),
            ("Uptime:", "Running"),
            ("Status:", "Operational"),
        ];
        
        for (i, (label, value)) in info_lines.iter().enumerate() {
            let y = 100 + i as i32 * 25;
            draw_text(Position::new(50, y), label, RGBA::new(200, 200, 200, 255));
            draw_text(Position::new(200, y), value, RGBA::WHITE);
        }
        
        // System stats
        draw_text(
            Position::new(50, 350),
            "Hardware Information:",
            RGBA::new(255, 200, 100, 255)
        );
        
        let hw_lines = [
            "CPU: x86_64 with APIC",
            "Memory: Dynamic allocation",
            "Storage: Virtual disk",
            "Display: 1920x1080 framebuffer",
            "Network: Not available",
        ];
        
        for (i, line) in hw_lines.iter().enumerate() {
            draw_text(
                Position::new(50, 380 + i as i32 * 20),
                line,
                RGBA::WHITE
            );
        }
        
        // Instructions
        draw_text(
            Position::new(50, 520),
            "Type 'main' to return to main screen",
            RGBA::new(128, 128, 128, 255)
        );
    }
}

fn draw_help_screen() {
    // Header
    draw_text(
        Position::new(50, 50),
        "Available Commands",
        RGBA::new(255, 255, 255, 255)
    );
    
    let commands = [
        ("help", "Show this help screen"),
        ("ls", "List files and directories"),
        ("ps", "Show running processes"),
        ("system", "Display system information"),
        ("uname", "Show system name and version"),
        ("uptime", "Display system uptime"),
        ("clear", "Clear terminal output"),
        ("main", "Return to main screen"),
    ];
    
    for (i, (cmd, desc)) in commands.iter().enumerate() {
        let y = 100 + i as i32 * 25;
        draw_text(Position::new(50, y), cmd, RGBA::new(0, 255, 255, 255));
        draw_text(Position::new(150, y), "-", RGBA::WHITE);
        draw_text(Position::new(170, y), desc, RGBA::WHITE);
    }
    
    // Usage instructions
    draw_text(
        Position::new(50, 350),
        "Usage Instructions:",
        RGBA::new(255, 200, 100, 255)
    );
    
    let instructions = [
        "Move your mouse to simulate typing commands",
        "Different mouse positions trigger different commands",
        "Commands are automatically executed after input",
        "Navigate between screens using the available commands",
    ];
    
    for (i, instruction) in instructions.iter().enumerate() {
        draw_text(
            Position::new(50, 380 + i as i32 * 20),
            instruction,
            RGBA::new(200, 200, 200, 255)
        );
    }
    
    // Instructions
    draw_text(
        Position::new(50, 500),
        "Type 'main' to return to main screen",
        RGBA::new(128, 128, 128, 255)
    );
}

fn draw_status_bar(dim: agave_lib::Dimensions) {
    let status_y = dim.height - 30;
    
    // Status bar background
    fill_rectangle(
        Position::new(0, status_y),
        dim.width,
        30,
        RGBA::new(30, 30, 30, 255)
    );
    
    // Status bar border
    draw_rectangle(
        Position::new(0, status_y),
        dim.width,
        30,
        RGBA::new(100, 100, 100, 255)
    );
    
    unsafe {
        // Current screen indicator
        let screen_text = match TERMINAL.current_screen {
            Screen::Main => "[MAIN]",
            Screen::Files => "[FILES]",
            Screen::Processes => "[PROCESSES]", 
            Screen::System => "[SYSTEM]",
            Screen::Help => "[HELP]",
        };
        
        draw_text(
            Position::new(20, status_y + 8),
            screen_text,
            RGBA::new(0, 255, 100, 255)
        );
        
        // System status
        draw_text(
            Position::new(200, status_y + 8),
            "Agave OS Terminal",
            RGBA::WHITE
        );
        
        // Time/uptime indicator
        let uptime_seconds = TERMINAL.uptime / 1000;
        let uptime_str = if uptime_seconds < 60 {
            "< 1 min"
        } else if uptime_seconds < 3600 {
            "< 1 hour"
        } else {
            "running"
        };
        
        draw_text(
            Position::new(dim.width - 150, status_y + 8),
            "Up: running",
            RGBA::new(200, 200, 200, 255)
        );
        
        // Animation indicator
        if ANIMATION_FRAME % 30 < 15 {
            draw_text(
                Position::new(dim.width - 30, status_y + 8),
                "*",
                RGBA::new(0, 255, 0, 255)
            );
        }
    }
}
