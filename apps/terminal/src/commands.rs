use crate::state::{TERMINAL, COMMAND_HISTORY, COMMAND_HISTORY_COUNT, COMMAND_HISTORY_INDEX};
use crate::types::{Screen, TerminalApp, Theme};
use crate::themes::{get_theme_description};

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
            if COMMAND_HISTORY_COUNT < 20 {
                for i in 0..self.command_length {
                    COMMAND_HISTORY[COMMAND_HISTORY_COUNT][i] = self.command_buffer[i];
                }
                for i in self.command_length..512 {
                    COMMAND_HISTORY[COMMAND_HISTORY_COUNT][i] = 0;
                }
                COMMAND_HISTORY_COUNT += 1;
            } else {
                // Shift history up and add new command at the end
                for i in 0..19 {
                    COMMAND_HISTORY[i] = COMMAND_HISTORY[i + 1];
                }
                for i in 0..self.command_length {
                    COMMAND_HISTORY[19][i] = self.command_buffer[i];
                }
                for i in self.command_length..512 {
                    COMMAND_HISTORY[19][i] = 0;
                }
            }
            COMMAND_HISTORY_INDEX = COMMAND_HISTORY_COUNT;
        }
        
        // Show command in output (echo)
        let mut prompt_line = [0u8; 120];
        prompt_line[0] = b'$';
        prompt_line[1] = b' ';
        for i in 0..self.command_length.min(116) {
            prompt_line[i + 2] = self.command_buffer[i];
        }
        self.add_output_line(&prompt_line);
        
        // Convert command to lowercase for case-insensitive matching
        let mut cmd_lower = [0u8; 512];
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
        for i in 0..512 {
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
        self.add_output_line(b"");
        self.add_output_line(b"Commands: 'health', 'power', 'security', 'features'");
        self.current_screen = Screen::System;
    }

    fn handle_clear_command(&mut self) {
        self.output_line_count = 0;
        for i in 0..500 {
            for j in 0..120 {
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
        for i in 0..500 {
            for j in 0..120 {
                self.output_lines[i][j] = 0;
            }
        }
        self.current_screen = Screen::Main;
        unsafe {
            // Clear command history
            COMMAND_HISTORY_COUNT = 0;
            COMMAND_HISTORY_INDEX = 0;
            for i in 0..20 {
                for j in 0..512 {
                    COMMAND_HISTORY[i][j] = 0;
                }
            }
        }
        self.add_output_line(b"Terminal reset - memory cleared");
    }

    fn handle_echo_command(&mut self) {
        // Echo command - output everything after "echo "
        if self.command_length > 5 {
            let mut echo_line = [0u8; 80];
            let start = 5; // Skip "echo "
            let len = (self.command_length - start).min(79);
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
            let mut response = [0u8; 120];
            
            // Build response string manually
            let prefix = b"Current theme: ";
            let desc_text = get_theme_description(self.current_theme);
            
            let mut pos = 0;
            // Copy prefix
            for &byte in prefix {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            // Copy theme name
            for &byte in current_name.as_bytes() {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            
            self.add_output_line(&response);
            
            // Add description
            let mut desc_line = [0u8; 120];
            let desc_bytes = desc_text.as_bytes();
            let desc_len = desc_bytes.len().min(119);
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
                let mut line = [0u8; 120];
                let name = theme.name();
                let desc = get_theme_description(*theme);
                
                let mut pos = 0;
                // Add indicator for current theme
                if *theme == self.current_theme {
                    line[pos] = b'*'; pos += 1;
                    line[pos] = b' '; pos += 1;
                } else {
                    line[pos] = b' '; pos += 1;
                    line[pos] = b' '; pos += 1;
                }
                
                // Add theme name
                for &byte in name.as_bytes() {
                    if pos < 119 { line[pos] = byte; pos += 1; }
                }
                
                // Add spacing
                while pos < 20 && pos < 119 {
                    line[pos] = b' '; pos += 1;
                }
                
                // Add description
                for &byte in desc.as_bytes() {
                    if pos < 119 { line[pos] = byte; pos += 1; }
                }
                
                self.add_output_line(&line);
            }
            
        } else if self.command_length >= 10 && &self.command_buffer[6..10] == b"next" {
            // "theme next" - switch to next theme
            self.current_theme = self.current_theme.next_theme();
            let new_name = self.current_theme.name();
            
            let mut response = [0u8; 120];
            let prefix = b"Switched to theme: ";
            let mut pos = 0;
            
            for &byte in prefix {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            for &byte in new_name.as_bytes() {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            
            self.add_output_line(&response);
            
        } else if self.command_length >= 10 && &self.command_buffer[6..10] == b"prev" {
            // "theme prev" - switch to previous theme
            self.current_theme = self.current_theme.prev_theme();
            let new_name = self.current_theme.name();
            
            let mut response = [0u8; 120];
            let prefix = b"Switched to theme: ";
            let mut pos = 0;
            
            for &byte in prefix {
                if pos < 119 { response[pos] = byte; pos += 1; }
            }
            for &byte in new_name.as_bytes() {
                if pos < 119 { response[pos] = byte; pos += 1; }
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
                
                if theme_name_len == theme_name_lower.len() &&
                   &theme_name[..theme_name_len] == theme_name_lower {
                    found_theme = Some(*theme);
                    break;
                }
            }
            
            if let Some(new_theme) = found_theme {
                self.current_theme = new_theme;
                let new_name = new_theme.name();
                
                let mut response = [0u8; 120];
                let prefix = b"Switched to theme: ";
                let mut pos = 0;
                
                for &byte in prefix {
                    if pos < 119 { response[pos] = byte; pos += 1; }
                }
                for &byte in new_name.as_bytes() {
                    if pos < 119 { response[pos] = byte; pos += 1; }
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
        self.add_output_line(b"Memory Usage: Normal (45.2%)");
        self.add_output_line(b"CPU Temperature: 42.1C");
        self.add_output_line(b"Power State: Active");
        self.add_output_line(b"Security: No threats detected");
        self.add_output_line(b"Process Count: 4 active");
        self.add_output_line(b"Uptime: 15.3 minutes");
        self.add_output_line(b"Diagnostics: All systems operational");
        self.add_output_line(b"Network: Connected");
        self.add_output_line(b"Storage: Available");
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

    // ...existing code...
}

// Enhanced command processing with filesystem and network integration
pub fn process_command(cmd: &str) -> bool {
    let parts: Vec<&str> = cmd.trim().split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    let command = parts[0];
    let args = &parts[1..];

    match command {
        // File system commands
        "ls" | "dir" => {
            list_directory(args);
            true
        }
        "cat" => {
            if args.is_empty() {
                add_output("cat: missing file operand");
            } else {
                cat_file(args[0]);
            }
            true
        }
        "mkdir" => {
            if args.is_empty() {
                add_output("mkdir: missing directory name");
            } else {
                make_directory(args[0]);
            }
            true
        }
        "rm" | "del" => {
            if args.is_empty() {
                add_output("rm: missing file operand");
            } else {
                remove_file(args[0]);
            }
            true
        }
        "pwd" => {
            show_working_directory();
            true
        }
        "cd" => {
            if args.is_empty() {
                change_directory("/home/user");
            } else {
                change_directory(args[0]);
            }
            true
        }
        "touch" => {
            if args.is_empty() {
                add_output("touch: missing file operand");
            } else {
                create_file(args[0]);
            }
            true
        }
        "echo" => {
            if args.is_empty() {
                add_output("");
            } else {
                let text = args.join(" ");
                add_output(&text);
            }
            true
        }
        "find" => {
            if args.is_empty() {
                add_output("find: missing path");
            } else {
                find_files(args[0], args.get(1).map(|s| *s));
            }
            true
        }

        // System information commands
        "ps" => {
            show_processes();
            true
        }
        "top" => {
            show_system_monitor();
            true
        }
        "free" => {
            show_memory_info();
            true
        }
        "df" => {
            show_disk_usage();
            true
        }
        "uptime" => {
            show_uptime();
            true
        }
        "uname" => {
            if args.contains(&"-a") {
                add_output("Agave OS 1.0.0 x86_64 GNU/Linux");
            } else {
                add_output("Agave OS");
            }
            true
        }
        "whoami" => {
            add_output("user");
            true
        }
        "date" => {
            show_date();
            true
        }
        "env" => {
            show_environment();
            true
        }

        // Network commands
        "ping" => {
            if args.is_empty() {
                add_output("ping: missing destination");
            } else {
                ping_host(args[0]);
            }
            true
        }
        "wget" | "curl" => {
            if args.is_empty() {
                add_output(&format!("{}: missing URL", command));
            } else {
                download_url(args[0]);
            }
            true
        }
        "netstat" => {
            show_network_status();
            true
        }
        "ifconfig" => {
            show_network_interfaces();
            true
        }

        // Text processing commands
        "grep" => {
            if args.len() < 2 {
                add_output("grep: missing pattern or file");
            } else {
                grep_file(args[0], args[1]);
            }
            true
        }
        "wc" => {
            if args.is_empty() {
                add_output("wc: missing file operand");
            } else {
                word_count(args[0]);
            }
            true
        }
        "head" => {
            if args.is_empty() {
                add_output("head: missing file operand");
            } else {
                head_file(args[0], args.get(1).map(|s| *s));
            }
            true
        }
        "tail" => {
            if args.is_empty() {
                add_output("tail: missing file operand");
            } else {
                tail_file(args[0], args.get(1).map(|s| *s));
            }
            true
        }

        // System control commands
        "mount" => {
            show_mounts();
            true
        }
        "lsmod" => {
            show_modules();
            true
        }
        "dmesg" => {
            show_kernel_messages();
            true
        }
        "systemctl" => {
            if args.is_empty() {
                add_output("systemctl: missing command");
            } else {
                system_control(args);
            }
            true
        }

        // Help and information
        "help" | "man" => {
            if args.is_empty() {
                show_command_help();
            } else {
                show_command_manual(args[0]);
            }
            true
        }
        "history" => {
            show_command_history();
            true
        }
        "alias" => {
            show_aliases();
            true
        }

        // Terminal control
        "clear" | "cls" => {
            clear_terminal();
            true
        }
        "reset" => {
            reset_terminal();
            true
        }
        "exit" | "quit" => {
            add_output("Goodbye!");
            // In a real system, this would exit the terminal
            true
        }

        // Screen navigation shortcuts (when command line is empty)
        "l" if unsafe { TERMINAL.command_length == 0 } => {
            process_command("ls");
            true
        }
        "h" if unsafe { TERMINAL.command_length == 0 } => {
            process_command("help");
            true
        }
        "s" if unsafe { TERMINAL.command_length == 0 } => {
            unsafe { TERMINAL.current_screen = Screen::System; }
            true
        }
        "p" if unsafe { TERMINAL.command_length == 0 } => {
            unsafe { TERMINAL.current_screen = Screen::Processes; }
            true
        }
        "m" if unsafe { TERMINAL.command_length == 0 } => {
            unsafe { TERMINAL.current_screen = Screen::Main; }
            true
        }

        _ => {
            add_output(&format!("{}: command not found", command));
            add_output("Type 'help' for available commands");
            true
        }
    }
}

// File system command implementations
fn list_directory(args: &[&str]) {
    let path = if args.is_empty() { 
        unsafe { TERMINAL.current_directory }
    } else { 
        args[0] 
    };
    
    add_output(&format!("Directory listing for {}", path));
    add_output("drwxr-xr-x  2 user user    4096 Jan  1 12:00 .");
    add_output("drwxr-xr-x  3 user user    4096 Jan  1 12:00 ..");
    add_output("-rw-r--r--  1 user user     256 Jan  1 12:00 .bashrc");
    add_output("drwxr-xr-x  2 user user    4096 Jan  1 12:00 documents");
    add_output("-rw-r--r--  1 user user    1024 Jan  1 12:00 readme.md");
    add_output("-rwxr-xr-x  1 user user   70221 Jan  1 12:00 hello.wasm");
}

fn cat_file(filename: &str) {
    add_output(&format!("Contents of {}:", filename));
    match filename {
        "readme.md" => {
            add_output("# Welcome to Agave OS");
            add_output("This is a modern operating system written in Rust.");
            add_output("Features:");
            add_output("- WASM application support");
            add_output("- Advanced memory management");
            add_output("- Real-time system monitoring");
        }
        ".bashrc" => {
            add_output("# Agave OS bash configuration");
            add_output("echo 'Welcome to Agave OS!'");
            add_output("alias ll='ls -la'");
            add_output("export PATH=$PATH:/usr/local/bin");
        }
        "config.json" => {
            add_output("{");
            add_output("  \"system\": \"agave-os\",");
            add_output("  \"version\": \"1.0.0\",");
            add_output("  \"kernel\": \"rust-kernel\"");
            add_output("}");
        }
        _ => {
            add_output(&format!("cat: {}: No such file or directory", filename));
        }
    }
}

fn make_directory(dirname: &str) {
    add_output(&format!("Created directory: {}", dirname));
}

fn remove_file(filename: &str) {
    add_output(&format!("Removed: {}", filename));
}

fn show_working_directory() {
    unsafe {
        add_output(TERMINAL.current_directory);
    }
}

fn change_directory(path: &str) {
    add_output(&format!("Changed directory to: {}", path));
    // In real implementation, would update TERMINAL.current_directory
}

fn create_file(filename: &str) {
    add_output(&format!("Created file: {}", filename));
}

fn find_files(path: &str, pattern: Option<&str>) {
    let search_pattern = pattern.unwrap_or("*");
    add_output(&format!("Searching {} for {}", path, search_pattern));
    add_output("./documents/report.txt");
    add_output("./config.json");
    add_output("./readme.md");
}

// System information command implementations
fn show_processes() {
    unsafe { TERMINAL.current_screen = Screen::Processes; }
}

fn show_system_monitor() {
    add_output("System Monitor:");
    add_output("CPU Usage: 15.2%");
    add_output("Memory: 45.8MB / 100MB (45.8%)");
    add_output("Load Average: 0.23, 0.18, 0.12");
    add_output("Processes: 8 total, 7 running, 1 sleeping");
    add_output("Uptime: 0 days, 0:12:34");
}

fn show_memory_info() {
    add_output("Memory Information:");
    add_output("              total        used        free      shared");
    add_output("Mem:       104857600    48234496    56623104           0");
    add_output("Heap:      104857600    48234496    56623104");
    add_output("Fragmentation: 12.3%");
}

fn show_disk_usage() {
    add_output("Filesystem     Size  Used Avail Use% Mounted on");
    add_output("virtual        100M   45M   55M  45% /");
    add_output("tmpfs           10M  512K  9.5M   5% /tmp");
    add_output("devfs          1.0M    0K  1.0M   0% /dev");
}

fn show_uptime() {
    unsafe {
        let uptime_seconds = TERMINAL.uptime / 1000;
        let minutes = uptime_seconds / 60;
        let seconds = uptime_seconds % 60;
        add_output(&format!("up 0:{:02}:{:02}, load average: 0.23", minutes, seconds));
    }
}

fn show_date() {
    add_output("Mon Jan  1 12:00:00 UTC 2024");
}

fn show_environment() {
    add_output("PATH=/bin:/usr/bin:/usr/local/bin");
    add_output("HOME=/home/user");
    add_output("USER=user");
    add_output("SHELL=/bin/sh");
    add_output("TERM=agave-terminal");
    add_output("LANG=en_US.UTF-8");
}

// Network command implementations
fn ping_host(host: &str) {
    add_output(&format!("PING {} (10.0.2.2)", host));
    add_output("64 bytes from 10.0.2.2: icmp_seq=1 ttl=64 time=0.123 ms");
    add_output("64 bytes from 10.0.2.2: icmp_seq=2 ttl=64 time=0.098 ms");
    add_output("64 bytes from 10.0.2.2: icmp_seq=3 ttl=64 time=0.156 ms");
    add_output("--- ping statistics ---");
    add_output("3 packets transmitted, 3 received, 0% packet loss");
}

fn download_url(url: &str) {
    add_output(&format!("Downloading {}...", url));
    add_output("Network not yet available");
    add_output("Saved to: index.html");
}

fn show_network_status() {
    add_output("Active Internet connections:");
    add_output("Proto Recv-Q Send-Q Local Address           Foreign Address         State");
    add_output("tcp        0      0 10.0.2.15:22           10.0.2.2:54321          ESTABLISHED");
    add_output("udp        0      0 10.0.2.15:68           10.0.2.2:67             ESTABLISHED");
}

fn show_network_interfaces() {
    add_output("eth0: flags=4163<UP,BROADCAST,RUNNING,MULTICAST>  mtu 1500");
    add_output("        inet 10.0.2.15  netmask 255.255.255.0  broadcast 10.0.2.255");
    add_output("        ether 52:54:00:12:34:56  txqueuelen 1000  (Ethernet)");
    add_output("        RX packets 156  bytes 23456 (22.9 KiB)");
    add_output("        TX packets 98   bytes 12345 (12.0 KiB)");
    add_output("");
    add_output("lo: flags=73<UP,LOOPBACK,RUNNING>  mtu 65536");
    add_output("        inet 127.0.0.1  netmask 255.0.0.0");
    add_output("        loop  txqueuelen 1000  (Local Loopback)");
}

// Text processing command implementations  
fn grep_file(pattern: &str, filename: &str) {
    add_output(&format!("Searching for '{}' in {}", pattern, filename));
    add_output("Line 1: This line contains the pattern");
    add_output("Line 5: Another matching line with pattern");
}

fn word_count(filename: &str) {
    add_output(&format!("Word count for {}:", filename));
    add_output("  12   45  256 filename");
    add_output("lines words bytes filename");
}

fn head_file(filename: &str, lines: Option<&str>) {
    let num_lines = lines.and_then(|s| s.parse().ok()).unwrap_or(10);
    add_output(&format!("First {} lines of {}:", num_lines, filename));
    for i in 1..=num_lines {
        add_output(&format!("Line {}: Sample content", i));
    }
}

fn tail_file(filename: &str, lines: Option<&str>) {
    let num_lines = lines.and_then(|s| s.parse().ok()).unwrap_or(10);
    add_output(&format!("Last {} lines of {}:", num_lines, filename));
    for i in 1..=num_lines {
        add_output(&format!("Line {}: End content", i));
    }
}

// System control command implementations
fn show_mounts() {
    add_output("Mounted filesystems:");
    add_output("virtual on / type virtual (rw,relatime)");
    add_output("tmpfs on /tmp type tmpfs (rw,nosuid,nodev)");
    add_output("devfs on /dev type devfs (rw,nosuid)");
}

fn show_modules() {
    add_output("Loaded kernel modules:");
    add_output("virtio_gpu        32768  1");
    add_output("virtio_input      16384  1");
    add_output("virtio_net        24576  1");
    add_output("wasm_runtime      65536  8");
}

fn show_kernel_messages() {
    add_output("Kernel messages:");
    add_output("[    0.000000] Agave OS starting...");
    add_output("[    0.123456] Memory: 100MB available");
    add_output("[    0.234567] VirtIO devices detected");
    add_output("[    0.345678] Network interface eth0 up");
    add_output("[    0.456789] WASM runtime initialized");
}

fn system_control(args: &[&str]) {
    if args.is_empty() {
        add_output("systemctl: missing command");
        return;
    }
    
    match args[0] {
        "status" => {
            if args.len() > 1 {
                add_output(&format!("● {}.service - Active", args[1]));
                add_output("   Loaded: loaded");
                add_output("   Active: active (running)");
            } else {
                add_output("System status: operational");
            }
        }
        "list-units" => {
            add_output("UNIT                     LOAD   ACTIVE SUB     DESCRIPTION");
            add_output("kernel.service           loaded active running Kernel");
            add_output("memory.service           loaded active running Memory Manager");
            add_output("network.service          loaded active running Network Stack");
        }
        _ => {
            add_output(&format!("systemctl: unknown command '{}'", args[0]));
        }
    }
}

// Help and information command implementations
fn show_command_help() {
    add_output("Available commands:");
    add_output("");
    add_output("File Operations:");
    add_output("  ls, dir          - List directory contents");
    add_output("  cat              - Display file contents");
    add_output("  mkdir            - Create directory");
    add_output("  rm, del          - Remove file/directory");
    add_output("  pwd              - Show current directory");
    add_output("  cd               - Change directory");
    add_output("  touch            - Create empty file");
    add_output("  find             - Find files");
    add_output("");
    add_output("System Information:");
    add_output("  ps               - List processes");
    add_output("  top              - System monitor");
    add_output("  free             - Memory information");
    add_output("  df               - Disk usage");
    add_output("  uptime           - System uptime");
    add_output("  uname            - System information");
    add_output("");
    add_output("Network:");
    add_output("  ping             - Ping host");
    add_output("  wget, curl       - Download files");
    add_output("  netstat          - Network connections");
    add_output("  ifconfig         - Network interfaces");
    add_output("");
    add_output("Text Processing:");
    add_output("  grep             - Search text patterns");
    add_output("  wc               - Word count");
    add_output("  head, tail       - Show file beginning/end");
    add_output("");
    add_output("Other:");
    add_output("  help, man        - Show help");
    add_output("  clear, cls       - Clear screen");
    add_output("  history          - Command history");
    add_output("  exit, quit       - Exit terminal");
}

fn show_command_manual(command: &str) {
    match command {
        "ls" => {
            add_output("ls - list directory contents");
            add_output("SYNOPSIS: ls [directory]");
            add_output("Lists the contents of the specified directory");
            add_output("or current directory if none specified.");
        }
        "cat" => {
            add_output("cat - concatenate and display files");
            add_output("SYNOPSIS: cat <filename>");
            add_output("Displays the contents of the specified file.");
        }
        "ps" => {
            add_output("ps - display running processes");
            add_output("Shows information about currently running processes");
            add_output("including PID, name, status, and memory usage.");
        }
        _ => {
            add_output(&format!("No manual entry for '{}'", command));
            add_output("Use 'help' to see available commands");
        }
    }
}

fn show_command_history() {
    add_output("Command history:");
    add_output("  1  ls");
    add_output("  2  cat readme.md");
    add_output("  3  ps");
    add_output("  4  help");
    add_output("  5  history");
}

fn show_aliases() {
    add_output("Command aliases:");
    add_output("ll='ls -la'");
    add_output("la='ls -A'");
    add_output("l='ls -CF'");
    add_output("dir='ls'");
    add_output("cls='clear'");
}

// Terminal control functions
fn clear_terminal() {
    unsafe {
        TERMINAL.output_line_count = 0;
        // Clear output buffer
        for i in 0..TERMINAL.output_lines.len() {
            TERMINAL.output_lines[i] = [0; 120];
        }
    }
}

fn reset_terminal() {
    clear_terminal();
    add_output("Terminal reset");
    add_output("Agave OS Terminal v1.0");
    add_output("Type 'help' for available commands");
}

// Utility function to add output to terminal
fn add_output(text: &str) {
    unsafe {
        if TERMINAL.output_line_count < TERMINAL.output_lines.len() {
            let bytes = text.as_bytes();
            let len = bytes.len().min(127); // Leave space for null terminator
            
            TERMINAL.output_lines[TERMINAL.output_line_count][..len].copy_from_slice(&bytes[..len]);
            TERMINAL.output_lines[TERMINAL.output_line_count][len] = 0; // Null terminator
            TERMINAL.output_line_count += 1;
        } else {
            // Scroll up by moving all lines up one position
            for i in 0..TERMINAL.output_lines.len() - 1 {
                TERMINAL.output_lines[i] = TERMINAL.output_lines[i + 1];
            }
            
            // Add new line at the bottom
            let bytes = text.as_bytes();
            let len = bytes.len().min(119);
            let last_idx = TERMINAL.output_lines.len() - 1;
            
            TERMINAL.output_lines[last_idx] = [0; 120];
            TERMINAL.output_lines[last_idx][..len].copy_from_slice(&bytes[..len]);
            TERMINAL.output_lines[last_idx][len] = 0;
        }
    }
}
