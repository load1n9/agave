use crate::types::{TerminalApp, Screen};
use crate::state::{COMMAND_HISTORY, COMMAND_HISTORY_COUNT, COMMAND_HISTORY_INDEX};

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
            if COMMAND_HISTORY_COUNT < 10 {
                for i in 0..self.command_length {
                    COMMAND_HISTORY[COMMAND_HISTORY_COUNT][i] = self.command_buffer[i];
                }
                for i in self.command_length..256 {
                    COMMAND_HISTORY[COMMAND_HISTORY_COUNT][i] = 0;
                }
                COMMAND_HISTORY_COUNT += 1;
            } else {
                // Shift history up and add new command at the end
                for i in 0..9 {
                    COMMAND_HISTORY[i] = COMMAND_HISTORY[i + 1];
                }
                for i in 0..self.command_length {
                    COMMAND_HISTORY[9][i] = self.command_buffer[i];
                }
                for i in self.command_length..256 {
                    COMMAND_HISTORY[9][i] = 0;
                }
            }
            COMMAND_HISTORY_INDEX = COMMAND_HISTORY_COUNT;
        }
        
        // Show command in output (echo)
        let mut prompt_line = [0u8; 80];
        prompt_line[0] = b'$';
        prompt_line[1] = b' ';
        for i in 0..self.command_length.min(76) {
            prompt_line[i + 2] = self.command_buffer[i];
        }
        self.add_output_line(&prompt_line);
        
        // Convert command to lowercase for case-insensitive matching
        let mut cmd_lower = [0u8; 256];
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
        } else {
            self.add_output_line(b"Command not found. Type 'help' for available commands.");
        }

        // Clear command buffer
        self.command_length = 0;
        for i in 0..256 {
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
        uptime_line[pos] = b's'; pos += 1;
        
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
        self.add_output_line(b"  exit      - Exit the terminal");
        self.add_output_line(b"");
        self.add_output_line(b"Quick shortcuts (when command line is empty):");
        self.add_output_line(b"  L-ls  H-help  S-system  P-ps");
        self.add_output_line(b"  U-uname  C-clear  M-main");
        self.current_screen = Screen::Help;
    }

    fn handle_system_command(&mut self) {
        self.add_output_line(b"System Information:");
        self.add_output_line(b"  OS: Agave OS v0.1.0");
        self.add_output_line(b"  Kernel: Rust-based microkernel");
        self.add_output_line(b"  Runtime: WASM execution environment");
        self.add_output_line(b"  Architecture: x86_64");
        self.add_output_line(b"  Memory: 128MB available");
        self.add_output_line(b"  Graphics: VirtIO-GPU framebuffer");
        self.current_screen = Screen::System;
    }

    fn handle_clear_command(&mut self) {
        self.output_line_count = 0;
        for i in 0..24 {
            for j in 0..80 {
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
        for i in 0..24 {
            for j in 0..80 {
                self.output_lines[i][j] = 0;
            }
        }
        self.current_screen = Screen::Main;
        unsafe {
            // Clear command history
            COMMAND_HISTORY_COUNT = 0;
            COMMAND_HISTORY_INDEX = 0;
            for i in 0..10 {
                for j in 0..256 {
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
}
