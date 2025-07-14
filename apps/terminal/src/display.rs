use agave_lib::{
    clear_screen, draw_rectangle, fill_rectangle, get_dimensions, draw_text, Position, RGBA
};

use crate::types::{Screen, TerminalApp};
use crate::state::{TERMINAL, CURSOR_BLINK, ANIMATION_FRAME};

pub fn draw_terminal() {
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
            "Welcome to Agave OS Terminal v1.0",
            "A WASM-based operating system",
            "",
        ];
        
        for (i, line) in header_lines.iter().enumerate() {
            draw_text(
                Position::new(50, 50 + i as i32 * 20),
                line,
                RGBA::new(0, 255, 150, 255) // Bright green text
            );
        }
        
        // Display current status
        draw_text(
            Position::new(50, 270),
            "System Status: Online",
            RGBA::new(0, 255, 0, 255)
        );
        
        let uptime_seconds = TERMINAL.uptime / 1000;
        let uptime_text = if uptime_seconds < 60 {
            "Uptime: < 1 minute"
        } else if uptime_seconds < 3600 {
            "Uptime: < 1 hour"
        } else {
            "Uptime: > 1 hour"
        };
        
        draw_text(
            Position::new(50, 290),
            uptime_text,
            RGBA::new(200, 200, 200, 255)
        );
        
        // Terminal output
        let start_y = 330;
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
            RGBA::new(100, 200, 255, 255) // Light blue prompt
        );
        
        // Current command
        if TERMINAL.command_length > 0 {
            let cmd_str = core::str::from_utf8(&TERMINAL.command_buffer[..TERMINAL.command_length]).unwrap_or("???");
            draw_text(
                Position::new(160, prompt_y),
                cmd_str,
                RGBA::WHITE
            );
        }
        
        // Blinking cursor
        if CURSOR_BLINK {
            let cursor_x = 160 + TERMINAL.command_length as i32 * 8;
            draw_text(
                Position::new(cursor_x, prompt_y),
                "█",
                RGBA::new(255, 255, 255, 200) // Semi-transparent cursor
            );
        }
        
        // Instructions
        draw_text(
            Position::new(50, prompt_y + 40),
            "Type commands and press Enter. Use shortcuts when command line is empty.",
            RGBA::new(128, 128, 128, 255)
        );
        
        draw_text(
            Position::new(50, prompt_y + 60),
            "Try: 'help', 'ls', 'ps', 'system', 'uname', 'uptime', 'clear'",
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
            "Press Enter to return to main screen",
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
            "Press Enter to return to main screen",
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
            "Press Enter to return to main screen",
            RGBA::new(128, 128, 128, 255)
        );
    }
}

fn draw_help_screen() {
    // Header
    draw_text(
        Position::new(50, 50),
        "Agave OS Terminal - Help",
        RGBA::new(255, 255, 100, 255) // Yellow header
    );
    
    draw_text(
        Position::new(50, 80),
        "Command Reference",
        RGBA::new(200, 200, 200, 255)
    );
    
    // Command categories
    draw_text(
        Position::new(50, 120),
        "File System Commands:",
        RGBA::new(100, 255, 100, 255) // Green category
    );
    
    let file_commands = [
        ("ls", "List files and directories"),
    ];
    
    for (i, (cmd, desc)) in file_commands.iter().enumerate() {
        let y = 145 + i as i32 * 20;
        draw_text(Position::new(70, y), cmd, RGBA::new(0, 255, 255, 255));
        draw_text(Position::new(120, y), "-", RGBA::WHITE);
        draw_text(Position::new(140, y), desc, RGBA::WHITE);
    }
    
    draw_text(
        Position::new(50, 180),
        "System Commands:",
        RGBA::new(100, 255, 100, 255)
    );
    
    let system_commands = [
        ("ps", "Show running processes"),
        ("system", "Display system information"),
        ("uname", "Show system name and version"),
        ("uptime", "Display system uptime"),
        ("whoami", "Show current user"),
        ("date", "Show current date/time"),
    ];
    
    for (i, (cmd, desc)) in system_commands.iter().enumerate() {
        let y = 205 + i as i32 * 20;
        draw_text(Position::new(70, y), cmd, RGBA::new(0, 255, 255, 255));
        draw_text(Position::new(130, y), "-", RGBA::WHITE);
        draw_text(Position::new(150, y), desc, RGBA::WHITE);
    }
    
    draw_text(
        Position::new(50, 330),
        "Utility Commands:",
        RGBA::new(100, 255, 100, 255)
    );
    
    let utility_commands = [
        ("help", "Show this help screen"),
        ("clear", "Clear terminal output"),
        ("reset", "Reset terminal and clear memory"),
        ("echo", "Echo text to output"),
        ("main", "Return to main screen"),
        ("exit", "Exit the terminal"),
    ];
    
    for (i, (cmd, desc)) in utility_commands.iter().enumerate() {
        let y = 355 + i as i32 * 20;
        draw_text(Position::new(70, y), cmd, RGBA::new(0, 255, 255, 255));
        draw_text(Position::new(120, y), "-", RGBA::WHITE);
        draw_text(Position::new(140, y), desc, RGBA::WHITE);
    }
    
    // Quick shortcuts
    draw_text(
        Position::new(50, 460),
        "Quick Shortcuts (when command line is empty):",
        RGBA::new(255, 200, 100, 255)
    );
    
    let shortcuts = [
        "L - ls command    H - help command    S - system command",
        "P - ps command    U - uname command   C - clear command",
        "M - main screen",
    ];
    
    for (i, shortcut) in shortcuts.iter().enumerate() {
        draw_text(
            Position::new(70, 485 + i as i32 * 20),
            shortcut,
            RGBA::new(200, 200, 200, 255)
        );
    }
    
    // Instructions
    draw_text(
        Position::new(50, 560),
        "Press Enter to return to main screen",
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
        RGBA::new(25, 25, 25, 255) // Slightly lighter background
    );
    
    // Status bar border
    draw_rectangle(
        Position::new(0, status_y),
        dim.width,
        30,
        RGBA::new(80, 80, 80, 255) // Lighter border
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
            RGBA::new(0, 255, 150, 255) // Bright green
        );
        
        // System status
        draw_text(
            Position::new(150, status_y + 8),
            "Agave OS Terminal v1.0",
            RGBA::WHITE
        );
        
        // Command buffer indicator
        let cmd_indicator = if TERMINAL.command_length > 0 {
            "TYPING"
        } else {
            "READY"
        };
        
        let cmd_color = if TERMINAL.command_length > 0 {
            RGBA::new(255, 255, 0, 255) // Yellow when typing
        } else {
            RGBA::new(0, 255, 0, 255) // Green when ready
        };
        
        draw_text(
            Position::new(400, status_y + 8),
            cmd_indicator,
            cmd_color
        );
        
        // Time/uptime indicator
        let uptime_seconds = TERMINAL.uptime / 1000;
        let uptime_str = if uptime_seconds < 60 {
            "< 1m"
        } else if uptime_seconds < 3600 {
            "< 1h"
        } else {
            "> 1h"
        };
        
        draw_text(
            Position::new(dim.width - 120, status_y + 8),
            "Uptime:",
            RGBA::new(180, 180, 180, 255)
        );
        
        draw_text(
            Position::new(dim.width - 60, status_y + 8),
            uptime_str,
            RGBA::WHITE
        );
        
        // Animation indicator (heartbeat)
        if ANIMATION_FRAME % 60 < 30 {
            draw_text(
                Position::new(dim.width - 20, status_y + 8),
                "♥",
                RGBA::new(255, 100, 100, 255) // Red heart
            );
        }
    }
}
