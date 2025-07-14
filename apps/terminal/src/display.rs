use agave_lib::{
    clear_screen, draw_rectangle, fill_rectangle, get_dimensions, draw_text, Position, RGBA
};

use crate::types::Screen;
use crate::state::{TERMINAL, CURSOR_BLINK, ANIMATION_FRAME};

// Enhanced color palette
const BG_PRIMARY: RGBA = RGBA::new(16, 20, 24, 255);      // Dark blue-gray background
const BG_SECONDARY: RGBA = RGBA::new(24, 30, 36, 255);    // Slightly lighter background
const BG_ACCENT: RGBA = RGBA::new(32, 40, 48, 255);       // Card/panel background
const BORDER_COLOR: RGBA = RGBA::new(64, 80, 96, 255);    // Subtle borders
const TEXT_PRIMARY: RGBA = RGBA::new(220, 225, 230, 255); // Primary text
const TEXT_SECONDARY: RGBA = RGBA::new(160, 170, 180, 255); // Secondary text
const TEXT_MUTED: RGBA = RGBA::new(120, 130, 140, 255);   // Muted text
const ACCENT_GREEN: RGBA = RGBA::new(72, 187, 120, 255);  // Success/active green
const ACCENT_BLUE: RGBA = RGBA::new(96, 165, 250, 255);   // Info blue
const ACCENT_YELLOW: RGBA = RGBA::new(251, 191, 36, 255); // Warning yellow
const ACCENT_RED: RGBA = RGBA::new(248, 113, 113, 255);   // Error red
const ACCENT_PURPLE: RGBA = RGBA::new(168, 85, 247, 255); // Highlight purple
const ACCENT_CYAN: RGBA = RGBA::new(34, 211, 238, 255);   // Bright cyan

pub fn draw_terminal() {
    let dim = get_dimensions();
    
    // Clear screen with enhanced background
    clear_screen(BG_PRIMARY);
    
    // Draw subtle gradient background
    draw_background_gradient(dim);
    
    unsafe {
        match TERMINAL.current_screen {
            Screen::Main => draw_main_screen(dim),
            Screen::Files => draw_files_screen(dim),
            Screen::Processes => draw_processes_screen(dim),
            Screen::System => draw_system_screen(dim),
            Screen::Help => draw_help_screen(dim),
        }
    }
    
    // Draw enhanced status bar at bottom
    draw_status_bar(dim);
}

fn draw_background_gradient(dim: agave_lib::Dimensions) {
    // Draw subtle vertical gradient for visual depth
    for y in 0..dim.height/4 {
        let alpha = 10 + (y * 15 / (dim.height/4));
        fill_rectangle(
            Position::new(0, y),
            dim.width,
            1,
            RGBA::new(32, 48, 64, alpha as i32)
        );
    }
}

fn draw_main_screen(dim: agave_lib::Dimensions) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);
    
    unsafe {
        // Draw main content card with subtle background
        draw_card(Position::new(margin - 20, 40), content_width + 40, dim.height - 140);
        
        // ASCII Art Header with better spacing
        let header_lines = [
            "    _                         ___  ____",
            "   / \\   __ _  __ ___   _____/ _ \\/ ___|",
            "  / _ \\ / _` |/ _` \\ \\ / / _ \\ | | \\___ \\",
            " / ___ \\ (_| | (_| |\\ V /  __/ |_| |___) |",
            "/_/   \\_\\__, |\\__,_| \\_/ \\___|\\___/|____/",
            "        |___/",
        ];
        
        for (i, line) in header_lines.iter().enumerate() {
            draw_text(
                Position::new(margin + 20, 70 + i as i32 * 22),
                line,
                ACCENT_CYAN
            );
        }
        
        // Welcome text with better typography
        draw_text(
            Position::new(margin + 20, 210),
            "Welcome to Agave OS Terminal",
            TEXT_PRIMARY
        );
        draw_text(
            Position::new(margin + 20, 235),
            "A WASM-based operating system",
            TEXT_SECONDARY
        );
        
        // Status section with visual separation
        draw_section_divider(Position::new(margin, 270), content_width);
        
        draw_text(
            Position::new(margin + 20, 295),
            "● System Status:",
            ACCENT_GREEN
        );
        draw_text(
            Position::new(margin + 160, 295),
            "Online",
            ACCENT_GREEN
        );
        
        let uptime_seconds = TERMINAL.uptime / 1000;
        let uptime_text = if uptime_seconds < 60 {
            "● Uptime: < 1 minute"
        } else if uptime_seconds < 3600 {
            "● Uptime: < 1 hour"
        } else {
            "● Uptime: > 1 hour"
        };
        
        draw_text(
            Position::new(margin + 20, 320),
            uptime_text,
            TEXT_SECONDARY
        );
        
        // Terminal output section with scroll support
        draw_section_divider(Position::new(margin, 355), content_width);
        
        let output_header_y = 380;
        
        // Show scroll indicators if needed
        if TERMINAL.scroll_offset > 0 {
            draw_text(
                Position::new(margin + 20, output_header_y),
                "Terminal Output: (Scrolled - Use ↑↓ PgUp/PgDn to navigate)",
                ACCENT_YELLOW
            );
        } else {
            draw_text(
                Position::new(margin + 20, output_header_y),
                "Terminal Output:",
                TEXT_PRIMARY
            );
        }
        
        // Calculate which lines to display based on scroll offset
        let output_start_y = 410;
        let max_display_lines = 15; // Maximum lines to show in terminal output area
        
        let total_lines = TERMINAL.output_line_count;
        let scroll_offset = TERMINAL.scroll_offset;
        
        // Calculate the range of lines to display
        let display_start = if total_lines > max_display_lines {
            if scroll_offset >= total_lines - max_display_lines {
                0 // Show from beginning if scrolled too far
            } else {
                total_lines - max_display_lines - scroll_offset
            }
        } else {
            0
        };
        
        let display_end = if total_lines > max_display_lines {
            if scroll_offset == 0 {
                total_lines // Show latest lines when not scrolled
            } else {
                (total_lines - scroll_offset).min(total_lines)
            }
        } else {
            total_lines
        };
        
        // Render the visible lines
        let mut line_count = 0;
        for i in display_start..display_end {
            if line_count >= max_display_lines {
                break;
            }
            
            let line_str = core::str::from_utf8(&TERMINAL.output_lines[i]).unwrap_or("???");
            let trimmed_line = line_str.trim_end_matches('\0');
            
            if !trimmed_line.is_empty() {
                draw_text(
                    Position::new(margin + 30, output_start_y + line_count as i32 * 18),
                    trimmed_line,
                    TEXT_PRIMARY
                );
            }
            line_count += 1;
        }
        
        // Show scroll position indicator
        if total_lines > max_display_lines {
            let scroll_indicator_x = margin + content_width - 60;
            let scroll_indicator_y = output_start_y;
            
            // Draw scroll track
            draw_rectangle(
                Position::new(scroll_indicator_x, scroll_indicator_y),
                8,
                max_display_lines as i32 * 18,
                BORDER_COLOR
            );
            
            // Calculate scroll thumb position
            let track_height = max_display_lines as i32 * 18 - 20;
            let scroll_progress = if total_lines > max_display_lines {
                1.0 - (scroll_offset as f32 / (total_lines - max_display_lines) as f32)
            } else {
                1.0
            };
            let thumb_y = scroll_indicator_y + 10 + (track_height as f32 * scroll_progress) as i32;
            
            // Draw scroll thumb
            fill_rectangle(
                Position::new(scroll_indicator_x + 1, thumb_y),
                6,
                10,
                ACCENT_CYAN
            );
        }
        
        // Command prompt section with enhanced styling
        let prompt_y = output_start_y + max_display_lines as i32 * 18 + 30;
        
        // Draw prompt background
        fill_rectangle(
            Position::new(margin + 15, prompt_y - 5),
            content_width - 30,
            30,
            BG_ACCENT
        );
        draw_rectangle(
            Position::new(margin + 15, prompt_y - 5),
            content_width - 30,
            30,
            BORDER_COLOR
        );
        
        draw_text(
            Position::new(margin + 25, prompt_y + 5),
            "user@agave:~$",
            ACCENT_BLUE
        );
        
        // Current command
        if TERMINAL.command_length > 0 {
            let cmd_str = core::str::from_utf8(&TERMINAL.command_buffer[..TERMINAL.command_length]).unwrap_or("???");
            draw_text(
                Position::new(margin + 140, prompt_y + 5),
                cmd_str,
                TEXT_PRIMARY
            );
        }
        
        // Enhanced blinking cursor
        if CURSOR_BLINK {
            let cursor_x = margin + 140 + TERMINAL.command_length as i32 * 8;
            draw_text(
                Position::new(cursor_x, prompt_y + 5),
                "▊",
                ACCENT_CYAN
            );
        }
        

    }
}

fn draw_files_screen(dim: agave_lib::Dimensions) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);
    
    unsafe {
        // Draw main content card
        draw_card(Position::new(margin - 20, 40), content_width + 40, dim.height - 140);
        
        // Header with icon
        draw_text(
            Position::new(margin + 20, 70),
            "📁 File System Browser",
            ACCENT_BLUE
        );
        
        draw_text(
            Position::new(margin + 20, 100),
            "Current Directory:",
            TEXT_SECONDARY
        );
        draw_text(
            Position::new(margin + 180, 100),
            "/home/user",
            ACCENT_CYAN
        );
        
        // Table header with better styling
        draw_section_divider(Position::new(margin, 130), content_width);
        
        fill_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            BG_ACCENT
        );
        
        draw_text(Position::new(margin + 25, 155), "Name", ACCENT_PURPLE);
        draw_text(Position::new(margin + 320, 155), "Size", ACCENT_PURPLE);
        draw_text(Position::new(margin + 420, 155), "Type", ACCENT_PURPLE);
        
        draw_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            BORDER_COLOR
        );
        
        // File listing with alternating backgrounds
        for i in 0..TERMINAL.file_count {
            let file = &TERMINAL.file_system[i];
            let y = 190 + i as i32 * 30;
            
            // Alternating row backgrounds
            if i % 2 == 0 {
                fill_rectangle(
                    Position::new(margin + 10, y - 5),
                    content_width - 20,
                    30,
                    RGBA::new(24, 30, 36, 128)
                );
            }
            
            let (icon, name_color) = if file.is_directory {
                ("📂", ACCENT_BLUE)
            } else {
                ("📄", TEXT_PRIMARY)
            };
            
            draw_text(Position::new(margin + 25, y + 5), icon, TEXT_PRIMARY);
            draw_text(Position::new(margin + 50, y + 5), file.name, name_color);
            
            if !file.is_directory {
                let size_str = if file.size > 1024 * 1024 {
                    "Large"
                } else if file.size > 1024 {
                    "Medium"
                } else {
                    "Small"
                };
                draw_text(Position::new(margin + 320, y + 5), size_str, TEXT_SECONDARY);
                draw_text(Position::new(margin + 420, y + 5), "File", TEXT_SECONDARY);
            } else {
                draw_text(Position::new(margin + 420, y + 5), "Directory", ACCENT_BLUE);
            }
        }
        
        // Instructions
        draw_section_divider(Position::new(margin, dim.height - 120), content_width);
        draw_text(
            Position::new(margin + 20, dim.height - 95),
            "⏎ Press Enter to return to main screen",
            TEXT_MUTED
        );
    }
}

fn draw_processes_screen(dim: agave_lib::Dimensions) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);
    
    unsafe {
        // Draw main content card
        draw_card(Position::new(margin - 20, 40), content_width + 40, dim.height - 140);
        
        // Header with icon
        draw_text(
            Position::new(margin + 20, 70),
            "⚙️ Process Manager",
            ACCENT_RED
        );
        
        // Performance indicator
        draw_text(
            Position::new(margin + 20, 100),
            "System Load:",
            TEXT_SECONDARY
        );
        draw_text(
            Position::new(margin + 130, 100),
            "Normal",
            ACCENT_GREEN
        );
        
        // Table header
        draw_section_divider(Position::new(margin, 130), content_width);
        
        fill_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            BG_ACCENT
        );
        
        draw_text(Position::new(margin + 25, 155), "PID", ACCENT_PURPLE);
        draw_text(Position::new(margin + 80, 155), "Name", ACCENT_PURPLE);
        draw_text(Position::new(margin + 250, 155), "Status", ACCENT_PURPLE);
        draw_text(Position::new(margin + 350, 155), "Memory", ACCENT_PURPLE);
        
        draw_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            BORDER_COLOR
        );
        
        // Process listing with enhanced styling
        for i in 0..TERMINAL.process_count {
            let proc = &TERMINAL.processes[i];
            let y = 190 + i as i32 * 32;
            
            // Alternating row backgrounds
            if i % 2 == 0 {
                fill_rectangle(
                    Position::new(margin + 10, y - 5),
                    content_width - 20,
                    32,
                    RGBA::new(24, 30, 36, 128)
                );
            }
            
            // PID with leading zeros
            let pid_str = match proc.pid {
                1 => "001",
                2 => "002", 
                3 => "003",
                4 => "004",
                5 => "005",
                6 => "006",
                7 => "007",
                8 => "008",
                _ => "???",
            };
            draw_text(Position::new(margin + 25, y + 5), pid_str, TEXT_SECONDARY);
            
            // Process name with icon
            let proc_icon = match proc.name {
                "init" => "🚀",
                "kernel" => "🔧",
                "virtio-input" => "⌨️",
                "wasm-runtime" => "📦",
                "framebuffer" => "🖥️",
                "memory-mgr" => "💾",
                "task-executor" => "⚡",
                "terminal-app" => "💻",
                _ => "⚙️",
            };
            
            draw_text(Position::new(margin + 80, y + 5), proc_icon, TEXT_PRIMARY);
            draw_text(Position::new(margin + 105, y + 5), proc.name, ACCENT_CYAN);
            
            // Status with colored indicator
            let (status_color, status_icon) = if proc.status == "running" {
                (ACCENT_GREEN, "●")
            } else {
                (ACCENT_YELLOW, "⏸")
            };
            
            draw_text(Position::new(margin + 250, y + 5), status_icon, status_color);
            draw_text(Position::new(margin + 270, y + 5), proc.status, status_color);
            
            // Memory usage with bar visualization
            let mem_str = if proc.memory > 4096 {
                "High"
            } else if proc.memory > 1024 {
                "Med"
            } else {
                "Low"
            };
            
            let mem_color = if proc.memory > 4096 {
                ACCENT_RED
            } else if proc.memory > 1024 {
                ACCENT_YELLOW
            } else {
                ACCENT_GREEN
            };
            
            draw_text(Position::new(margin + 350, y + 5), mem_str, mem_color);
            
            // Memory usage bar
            let bar_width = (proc.memory / 100).min(60) as i32;
            fill_rectangle(
                Position::new(margin + 390, y + 10),
                bar_width,
                6,
                mem_color
            );
            draw_rectangle(
                Position::new(margin + 390, y + 10),
                60,
                6,
                BORDER_COLOR
            );
        }
        
        // Instructions
        draw_section_divider(Position::new(margin, dim.height - 120), content_width);
        draw_text(
            Position::new(margin + 20, dim.height - 95),
            "⏎ Press Enter to return to main screen",
            TEXT_MUTED
        );
    }
}

fn draw_system_screen(dim: agave_lib::Dimensions) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);
    
    unsafe {
        // Draw main content card
        draw_card(Position::new(margin - 20, 40), content_width + 40, dim.height - 140);
        
        // Header with icon
        draw_text(
            Position::new(margin + 20, 70),
            "🖥️ System Information",
            ACCENT_YELLOW
        );
        
        // System overview section
        draw_section_header(Position::new(margin + 20, 110), "Operating System");
        
        let os_info = [
            ("OS:", "Agave OS v1.0.0", ACCENT_CYAN),
            ("Architecture:", "x86_64", TEXT_PRIMARY),
            ("Kernel:", "Custom Rust Kernel", ACCENT_PURPLE),
            ("Runtime:", "WASM + Native", ACCENT_GREEN),
        ];
        
        for (i, (label, value, color)) in os_info.iter().enumerate() {
            let y = 140 + i as i32 * 25;
            draw_text(Position::new(margin + 40, y), label, TEXT_SECONDARY);
            draw_text(Position::new(margin + 200, y), value, *color);
        }
        
        // Hardware section
        draw_section_header(Position::new(margin + 20, 250), "Hardware Resources");
        
        let hw_info = [
            ("Memory:", "100 MB Heap", ACCENT_BLUE),
            ("Graphics:", "Direct Framebuffer", ACCENT_GREEN),
            ("Input:", "VirtIO Mouse/Keyboard", TEXT_PRIMARY),
            ("Storage:", "Virtual Disk", TEXT_PRIMARY),
        ];
        
        for (i, (label, value, color)) in hw_info.iter().enumerate() {
            let y = 280 + i as i32 * 25;
            draw_text(Position::new(margin + 40, y), label, TEXT_SECONDARY);
            draw_text(Position::new(margin + 200, y), value, *color);
        }
        
        // Status indicators
        draw_section_header(Position::new(margin + 20, 390), "System Status");
        
        let status_items = [
            ("Uptime:", "Running", ACCENT_GREEN, "✓"),
            ("Status:", "Operational", ACCENT_GREEN, "✓"),
            ("Load:", "Normal", ACCENT_YELLOW, "●"),
            ("Network:", "Not Available", ACCENT_RED, "✗"),
        ];
        
        for (i, (label, value, color, icon)) in status_items.iter().enumerate() {
            let y = 420 + i as i32 * 25;
            draw_text(Position::new(margin + 40, y), label, TEXT_SECONDARY);
            draw_text(Position::new(margin + 150, y), icon, *color);
            draw_text(Position::new(margin + 180, y), value, *color);
        }
        
        // Instructions
        draw_section_divider(Position::new(margin, dim.height - 120), content_width);
        draw_text(
            Position::new(margin + 20, dim.height - 95),
            "⏎ Press Enter to return to main screen",
            TEXT_MUTED
        );
    }
}

fn draw_help_screen(dim: agave_lib::Dimensions) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);
    
    // Draw main content card
    draw_card(Position::new(margin - 20, 40), content_width + 40, dim.height - 140);
    
    // Header with icon
    draw_text(
        Position::new(margin + 20, 70),
        "❓ Agave OS Terminal - Help",
        ACCENT_YELLOW
    );
    
    draw_text(
        Position::new(margin + 20, 100),
        "Command Reference Guide",
        TEXT_SECONDARY
    );
    
    let col1_x = margin + 30;
    let col2_x = margin + 350;
    let mut current_y = 140;
    
    // File System Commands
    draw_section_header(Position::new(col1_x, current_y), "📁 File System");
    current_y += 30;
    
    let file_commands = [
        ("ls", "List files and directories"),
    ];
    
    for (cmd, desc) in file_commands.iter() {
        draw_text(Position::new(col1_x + 20, current_y), cmd, ACCENT_CYAN);
        draw_text(Position::new(col1_x + 60, current_y), "-", TEXT_MUTED);
        draw_text(Position::new(col1_x + 80, current_y), desc, TEXT_PRIMARY);
        current_y += 22;
    }
    
    current_y += 10;
    
    // System Commands
    draw_section_header(Position::new(col1_x, current_y), "⚙️ System Commands");
    current_y += 30;
    
    let system_commands = [
        ("ps", "Show running processes"),
        ("system", "Display system information"),
        ("uname", "Show system name and version"),
        ("uptime", "Display system uptime"),
        ("whoami", "Show current user"),
        ("date", "Show current date/time"),
    ];
    
    for (cmd, desc) in system_commands.iter() {
        draw_text(Position::new(col1_x + 20, current_y), cmd, ACCENT_CYAN);
        draw_text(Position::new(col1_x + 80, current_y), "-", TEXT_MUTED);
        draw_text(Position::new(col1_x + 100, current_y), desc, TEXT_PRIMARY);
        current_y += 22;
    }
    
    // Right column - Utility Commands
    current_y = 170;
    draw_section_header(Position::new(col2_x, current_y), "🔧 Utility Commands");
    current_y += 30;
    
    let utility_commands = [
        ("help", "Show this help screen"),
        ("clear", "Clear terminal output"),
        ("reset", "Reset terminal state"),
        ("echo", "Echo text to output"),
        ("main", "Return to main screen"),
        ("exit", "Exit the terminal"),
    ];
    
    for (cmd, desc) in utility_commands.iter() {
        draw_text(Position::new(col2_x + 20, current_y), cmd, ACCENT_CYAN);
        draw_text(Position::new(col2_x + 70, current_y), "-", TEXT_MUTED);
        draw_text(Position::new(col2_x + 90, current_y), desc, TEXT_PRIMARY);
        current_y += 22;
    }
    
    current_y += 20;
    
    // Keyboard shortcuts section
    draw_section_header(Position::new(col2_x, current_y), "⌨️ Quick Shortcuts");
    current_y += 25;
    
    draw_text(
        Position::new(col2_x + 10, current_y),
        "(when command line is empty)",
        TEXT_MUTED
    );
    current_y += 25;
    
    let shortcuts = [
        ("L", "ls command"),
        ("H", "help command"),
        ("S", "system command"),
        ("P", "ps command"),
        ("U", "uname command"),
        ("C", "clear command"),
        ("M", "main screen"),
    ];
    
    for (key, action) in shortcuts.iter() {
        draw_text(Position::new(col2_x + 20, current_y), key, ACCENT_PURPLE);
        draw_text(Position::new(col2_x + 40, current_y), "-", TEXT_MUTED);
        draw_text(Position::new(col2_x + 60, current_y), action, TEXT_SECONDARY);
        current_y += 20;
    }
    
    // Instructions
    draw_section_divider(Position::new(margin, dim.height - 120), content_width);
    draw_text(
        Position::new(margin + 20, dim.height - 95),
        "⏎ Press Enter to return to main screen",
        TEXT_MUTED
    );
}

fn draw_status_bar(dim: agave_lib::Dimensions) {
    let status_y = dim.height - 40;
    let status_height = 40;
    
    // Enhanced status bar background with gradient effect
    fill_rectangle(
        Position::new(0, status_y),
        dim.width,
        status_height,
        BG_SECONDARY
    );
    
    // Top border with accent color
    fill_rectangle(
        Position::new(0, status_y),
        dim.width,
        2,
        ACCENT_CYAN
    );
    
    // Subtle inner border
    draw_rectangle(
        Position::new(0, status_y),
        dim.width,
        status_height,
        BORDER_COLOR
    );
    
    unsafe {
        // Current screen indicator with enhanced styling
        let (screen_text, screen_color) = match TERMINAL.current_screen {
            Screen::Main => ("● MAIN", ACCENT_GREEN),
            Screen::Files => ("📁 FILES", ACCENT_BLUE),
            Screen::Processes => ("⚙️ PROCESSES", ACCENT_RED), 
            Screen::System => ("🖥️ SYSTEM", ACCENT_YELLOW),
            Screen::Help => ("❓ HELP", ACCENT_PURPLE),
        };
        
        draw_text(
            Position::new(25, status_y + 12),
            screen_text,
            screen_color
        );
        
        // Separator
        draw_text(
            Position::new(160, status_y + 12),
            "│",
            BORDER_COLOR
        );
        
        // Application title
        draw_text(
            Position::new(180, status_y + 12),
            "Agave OS Terminal v1.0",
            TEXT_PRIMARY
        );
        
        // Separator
        draw_text(
            Position::new(380, status_y + 12),
            "│",
            BORDER_COLOR
        );
        
        // Command buffer indicator with better styling
        let (cmd_text, cmd_color) = if TERMINAL.command_length > 0 {
            ("⌨️ TYPING", ACCENT_YELLOW)
        } else {
            ("✓ READY", ACCENT_GREEN)
        };
        
        draw_text(
            Position::new(400, status_y + 12),
            cmd_text,
            cmd_color
        );
        
        // Right side indicators
        let right_x = dim.width - 200;
        
        // Uptime indicator
        let uptime_seconds = TERMINAL.uptime / 1000;
        let uptime_str = if uptime_seconds < 60 {
            "⏱️ < 1m"
        } else if uptime_seconds < 3600 {
            "⏱️ < 1h"
        } else {
            "⏱️ > 1h"
        };
        
        draw_text(
            Position::new(right_x, status_y + 12),
            uptime_str,
            TEXT_SECONDARY
        );
        
        // Separator
        draw_text(
            Position::new(right_x + 80, status_y + 12),
            "│",
            BORDER_COLOR
        );
        
        // System heartbeat indicator
        let heartbeat_color = if ANIMATION_FRAME % 120 < 20 {
            ACCENT_RED
        } else if ANIMATION_FRAME % 120 < 40 {
            RGBA::new(248, 113, 113, 180)
        } else {
            RGBA::new(248, 113, 113, 80)
        };
        
        draw_text(
            Position::new(right_x + 100, status_y + 12),
            "♥",
            heartbeat_color
        );
        
        // System status
        draw_text(
            Position::new(right_x + 125, status_y + 12),
            "Online",
            ACCENT_GREEN
        );
    }
}

// Helper functions for drawing UI components
// IMPORTANT: These functions maintain the visual consistency of the terminal UI
// Do not modify spacing, colors, or layout without updating the design system above

fn draw_card(pos: Position, width: i32, height: i32) {
    // Card background with consistent styling
    fill_rectangle(pos, width, height, BG_ACCENT);
    
    // Card border for definition
    draw_rectangle(pos, width, height, BORDER_COLOR);
    
    // Subtle inner shadow effect for depth
    draw_rectangle(
        Position::new(pos.x + 1, pos.y + 1),
        width - 2,
        height - 2,
        RGBA::new(255, 255, 255, 10)
    );
}

fn draw_section_divider(pos: Position, width: i32) {
    // Main divider line with consistent styling
    fill_rectangle(pos, width, 1, BORDER_COLOR);
    
    // Subtle highlight above for depth
    fill_rectangle(
        Position::new(pos.x, pos.y - 1),
        width,
        1,
        RGBA::new(255, 255, 255, 20)
    );
}

fn draw_section_header(pos: Position, text: &str) {
    // Section header with professional styling
    draw_text(pos, text, ACCENT_PURPLE);
    
    // Underline for emphasis (maintain 8px character width assumption)
    let text_width = text.len() as i32 * 8;
    fill_rectangle(
        Position::new(pos.x, pos.y + 18),
        text_width,
        2,
        ACCENT_PURPLE
    );
}
