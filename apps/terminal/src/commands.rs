use crate::state::{COMMAND_HISTORY, COMMAND_HISTORY_COUNT, COMMAND_HISTORY_INDEX};
use crate::themes::get_theme_description;
use crate::types::{Screen, TerminalApp, Theme};

impl TerminalApp {
    pub fn process_command(&mut self) {
        if self.command_length == 0 {
            // If we're not on the main screen, pressing Enter returns to main
            if self.current_screen != Screen::Main {
                self.add_output_line(b"Returning to main screen...");
                self.current_screen = Screen::Main;
            } else {
                self.add_output_line(b"");
            }
            return;
        }

        // Add command to history
        unsafe {
            if COMMAND_HISTORY_COUNT < 100 {
                for i in 0..self.command_length {
                    COMMAND_HISTORY[COMMAND_HISTORY_COUNT][i] = self.command_buffer[i];
                }
                for i in self.command_length..2048 {
                    COMMAND_HISTORY[COMMAND_HISTORY_COUNT][i] = 0;
                }
                COMMAND_HISTORY_COUNT += 1;
            } else {
                // Shift history up and add new command at the end
                for i in 0..99 {
                    COMMAND_HISTORY[i] = COMMAND_HISTORY[i + 1];
                }
                for i in 0..self.command_length {
                    COMMAND_HISTORY[99][i] = self.command_buffer[i];
                }
                for i in self.command_length..2048 {
                    COMMAND_HISTORY[99][i] = 0;
                }
            }
            COMMAND_HISTORY_INDEX = COMMAND_HISTORY_COUNT;
        }

        // Show command in output (echo)
        let mut prompt_line = [0u8; 200];
        prompt_line[0] = b'$';
        prompt_line[1] = b' ';
        for i in 0..self.command_length.min(196) {
            prompt_line[i + 2] = self.command_buffer[i];
        }
        self.add_output_line(&prompt_line);

        // Convert command to lowercase for case-insensitive matching
        let mut cmd_lower = [0u8; 2048];
        for i in 0..self.command_length {
            cmd_lower[i] = match self.command_buffer[i] {
                b'A'..=b'Z' => self.command_buffer[i] + 32, // Convert to lowercase
                _ => self.command_buffer[i],
            };
        }

        // Process the command
        if self.command_length == 2 && &cmd_lower[0..2] == b"ls" {
            self.handle_ls_command();
        } else if self.command_length == 2 && &cmd_lower[0..2] == b"ps" {
            self.handle_ps_command();
        } else if self.command_length == 5 && &cmd_lower[0..5] == b"uname" {
            self.add_output_line(b"Agave OS 0.1.0 x86_64");
        } else if self.command_length == 6 && &cmd_lower[0..6] == b"uptime" {
            self.handle_uptime_command();
        } else if self.command_length == 4 && &cmd_lower[0..4] == b"help" {
            self.handle_help_command();
        } else if self.command_length == 6 && &cmd_lower[0..6] == b"system" {
            self.handle_system_command();
        } else if self.command_length == 4 && &cmd_lower[0..4] == b"main" {
            self.add_output_line(b"Returning to main screen...");
            self.current_screen = Screen::Main;
        } else if self.command_length == 5 && &cmd_lower[0..5] == b"clear" {
            self.handle_clear_command();
        } else if self.command_length == 5 && &cmd_lower[0..5] == b"reset" {
            self.handle_reset_command();
        } else if self.command_length == 6 && &cmd_lower[0..6] == b"whoami" {
            self.add_output_line(b"user");
        } else if self.command_length == 4 && &cmd_lower[0..4] == b"date" {
            self.add_output_line(b"Mon Jan  1 00:00:00 UTC 2024");
        } else if self.command_length >= 4 && &cmd_lower[0..4] == b"echo" {
            self.handle_echo_command();
        } else if self.command_length == 4 && &cmd_lower[0..4] == b"exit" {
            self.add_output_line(b"Goodbye! Thanks for using Agave OS Terminal.");
            self.add_output_line(b"Session terminated.");
        } else if self.command_length >= 5 && &cmd_lower[0..5] == b"theme" {
            self.handle_theme_command();
        } else if self.command_length == 6 && &cmd_lower[0..6] == b"health" {
            self.handle_health_command();
        } else if self.command_length == 5 && &cmd_lower[0..5] == b"power" {
            self.handle_power_command();
        } else if self.command_length == 8 && &cmd_lower[0..8] == b"security" {
            self.handle_security_command();
        } else if self.command_length == 8 && &cmd_lower[0..8] == b"features" {
            self.handle_features_command();
        } else {
            self.add_output_line(b"Command not found. Type 'help' for available commands.");
        }

        // Clear command buffer
        self.command_length = 0;
        for i in 0..2048 {
            self.command_buffer[i] = 0;
        }
    }

    fn handle_ls_command(&mut self) {
        self.add_output_line(b"Files and directories:");
        self.add_output_line(b"drwxr-xr-x  bin/");
        self.add_output_line(b"drwxr-xr-x  etc/");
        self.add_output_line(b"drwxr-xr-x  home/");
        self.add_output_line(b"drwxr-xr-x  usr/");
        self.add_output_line(b"drwxr-xr-x  var/");
        self.add_output_line(b"drwxr-xr-x  tmp/");
        self.add_output_line(b"-rw-r--r--  hello.wasm");
        self.add_output_line(b"-rw-r--r--  config.json");
        self.add_output_line(b"-rw-r--r--  readme.md");
        self.add_output_line(b"-rw-r--r--  system.log");
        self.current_screen = Screen::Files;
    }

    fn handle_ps_command(&mut self) {
        self.add_output_line(b"Running processes:");
        self.add_output_line(b"PID  PPID CMD");
        self.add_output_line(b"  1     0 init");
        self.add_output_line(b"  2     1 kernel_thread");
        self.add_output_line(b"  3     1 wasm_runtime");
        self.add_output_line(b"  4     3 terminal_app");
        self.current_screen = Screen::Processes;
    }

    fn handle_uptime_command(&mut self) {
        let uptime_seconds = self.uptime / 1000;
        let hours = uptime_seconds / 3600;
        let minutes = (uptime_seconds % 3600) / 60;
        let seconds = uptime_seconds % 60;

        // Format uptime string manually (no std format!)
        let mut uptime_line = [0u8; 80];
        let mut pos = 0;

        // "up "
        uptime_line[pos] = b'u';
        pos += 1;
        uptime_line[pos] = b'p';
        pos += 1;
        uptime_line[pos] = b' ';
        pos += 1;

        // hours
        if hours > 0 {
            if hours >= 10 {
                uptime_line[pos] = b'0' + (hours / 10) as u8;
                pos += 1;
            }
            uptime_line[pos] = b'0' + (hours % 10) as u8;
            pos += 1;
            uptime_line[pos] = b'h';
            pos += 1;
            uptime_line[pos] = b' ';
            pos += 1;
        }

        // minutes
        if minutes > 0 || hours > 0 {
            if minutes >= 10 {
                uptime_line[pos] = b'0' + (minutes / 10) as u8;
                pos += 1;
            }
            uptime_line[pos] = b'0' + (minutes % 10) as u8;
            pos += 1;
            uptime_line[pos] = b'm';
            pos += 1;
            uptime_line[pos] = b' ';
            pos += 1;
        }

        // seconds
        if seconds >= 10 {
            uptime_line[pos] = b'0' + (seconds / 10) as u8;
            pos += 1;
        }
        uptime_line[pos] = b'0' + (seconds % 10) as u8;
        pos += 1;
        uptime_line[pos] = b's';

        self.add_output_line(&uptime_line);
    }

    fn handle_help_command(&mut self) {
        self.add_output_line(b"Available commands:");
        self.add_output_line(b"  ls        - List files and directories");
        self.add_output_line(b"  ps        - List running processes");
        self.add_output_line(b"  system    - Show system information");
        self.add_output_line(b"  uname     - System name and version");
        self.add_output_line(b"  uptime    - System uptime");
        self.add_output_line(b"  clear     - Clear the screen");
        self.add_output_line(b"  reset     - Reset terminal and clear memory");
        self.add_output_line(b"  main      - Return to main screen");
        self.add_output_line(b"  whoami    - Show current user");
        self.add_output_line(b"  date      - Show current date/time");
        self.add_output_line(b"  echo      - Echo text");
        self.add_output_line(b"  theme     - Change color themes");
        self.add_output_line(b"  exit      - Exit the terminal");
        self.add_output_line(b"");
        self.add_output_line(b"Theme commands:");
        self.add_output_line(b"  theme list    - List all available themes");
        self.add_output_line(b"  theme <name>  - Switch to specific theme");
        self.add_output_line(b"  theme next    - Switch to next theme");
        self.add_output_line(b"  theme prev    - Switch to previous theme");
        self.add_output_line(b"");
        self.add_output_line(b"Quick shortcuts (when command line is empty):");
        self.add_output_line(b"  L-ls  H-help  S-system  P-ps");
        self.add_output_line(b"  U-uname  C-clear  M-main  T-theme next");
        self.current_screen = Screen::Help;
    }

    fn handle_system_command(&mut self) {
        self.add_output_line(b"=== Agave OS System Information ===");
        self.add_output_line(b"  OS: Agave OS v1.0.0 (Enhanced)");
        self.add_output_line(b"  Kernel: Rust-based microkernel");
        self.add_output_line(b"  Runtime: WASM execution environment");
        self.add_output_line(b"  Architecture: x86_64");
        self.add_output_line(b"  Memory: 128MB available");
        self.add_output_line(b"  Graphics: VirtIO-GPU framebuffer");
        self.add_output_line(b"");
        self.add_output_line(b"Enhanced Features:");
        self.add_output_line(b"  + Real-time diagnostics");
        self.add_output_line(b"  + Multi-priority process scheduling");
        self.add_output_line(b"  + Security framework & sandboxing");
        self.add_output_line(b"  + Power management & thermal control");
        self.add_output_line(b"  + Enhanced networking (TCP/UDP/HTTP)");
        self.add_output_line(b"  + Virtual filesystem");
        self.add_output_line(b"  + Extended command history (100 commands)"); // New feature
        self.add_output_line(b"  + Large output buffer (2000 lines x 200 chars)"); // New feature
        self.add_output_line(b"");
        self.add_output_line(b"Commands: 'health', 'power', 'security', 'features'");
        self.current_screen = Screen::System;
    }

    fn handle_clear_command(&mut self) {
        self.output_line_count = 0;
        for i in 0..2000 {
            for j in 0..200 {
                self.output_lines[i][j] = 0;
            }
        }
        // Also clear any memory cleanup counter
        unsafe {
            static mut CLEAR_COUNTER: u32 = 0;
            CLEAR_COUNTER = 0;
        }
    }

    fn handle_reset_command(&mut self) {
        // Reset command - clears everything and resets to initial state
        self.output_line_count = 0;
        for i in 0..2000 {
            for j in 0..200 {
                self.output_lines[i][j] = 0;
            }
        }
        self.current_screen = Screen::Main;
        unsafe {
            // Clear command history
            COMMAND_HISTORY_COUNT = 0;
            COMMAND_HISTORY_INDEX = 0;
            for i in 0..100 {
                for j in 0..2048 {
                    COMMAND_HISTORY[i][j] = 0;
                }
            }
        }
        self.add_output_line(b"Terminal reset - memory cleared");
    }

    fn handle_echo_command(&mut self) {
        // Echo command - output everything after "echo "
        if self.command_length > 5 {
            let mut echo_line = [0u8; 160]; // Increased buffer size for longer echo output
            let start = 5; // Skip "echo "
            let len = (self.command_length - start).min(159);
            for i in 0..len {
                echo_line[i] = self.command_buffer[start + i];
            }
            self.add_output_line(&echo_line);
        } else {
            self.add_output_line(b"");
        }
    }

    fn handle_theme_command(&mut self) {
        // Handle theme command - can be "theme", "theme list", "theme <name>", or "theme next/prev"
        if self.command_length == 5 {
            // Just "theme" - show current theme
            let current_name = self.current_theme.name();
            let mut response = [0u8; 200];

            // Build response string manually
            let prefix = b"Current theme: ";
            let desc_text = get_theme_description(self.current_theme);

            let mut pos = 0;
            // Copy prefix
            for &byte in prefix {
                if pos < 199 {
                    response[pos] = byte;
                    pos += 1;
                }
            }
            // Copy theme name
            for &byte in current_name.as_bytes() {
                if pos < 199 {
                    response[pos] = byte;
                    pos += 1;
                }
            }

            self.add_output_line(&response);

            // Add description
            let mut desc_line = [0u8; 200];
            let desc_bytes = desc_text.as_bytes();
            let desc_len = desc_bytes.len().min(199);
            desc_line[..desc_len].copy_from_slice(&desc_bytes[..desc_len]);
            self.add_output_line(&desc_line);

            self.add_output_line(b"Use 'theme list' to see all themes");
            self.add_output_line(b"Use 'theme <name>' to switch themes");
            self.add_output_line(b"Use 'theme next' or 'theme prev' to cycle");
        } else if self.command_length >= 10 && &self.command_buffer[6..10] == b"list" {
            // "theme list" - show all available themes
            self.add_output_line(b"Available themes:");
            self.add_output_line(b"");

            for theme in Theme::all_themes().iter() {
                let mut line = [0u8; 200];
                let name = theme.name();
                let desc = get_theme_description(*theme);

                let mut pos = 0;
                // Add indicator for current theme
                if *theme == self.current_theme {
                    line[pos] = b'*';
                    pos += 1;
                    line[pos] = b' ';
                    pos += 1;
                } else {
                    line[pos] = b' ';
                    pos += 1;
                    line[pos] = b' ';
                    pos += 1;
                }

                // Add theme name
                for &byte in name.as_bytes() {
                    if pos < 199 {
                        line[pos] = byte;
                        pos += 1;
                    }
                }

                // Add spacing
                while pos < 20 && pos < 199 {
                    line[pos] = b' ';
                    pos += 1;
                }

                // Add description
                for &byte in desc.as_bytes() {
                    if pos < 199 {
                        line[pos] = byte;
                        pos += 1;
                    }
                }

                self.add_output_line(&line);
            }
        } else if self.command_length >= 10 && &self.command_buffer[6..10] == b"next" {
            // "theme next" - switch to next theme
            self.current_theme = self.current_theme.next_theme();
            let new_name = self.current_theme.name();

            let mut response = [0u8; 200];
            let prefix = b"Switched to theme: ";
            let mut pos = 0;

            for &byte in prefix {
                if pos < 199 {
                    response[pos] = byte;
                    pos += 1;
                }
            }
            for &byte in new_name.as_bytes() {
                if pos < 199 {
                    response[pos] = byte;
                    pos += 1;
                }
            }

            self.add_output_line(&response);
        } else if self.command_length >= 10 && &self.command_buffer[6..10] == b"prev" {
            // "theme prev" - switch to previous theme
            self.current_theme = self.current_theme.prev_theme();
            let new_name = self.current_theme.name();

            let mut response = [0u8; 200];
            let prefix = b"Switched to theme: ";
            let mut pos = 0;

            for &byte in prefix {
                if pos < 199 {
                    response[pos] = byte;
                    pos += 1;
                }
            }
            for &byte in new_name.as_bytes() {
                if pos < 199 {
                    response[pos] = byte;
                    pos += 1;
                }
            }

            self.add_output_line(&response);
        } else if self.command_length > 6 {
            // "theme <name>" - switch to specific theme
            let theme_name_start = 6;
            let theme_name_len = self.command_length - theme_name_start;

            // Extract theme name and convert to lowercase for comparison
            let mut theme_name = [0u8; 64];
            for i in 0..theme_name_len.min(63) {
                let ch = self.command_buffer[theme_name_start + i];
                theme_name[i] = match ch {
                    b'A'..=b'Z' => ch + 32, // Convert to lowercase
                    _ => ch,
                };
            }

            // Try to match theme name
            let mut found_theme = None;
            for theme in Theme::all_themes().iter() {
                let theme_name_bytes = theme.name().to_lowercase();
                let theme_name_lower = theme_name_bytes.as_bytes();

                if theme_name_len == theme_name_lower.len()
                    && &theme_name[..theme_name_len] == theme_name_lower
                {
                    found_theme = Some(*theme);
                    break;
                }
            }

            if let Some(new_theme) = found_theme {
                self.current_theme = new_theme;
                let new_name = new_theme.name();

                let mut response = [0u8; 200];
                let prefix = b"Switched to theme: ";
                let mut pos = 0;

                for &byte in prefix {
                    if pos < 199 {
                        response[pos] = byte;
                        pos += 1;
                    }
                }
                for &byte in new_name.as_bytes() {
                    if pos < 199 {
                        response[pos] = byte;
                        pos += 1;
                    }
                }

                self.add_output_line(&response);
            } else {
                self.add_output_line(b"Unknown theme. Use 'theme list' to see available themes.");
            }
        } else {
            self.add_output_line(b"Usage: theme [list|next|prev|<name>]");
        }
    }

    fn handle_health_command(&mut self) {
        self.add_output_line(b"=== System Health Status ===");
        self.add_output_line(b"Overall Status: Healthy");
        self.add_output_line(b"Memory Usage: Normal (35.8%)");
        self.add_output_line(b"CPU Temperature: 42.1C");
        self.add_output_line(b"Power State: Active");
        self.add_output_line(b"Security: No threats detected");
        self.add_output_line(b"Process Count: 32 active");
        self.add_output_line(b"Uptime: 15.3 minutes");
        self.add_output_line(b"Diagnostics: All systems operational");
        self.add_output_line(b"Network: Connected");
        self.add_output_line(b"Storage: Available");
        self.add_output_line(b"Command Buffer: 2048 bytes available");
        self.add_output_line(b"Output History: 2000 lines capacity");
    }

    fn handle_power_command(&mut self) {
        self.add_output_line(b"=== Power Management Status ===");
        self.add_output_line(b"Current State: Active");
        self.add_output_line(b"CPU Frequency: 2400 MHz");
        self.add_output_line(b"Power Policy: Balanced");
        self.add_output_line(b"Thermal Throttling: Disabled");
        self.add_output_line(b"Sleep Mode: Enabled");
        self.add_output_line(b"Estimated Power: 25.4W");
        self.add_output_line(b"CPU Temperature: 42.1C");
        self.add_output_line(b"Fan Speed: Auto");
        self.add_output_line(b"Battery: N/A (Desktop)");
    }

    fn handle_security_command(&mut self) {
        self.add_output_line(b"=== Security Framework Status ===");
        self.add_output_line(b"Security Level: Standard");
        self.add_output_line(b"Sandbox: Enabled");
        self.add_output_line(b"Access Control: Active");
        self.add_output_line(b"Threat Detection: Running");
        self.add_output_line(b"Blocked Processes: 0");
        self.add_output_line(b"Security Events: 0 recent");
        self.add_output_line(b"Firewall: Enabled");
        self.add_output_line(b"Encryption: AES-256");
        self.add_output_line(b"Last Scan: 2 minutes ago");
    }

    fn handle_features_command(&mut self) {
        self.add_output_line(b"=== Enhanced OS Features ===");
        self.add_output_line(b"");
        self.add_output_line(b"[System Diagnostics]");
        self.add_output_line(b"- Real-time health monitoring");
        self.add_output_line(b"- Automated issue detection");
        self.add_output_line(b"- Performance analytics");
        self.add_output_line(b"");
        self.add_output_line(b"[Process Management]");
        self.add_output_line(b"- Multi-priority scheduling");
        self.add_output_line(b"- Inter-process communication");
        self.add_output_line(b"- Resource isolation");
        self.add_output_line(b"");
        self.add_output_line(b"[Security Framework]");
        self.add_output_line(b"- Capability-based access control");
        self.add_output_line(b"- Process sandboxing");
        self.add_output_line(b"- Security event monitoring");
        self.add_output_line(b"");
        self.add_output_line(b"[Power Management]");
        self.add_output_line(b"- CPU frequency scaling");
        self.add_output_line(b"- Thermal protection");
        self.add_output_line(b"- Power state management");
        self.add_output_line(b"");
        self.add_output_line(b"[Enhanced Networking]");
        self.add_output_line(b"- TCP/UDP protocol stack");
        self.add_output_line(b"- HTTP client/server");
        self.add_output_line(b"- DNS resolution");
        self.add_output_line(b"");
        self.add_output_line(b"[Virtual Filesystem]");
        self.add_output_line(b"- Unix-like directory structure");
        self.add_output_line(b"- File permissions");
        self.add_output_line(b"- Symbolic links");
    }
}
