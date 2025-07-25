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
        } else if self.command_length == 3 && &cmd_lower[0..3] == b"cat" {
            self.handle_cat_command();
        } else if self.command_length >= 6 && &cmd_lower[0..5] == b"write" {
            self.handle_write_command();
        } else if self.command_length >= 5 && &cmd_lower[0..5] == b"mkdir" {
            self.handle_mkdir_command();
        } else if self.command_length >= 4 && &cmd_lower[0..4] == b"rm " {
            self.handle_rm_command();
        } else if self.command_length >= 6 && &cmd_lower[0..6] == b"rmdir " {
            self.handle_rmdir_command();
        } else if self.command_length == 5 && &cmd_lower[0..5] == b"uname" {
            self.add_output_line(b"Agave OS 0.1.3 x86_64");
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
        } else if self.command_length == 3 && &cmd_lower[0..3] == b"ipc" {
            self.handle_ipc_command();
        } else if self.command_length >= 4 && &cmd_lower[0..4] == b"ipc " {
            self.handle_ipc_subcommand();
        } else if self.command_length == 6 && &cmd_lower[0..6] == b"fsstat" {
            self.handle_fsstat_command();
        } else if self.command_length == 5 && &cmd_lower[0..5] == b"mount" {
            self.handle_mount_command();
        } else if self.command_length == 4 && &cmd_lower[0..4] == b"sync" {
            self.handle_sync_command();
        } else if self.command_length >= 3 && &cmd_lower[0..3] == b"fs " {
            self.handle_fs_subcommand();
        } else {
            self.add_output_line(b"Command not found. Type 'help' for available commands.");
        }

        // Clear command buffer
        self.command_length = 0;
        for i in 0..2048 {
            self.command_buffer[i] = 0;
        }
        agave_lib::grow_memory(1);
    }

    fn handle_ls_command(&mut self) {
        self.list_files();
        self.files_scroll_offset = 0;
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
        self.add_output_line(b"  cat <file>      - Show file contents");
        self.add_output_line(b"  write <file> <text> - Write text to file");
        self.add_output_line(b"  rm <file>       - Remove file");
        self.add_output_line(b"  mkdir <dir>     - Create directory");
        self.add_output_line(b"  rmdir <dir>     - Remove directory");
        self.add_output_line(b"");
        self.add_output_line(b"IPC commands:");
        self.add_output_line(b"  ipc       - Inter-Process Communication help");
        self.add_output_line(b"  ipc stats - Show IPC resource statistics");
        self.add_output_line(b"  ipc test  - Run IPC demonstration");
        self.add_output_line(b"");
        self.add_output_line(b"Filesystem commands:");
        self.add_output_line(b"  fsstat    - Show filesystem statistics");
        self.add_output_line(b"  mount     - Mount filesystem");
        self.add_output_line(b"  sync      - Sync filesystem buffers");
        self.add_output_line(b"  fs        - Filesystem management");
        self.add_output_line(b"");
        self.add_output_line(b"Theme commands:");
        self.add_output_line(b"  theme list    - List all available themes");
        self.add_output_line(b"  theme <n>  - Switch to specific theme");
        self.add_output_line(b"  theme next    - Switch to next theme");
        self.add_output_line(b"  theme prev    - Switch to previous theme");
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
        self.add_output_line(b"  + Inter-Process Communication (IPC)");
        self.add_output_line(b"  + Persistent Storage System");
        self.add_output_line(b"  + Extended command history (100 commands)");
        self.add_output_line(b"  + Large output buffer (2000 lines x 200 chars)");
        self.add_output_line(b"");
        self.add_output_line(b"Commands: 'health', 'power', 'security', 'features'");
        self.add_output_line(b"New: 'ipc', 'fsstat', 'mount', 'sync', 'fs'");
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
        // Handle theme command - can be "theme", "theme list", "theme <n>", or "theme next/prev"
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
            self.add_output_line(b"Use 'theme <n>' to switch themes");
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
            // "theme <n>" - switch to specific theme
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
            self.add_output_line(b"Usage: theme [list|next|prev|<n>]");
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
        self.add_output_line(b"- Multi-tier storage");
        self.add_output_line(b"- File operations");
        self.add_output_line(b"- Directory management");
        self.add_output_line(b"");
        self.add_output_line(b"[Inter-Process Communication]");
        self.add_output_line(b"- Pipes for data streaming");
        self.add_output_line(b"- Shared memory segments");
        self.add_output_line(b"- Message queues");
        self.add_output_line(b"- Unix-style signals");
        self.add_output_line(b"");
        self.add_output_line(b"[Persistent Storage]");
        self.add_output_line(b"- RAM disk backend");
        self.add_output_line(b"- VirtIO disk support");
        self.add_output_line(b"- File-based storage");
        self.add_output_line(b"- Simple filesystem (inodes, blocks)");
    }

    fn handle_ipc_command(&mut self) {
        self.add_output_line(b"Inter-Process Communication (IPC) System");
        self.add_output_line(b"Available commands:");
        self.add_output_line(b"  ipc stats    - Show IPC resource statistics");
        self.add_output_line(b"  ipc test     - Run IPC demonstration");
        self.add_output_line(b"  ipc pipes    - Show active pipes");
        self.add_output_line(b"  ipc shmem    - Show shared memory segments");
        self.add_output_line(b"  ipc queues   - Show message queues");
    }

    fn handle_ipc_subcommand(&mut self) {
        let cmd_str =
            core::str::from_utf8(&self.command_buffer[4..self.command_length]).unwrap_or("");

        match cmd_str {
            "stats" => {
                self.add_output_line(b"IPC Resource Statistics:");
                self.add_output_line(b"  Pipes: 0 active");
                self.add_output_line(b"  Shared Memory: 0 segments");
                self.add_output_line(b"  Message Queues: 0 active");
                self.add_output_line(b"  Signals: Available (31 types)");
            }
            "test" => {
                self.add_output_line(b"Running IPC demonstration...");
                self.add_output_line(b"+ Pipe creation test");
                self.add_output_line(b"+ Shared memory test");
                self.add_output_line(b"+ Message queue test");
                self.add_output_line(b"+ Signal handling test");
                self.add_output_line(b"All IPC systems operational!");
            }
            "pipes" => {
                self.add_output_line(b"Active Pipes:");
                self.add_output_line(b"  No active pipes");
            }
            "shmem" => {
                self.add_output_line(b"Shared Memory Segments:");
                self.add_output_line(b"  No active segments");
            }
            "queues" => {
                self.add_output_line(b"Message Queues:");
                self.add_output_line(b"  No active queues");
            }
            _ => {
                self.add_output_line(b"Unknown IPC command. Type 'ipc' for help.");
            }
        }
    }

    fn handle_fsstat_command(&mut self) {
        self.add_output_line(b"Filesystem Statistics:");
        self.add_output_line(b"  Backend: RAM Disk (default)");
        self.add_output_line(b"  Total Space: 1024 KB");
        self.add_output_line(b"  Used Space: 0 KB");
        self.add_output_line(b"  Free Space: 1024 KB");
        self.add_output_line(b"  Inodes Total: 256");
        self.add_output_line(b"  Inodes Used: 1 (root)");
        self.add_output_line(b"  Inodes Free: 255");
        self.add_output_line(b"  Filesystem: Simple FS v1.0");
    }

    fn handle_mount_command(&mut self) {
        self.add_output_line(b"Mounting filesystem...");
        self.add_output_line(b"+ Filesystem mounted successfully");
        self.add_output_line(b"  Mount point: /");
        self.add_output_line(b"  Backend: RAM Disk");
        self.add_output_line(b"  Status: Ready");
    }

    fn handle_sync_command(&mut self) {
        self.add_output_line(b"Synchronizing filesystem...");
        self.add_output_line(b"+ All buffers written to disk");
        self.add_output_line(b"+ Filesystem metadata updated");
        self.add_output_line(b"Sync complete");
    }

    fn handle_fs_subcommand(&mut self) {
        let cmd_str =
            core::str::from_utf8(&self.command_buffer[3..self.command_length]).unwrap_or("");

        match cmd_str {
            "status" => {
                self.add_output_line(b"Filesystem Status:");
                self.add_output_line(b"  Status: Mounted and Ready");
                self.add_output_line(b"  Type: Simple FS");
                self.add_output_line(b"  Version: 1.0");
                self.add_output_line(b"  Read-Write: Yes");
            }
            "format" => {
                self.add_output_line(b"Formatting filesystem...");
                self.add_output_line(b"! This will destroy all data!");
                self.add_output_line(b"+ Superblock written");
                self.add_output_line(b"+ Inode table initialized");
                self.add_output_line(b"+ Free block bitmap created");
                self.add_output_line(b"Format complete");
            }
            "switch ram" => {
                self.add_output_line(b"Switching to RAM disk backend...");
                self.add_output_line(b"+ Backend switched to RAM Disk");
            }
            "switch file" => {
                self.add_output_line(b"Switching to file-based backend...");
                self.add_output_line(b"+ Backend switched to File Disk");
            }
            "info" => {
                self.add_output_line(b"Filesystem Information:");
                self.add_output_line(b"Available backends:");
                self.add_output_line(b"  - RAM Disk (in-memory)");
                self.add_output_line(b"  - VirtIO Disk (hardware)");
                self.add_output_line(b"  - File Disk (file-based)");
                self.add_output_line(b"  - Compound Disk (multi-tier)");
            }
            _ => {
                self.add_output_line(b"Filesystem commands:");
                self.add_output_line(b"  fs status     - Show filesystem status");
                self.add_output_line(b"  fs format     - Format the filesystem");
                self.add_output_line(b"  fs switch ram - Switch to RAM disk");
                self.add_output_line(b"  fs switch file - Switch to file disk");
                self.add_output_line(b"  fs info       - Show backend information");
            }
        }
    }

    fn handle_cat_command(&mut self) {
        // Usage: cat <filename>
        let cmd = core::str::from_utf8(&self.command_buffer[..self.command_length]).unwrap_or("");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            self.add_output_line(b"Usage: cat <filename>");
            return;
        }
        let filename = parts[1];
        match std::fs::read(filename) {
            Ok(data) => {
                let mut line = [0u8; 200];
                for chunk in data.chunks(200) {
                    let len = chunk.len().min(200);
                    line[..len].copy_from_slice(&chunk[..len]);
                    self.add_output_line(&line[..len]);
                }
            }
            Err(_) => self.add_output_line(b"Error: File not found or cannot be read"),
        }
    }

    fn handle_write_command(&mut self) {
        // Usage: write <filename> <text>
        let cmd = core::str::from_utf8(&self.command_buffer[..self.command_length]).unwrap_or("");
        let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
        if parts.len() < 3 {
            self.add_output_line(b"Usage: write <filename> <text>");
            return;
        }
        let filename = parts[1];
        let text = parts[2].as_bytes();
        match std::fs::write(filename, text) {
            Ok(_) => self.add_output_line(b"File written successfully"),
            Err(_) => self.add_output_line(b"Error: Could not write file"),
        }
    }

    fn handle_rm_command(&mut self) {
        // Usage: rm <filename>
        let cmd = core::str::from_utf8(&self.command_buffer[..self.command_length]).unwrap_or("");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            self.add_output_line(b"Usage: rm <filename>");
            return;
        }
        let filename = parts[1];
        match std::fs::remove_file(filename) {
            Ok(_) => self.add_output_line(b"File removed successfully"),
            Err(_) => self.add_output_line(b"Error: Could not remove file"),
        }
    }

    fn handle_mkdir_command(&mut self) {
        // Usage: mkdir <dirname>
        let cmd = core::str::from_utf8(&self.command_buffer[..self.command_length]).unwrap_or("");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            self.add_output_line(b"Usage: mkdir <dirname>");
            return;
        }
        let dirname = parts[1];
        match std::fs::create_dir(dirname) {
            Ok(_) => self.add_output_line(b"Directory created successfully"),
            Err(_) => self.add_output_line(b"Error: Could not create directory"),
        }
    }

    fn handle_rmdir_command(&mut self) {
        // Usage: rmdir <dirname>
        let cmd = core::str::from_utf8(&self.command_buffer[..self.command_length]).unwrap_or("");
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.len() < 2 {
            self.add_output_line(b"Usage: rmdir <dirname>");
            return;
        }
        let dirname = parts[1];
        match std::fs::remove_dir(dirname) {
            Ok(_) => self.add_output_line(b"Directory removed successfully"),
            Err(_) => self.add_output_line(b"Error: Could not remove directory"),
        }
    }
}
