use agave_lib::{
    clear_screen, draw_line, draw_rectangle, draw_text, fill_circle, fill_rectangle,
    get_dimensions, Position, RGBA,
};

use crate::state::{ANIMATION_FRAME, CURSOR_BLINK, TERMINAL};
use crate::themes::{get_theme_colors, ThemeColors};
use crate::types::Screen;

pub fn draw_terminal() {
    let dim = get_dimensions();

    unsafe {
        let colors = get_theme_colors(TERMINAL.current_theme);

        // Clear screen with theme background
        clear_screen(colors.bg_primary);

        // Draw subtle gradient background
        draw_background_gradient(dim, &colors);

        match TERMINAL.current_screen {
            Screen::Main => draw_main_screen(dim, &colors),
            Screen::Files => draw_files_screen(dim, &colors),
            Screen::Processes => draw_processes_screen(dim, &colors),
            Screen::System => draw_system_screen(dim, &colors),
            Screen::Help => draw_help_screen(dim, &colors),
        }
    }

    // Draw enhanced status bar at bottom
    unsafe {
        let colors = get_theme_colors(TERMINAL.current_theme);
        draw_status_bar(dim, &colors);
    }
}

fn draw_background_gradient(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    // Draw subtle vertical gradient for visual depth
    for y in 0..dim.height / 4 {
        let alpha = 10 + (y * 15 / (dim.height / 4));
        fill_rectangle(
            Position::new(0, y),
            dim.width,
            1,
            RGBA::new(
                colors.border_color.r / 2,
                colors.border_color.g / 2,
                colors.border_color.b / 2,
                alpha as i32,
            ),
        );
    }
}

fn draw_main_screen(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);

    unsafe {
        draw_card(
            Position::new(margin - 20, 40),
            content_width + 40,
            dim.height - 140,
            colors,
        );

        let header_lines = [
            "     ___       _______      ___   ____    ____  _______ ",
            "    /   \\     /  _____|    /   \\  \\   \\  /   / |   ____|",
            "   /  ^  \\   |  |  __     /  ^  \\  \\   \\/   /  |  |__   ",
            "  /  /_\\  \\  |  | |_ |   /  /_\\  \\  \\      /   |   __|  ",
            " /  _____  \\ |  |__| |  /  _____  \\  \\    /    |  |____ ",
            "/__/     \\__\\ \\______| /__/     \\__\\  \\__/     |_______|",
            "                                                        ",
        ];

        for (i, line) in header_lines.iter().enumerate() {
            draw_text(
                Position::new(margin + 20, 70 + i as i32 * 22),
                line,
                colors.accent_cyan,
            );
        }

        draw_text(
            Position::new(margin + 20, 210),
            "Welcome to Agave OS",
            colors.text_primary,
        );
        const FUN_FACTS: &[&str] = &[
            "Honey never spoils - archaeologists have found edible honey in ancient Egyptian tombs!",
            "A group of flamingos is called a 'flamboyance'.",
            "Octopuses have three hearts and blue blood.",
            "Bananas are berries, but strawberries aren't.",
            "All Pandas in the world are owned by China, even those in other countries.",
            "Bananas are curved because they grow towards the sun.",
            "The first bug in a computer was an actual moth found in 1947.",
            "A day on Venus is longer than a year on Venus due to its slow rotation.",
            "Wombat poop is cube-shaped to prevent it from rolling away.",
            "A jiffy is an actual unit of time - 1/100th of a second.",
            "The Eiffel Tower can be 15 cm taller during the summer due to thermal expansion.",
            "Platypuses get the most REM sleep of any mammal, averaging 5-8 hours a day.",
            "The human brain uses about 20% of your total energy despite being only 2% of your body weight.",
            "Sharks have been around longer than trees - about 400 million years!",
            "A single cloud can weigh more than a million pounds.",
            "You can't hum while holding your nose closed.",
            "Bubble wrap was originally invented as wallpaper.",
            "The shortest war in history lasted only 38-45 minutes.",
            "Dolphins have names for each other - they respond to unique whistle signatures.",
            "There are more possible games of chess than atoms in the observable universe.",
            "A group of ravens is called a 'conspiracy'.",
            "Your footprint is the only part of your body that creates a unique print.",
            "Pineapples take about 2 years to grow.",
            "The first computer programmer was Ada Lovelace in 1843.",
            "A single teaspoon of neutron star would weigh 6 billion tons.",
            "The world's largest desert is Antarctica, not the Sahara.",
            "A snail can sleep for three years at a time.",
            "The shortest war in history lasted only 38-45 minutes.",
            "The average person walks the equivalent of five times around the world in their lifetime.",
        ];

        let fact_index = ((TERMINAL.uptime / 30000) as usize) % FUN_FACTS.len();
        let fun_fact = FUN_FACTS[fact_index];

        draw_text(
            Position::new(margin + 20, 235),
            fun_fact,
            colors.text_secondary,
        );

    // let logo_x = margin + content_width - 320;
    // let logo_y = 70;
    // let logo_width = 120.0;
    // let logo_height = 120.0;

    let clock_radius = 60;
    let clock_center_x = margin + content_width - 60;
    let clock_center_y = 120;

        fill_circle(
            Position::new(clock_center_x + 3, clock_center_y + 4),
            clock_radius,
            colors.bg_primary,
        );

        fill_circle(
            Position::new(clock_center_x, clock_center_y),
            clock_radius,
            colors.bg_accent,
        );
        fill_circle(
            Position::new(clock_center_x, clock_center_y),
            clock_radius - 3,
            colors.bg_secondary,
        );

        for i in 0..12 {
            let angle = (i as f32) * 30.0 - 90.0;
            let rad = angle * core::f32::consts::PI / 180.0;
            let outer = (
                (clock_center_x as f32 + (clock_radius as f32 - 4.0) * rad.cos()) as i32,
                (clock_center_y as f32 + (clock_radius as f32 - 4.0) * rad.sin()) as i32,
            );
            let inner = (
                (clock_center_x as f32 + (clock_radius as f32 - 10.0) * rad.cos()) as i32,
                (clock_center_y as f32 + (clock_radius as f32 - 10.0) * rad.sin()) as i32,
            );
            draw_line(
                Position::new(inner.0, inner.1),
                Position::new(outer.0, outer.1),
                if i % 3 == 0 { colors.accent_blue } else { colors.border_color },
            );
        }

        // Draw clock hands based on uptime (as system time)
        let total_seconds = TERMINAL.uptime / 1000;
        let seconds = (total_seconds % 60) as f32;
        let minutes = ((total_seconds / 60) % 60) as f32;
        let hours = ((total_seconds / 3600) % 12) as f32;

        let to_rad = |deg: f32| deg * core::f32::consts::PI / 180.0;
        let hand = |len: f32, angle_deg: f32| {
            let angle = to_rad(angle_deg - 90.0);
            (
                (clock_center_x as f32 + len * angle.cos()) as i32,
                (clock_center_y as f32 + len * angle.sin()) as i32,
            )
        };

        // Hour hand (thicker, shorter, accent color)
        let hour_angle = (hours + minutes / 60.0) * 30.0;
        let (hx, hy) = hand(clock_radius as f32 * 0.45, hour_angle);
        for dx in -1..=1 {
            for dy in -1..=1 {
                draw_line(
                    Position::new(clock_center_x + dx, clock_center_y + dy),
                    Position::new(hx + dx, hy + dy),
                    colors.accent_purple,
                );
            }
        }

        let min_angle = (minutes + seconds / 60.0) * 6.0;
        let (mx, my) = hand(clock_radius as f32 * 0.7, min_angle);
        for dx in -1..=1 {
            draw_line(
                Position::new(clock_center_x + dx, clock_center_y + dx),
                Position::new(mx + dx, my + dx),
                colors.accent_blue,
            );
        }

        let sec_angle = seconds * 6.0;
        let (sx, sy) = hand(clock_radius as f32 * 0.82, sec_angle);
        draw_line(
            Position::new(clock_center_x, clock_center_y),
            Position::new(sx, sy),
            colors.accent_red,
        );

        fill_circle(
            Position::new(clock_center_x, clock_center_y),
            4,
            colors.accent_green,
        );

        // let svg_x = |x: f32| -> i32 {
        //     let min_x = 152.0;
        //     let max_x = 152.0 + 579.0;
        //     let scale = logo_width / (max_x - min_x);
        //     (logo_x as f32 + (x - min_x) * scale) as i32
        // };
        // let svg_y = |y: f32| -> i32 {
        //     let min_y = -20.0;
        //     let max_y = -20.0 + 606.0;
        //     let scale = logo_height / (max_y - min_y);
        //     (logo_y as f32 + (y - min_y) * scale) as i32
        // };

        // let svg_paths: &[&[(f32, f32)]] = &[
        //     &[
        //         (480.0, 345.0),
        //         (484.0, 423.0),
        //         (536.0, 353.0),
        //         (629.0, 119.0),
        //         (480.0, 345.0),
        //     ],
        //     &[
        //         (480.0, 308.0),
        //         (472.0, 224.0),
        //         (558.0, 56.0),
        //         (531.0, 208.0),
        //         (480.0, 308.0),
        //     ],
        //     &[
        //         (363.0, 581.0),
        //         (585.0, 341.0),
        //         (714.0, 224.0),
        //         (426.0, 586.0),
        //         (363.0, 581.0),
        //     ],
        //     &[
        //         (507.0, 506.0),
        //         (589.0, 408.0),
        //         (731.0, 324.0),
        //         (602.0, 456.0),
        //         (507.0, 506.0),
        //     ],
        //     &[
        //         (452.0, 585.0),
        //         (482.0, 538.0),
        //         (712.0, 421.0),
        //         (532.0, 584.0),
        //         (452.0, 585.0),
        //     ],
        //     &[
        //         (340.0, 578.0),
        //         (372.0, 532.0),
        //         (174.0, 435.0),
        //         (340.0, 578.0),
        //     ],
        //     &[
        //         (375.0, 515.0),
        //         (171.0, 351.0),
        //         (281.0, 476.0),
        //         (375.0, 515.0),
        //     ],
        //     &[
        //         (397.0, 518.0),
        //         (413.0, 500.0),
        //         (303.0, 369.0),
        //         (152.0, 221.0),
        //         (397.0, 518.0),
        //     ],
        //     &[
        //         (381.0, 432.0),
        //         (391.0, 360.0),
        //         (251.0, 120.0),
        //         (311.0, 349.0),
        //         (381.0, 432.0),
        //     ],
        //     &[
        //         (400.0, 455.0),
        //         (424.0, 486.0),
        //         (471.0, 430.0),
        //         (439.0, -20.0),
        //         (400.0, 455.0),
        //     ],
        //     &[
        //         (397.0, 332.0),
        //         (403.0, 203.0),
        //         (320.0, 52.0),
        //         (360.0, 235.0),
        //         (397.0, 332.0),
        //     ],
        // ];

        // for path in svg_paths {
        //     for pair in path.windows(2) {
        //         let (x1, y1) = pair[0];
        //         let (x2, y2) = pair[1];
        //         draw_line(
        //             Position::new(svg_x(x1), svg_y(y1)),
        //             Position::new(svg_x(x2), svg_y(y2)),
        //             colors.border_color,
        //         );
        //     }
        // }

        // Status section with visual separation
        draw_section_divider(Position::new(margin, 270), content_width, colors);

        draw_text(
            Position::new(margin + 20, 295),
            "● System Status:",
            colors.accent_green,
        );
        draw_text(
            Position::new(margin + 160, 295),
            "Online",
            colors.accent_green,
        );

        // Show current theme
        draw_text(
            Position::new(margin + 20, 320),
            "● Theme:",
            colors.text_secondary,
        );
        draw_text(
            Position::new(margin + 100, 320),
            #[allow(static_mut_refs)]
            TERMINAL.current_theme.name(),
            colors.accent_purple,
        );

        // Terminal output section with scroll support
        draw_section_divider(Position::new(margin, 375), content_width, colors);

        let output_header_y = 400;

        // Show scroll indicators if needed
        if TERMINAL.scroll_offset > 0 {
            draw_text(
                Position::new(margin + 20, output_header_y),
                "(Scrolled - Use ↑↓ PgUp/PgDn to navigate)",
                colors.accent_yellow,
            );
        }

        // Calculate which lines to display based on scroll offset
        let output_start_y = 430;
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
                // Handle longer lines by truncating if needed for display
                let display_line = if trimmed_line.len() > 80 {
                    &trimmed_line[..80]
                } else {
                    trimmed_line
                };

                draw_text(
                    Position::new(margin + 30, output_start_y + line_count as i32 * 18),
                    display_line,
                    colors.text_primary,
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
                colors.border_color,
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
                colors.accent_cyan,
            );
        }

        // Command prompt section with enhanced styling
        let prompt_y = output_start_y + max_display_lines as i32 * 18 + 30;

        // Draw prompt background
        fill_rectangle(
            Position::new(margin + 15, prompt_y - 5),
            content_width - 30,
            30,
            colors.bg_accent,
        );
        draw_rectangle(
            Position::new(margin + 15, prompt_y - 5),
            content_width - 30,
            30,
            colors.border_color,
        );

        draw_text(
            Position::new(margin + 25, prompt_y + 5),
            "user@agave:~$",
            colors.accent_blue,
        );

        // Current command
        if TERMINAL.command_length > 0 {
            let cmd_str = core::str::from_utf8(&TERMINAL.command_buffer[..TERMINAL.command_length])
                .unwrap_or("???");
            draw_text(
                Position::new(margin + 140, prompt_y + 5),
                cmd_str,
                colors.text_primary,
            );
        }

        // Enhanced blinking cursor
        if CURSOR_BLINK {
            let cursor_x = margin + 140 + TERMINAL.command_length as i32 * 8;
            draw_text(
                Position::new(cursor_x, prompt_y + 5),
                "▊",
                colors.accent_cyan,
            );
        }
    }
}

fn draw_files_screen(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);

    unsafe {
        // Draw main content card
        draw_card(
            Position::new(margin - 20, 40),
            content_width + 40,
            dim.height - 140,
            colors,
        );

        // Header with icon
        draw_text(
            Position::new(margin + 20, 70),
            "📁 File System Browser",
            colors.accent_blue,
        );

        draw_text(
            Position::new(margin + 20, 100),
            "Current Directory:",
            colors.text_secondary,
        );
        draw_text(
            Position::new(margin + 180, 100),
            "/home/user",
            colors.accent_cyan,
        );

        // Table header with better styling
        draw_section_divider(Position::new(margin, 130), content_width, colors);

        fill_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            colors.bg_accent,
        );

        draw_text(
            Position::new(margin + 25, 155),
            "Name",
            colors.accent_purple,
        );
        draw_text(
            Position::new(margin + 320, 155),
            "Size",
            colors.accent_purple,
        );
        draw_text(
            Position::new(margin + 420, 155),
            "Type",
            colors.accent_purple,
        );

        draw_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            colors.border_color,
        );

        // File listing with alternating backgrounds and scrolling support
        let max_visible_files = 15; // Limit visible files to fit on screen
        let files_to_show = TERMINAL.file_count.min(max_visible_files);

        for i in 0..files_to_show {
            let file = &TERMINAL.file_system[i];
            let y = 190 + i as i32 * 30;

            // Alternating row backgrounds
            if i % 2 == 0 {
                fill_rectangle(
                    Position::new(margin + 10, y - 5),
                    content_width - 20,
                    30,
                    RGBA::new(
                        colors.bg_secondary.r,
                        colors.bg_secondary.g,
                        colors.bg_secondary.b,
                        128,
                    ),
                );
            }

            let (icon, name_color) = if file.is_directory {
                ("📂", colors.accent_blue)
            } else {
                ("📄", colors.text_primary)
            };

            draw_text(Position::new(margin + 25, y + 5), icon, colors.text_primary);
            draw_text(Position::new(margin + 50, y + 5), file.name, name_color);

            if !file.is_directory {
                let size_str = if file.size > 1024 * 1024 {
                    "Large"
                } else if file.size > 1024 {
                    "Medium"
                } else {
                    "Small"
                };
                draw_text(
                    Position::new(margin + 320, y + 5),
                    size_str,
                    colors.text_secondary,
                );
                draw_text(
                    Position::new(margin + 420, y + 5),
                    "File",
                    colors.text_secondary,
                );
            } else {
                draw_text(
                    Position::new(margin + 420, y + 5),
                    "Directory",
                    colors.accent_blue,
                );
            }
        }

        // Show file count and scroll indicator if needed
        if TERMINAL.file_count > max_visible_files {
            let count_y = 190 + max_visible_files as i32 * 30 + 20;
            let count_text = if TERMINAL.file_count == 80 {
                "... and more files (80 total entries)"
            } else {
                "... and more files"
            };
            draw_text(
                Position::new(margin + 25, count_y),
                count_text,
                colors.text_muted,
            );
        }

        // Instructions
        draw_section_divider(
            Position::new(margin, dim.height - 120),
            content_width,
            colors,
        );
        draw_text(
            Position::new(margin + 20, dim.height - 95),
            "⏎ Press Enter to return to main screen",
            colors.text_muted,
        );
    }
}

fn draw_processes_screen(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);

    unsafe {
        // Draw main content card
        draw_card(
            Position::new(margin - 20, 40),
            content_width + 40,
            dim.height - 140,
            colors,
        );

        // Header with icon
        draw_text(
            Position::new(margin + 20, 70),
            "⚙️ Process Manager",
            colors.accent_red,
        );

        // Performance indicator
        draw_text(
            Position::new(margin + 20, 100),
            "System Load:",
            colors.text_secondary,
        );
        draw_text(
            Position::new(margin + 130, 100),
            "Normal",
            colors.accent_green,
        );

        // Table header
        draw_section_divider(Position::new(margin, 130), content_width, colors);

        fill_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            colors.bg_accent,
        );

        draw_text(Position::new(margin + 25, 155), "PID", colors.accent_purple);
        draw_text(
            Position::new(margin + 80, 155),
            "Name",
            colors.accent_purple,
        );
        draw_text(
            Position::new(margin + 250, 155),
            "Status",
            colors.accent_purple,
        );
        draw_text(
            Position::new(margin + 350, 155),
            "Memory",
            colors.accent_purple,
        );

        draw_rectangle(
            Position::new(margin + 10, 145),
            content_width - 20,
            35,
            colors.border_color,
        );

        // Process listing with enhanced styling and scrolling support
        let max_visible_processes = 12; // Limit visible processes to fit on screen
        let processes_to_show = TERMINAL.process_count.min(max_visible_processes);

        for i in 0..processes_to_show {
            let proc = &TERMINAL.processes[i];
            let y = 190 + i as i32 * 32;

            // Alternating row backgrounds
            if i % 2 == 0 {
                fill_rectangle(
                    Position::new(margin + 10, y - 5),
                    content_width - 20,
                    32,
                    RGBA::new(
                        colors.bg_secondary.r,
                        colors.bg_secondary.g,
                        colors.bg_secondary.b,
                        128,
                    ),
                );
            }

            // PID formatting for larger numbers
            let pid_str = if proc.pid < 10 {
                match proc.pid {
                    1 => "001",
                    2 => "002",
                    3 => "003",
                    4 => "004",
                    5 => "005",
                    6 => "006",
                    7 => "007",
                    8 => "008",
                    9 => "009",
                    _ => "0??",
                }
            } else if proc.pid < 100 {
                match proc.pid {
                    10 => "010",
                    11 => "011",
                    12 => "012",
                    13 => "013",
                    14 => "014",
                    15 => "015",
                    16 => "016",
                    17 => "017",
                    18 => "018",
                    19 => "019",
                    20 => "020",
                    21 => "021",
                    22 => "022",
                    23 => "023",
                    24 => "024",
                    25 => "025",
                    26 => "026",
                    27 => "027",
                    28 => "028",
                    29 => "029",
                    30 => "030",
                    31 => "031",
                    32 => "032",
                    _ => "???",
                }
            } else {
                "???"
            };
            draw_text(
                Position::new(margin + 25, y + 5),
                pid_str,
                colors.text_secondary,
            );

            // Process name with icon
            let proc_icon = match proc.name {
                "init" => "🚀",
                "kernel" => "🔧",
                "virtio-input" => "⌨️",
                "wasm-runtime" => "📦",
                "framebuffer" => "🖥️",
                "memory-mgr" => "💾",
                "task-executor" => "⚡",
                "terminal-app" => "�",
                "filesystem" => "📁",
                "network-stack" => "🌐",
                "audio-driver" => "🔊",
                "graphics-accel" => "🎮",
                "security-mgr" => "🔒",
                "power-mgr" => "🔋",
                "interrupt-hdl" => "⚡",
                "scheduler" => "�",
                "device-mgr" => "🔌",
                "io-subsystem" => "📤",
                "cache-mgr" => "💿",
                "crypto-engine" => "�",
                "vm-manager" => "🖥️",
                "backup-daemon" => "💾",
                "log-collector" => "📄",
                "perf-monitor" => "�",
                "user-session" => "👤",
                "service-mgr" => "⚙️",
                "event-loop" => "🔄",
                "debug-agent" => "🐛",
                "thermal-ctrl" => "🌡️",
                "watchdog" => "🐕",
                "profiler" => "�",
                "health-check" => "🏥",
                _ => "⚙️",
            };

            draw_text(
                Position::new(margin + 80, y + 5),
                proc_icon,
                colors.text_primary,
            );
            draw_text(
                Position::new(margin + 105, y + 5),
                proc.name,
                colors.accent_cyan,
            );

            // Status with colored indicator
            let (status_color, status_icon) = if proc.status == "running" {
                (colors.accent_green, "●")
            } else if proc.status == "idle" {
                (colors.accent_yellow, "⏸")
            } else {
                (colors.accent_red, "✗")
            };

            draw_text(
                Position::new(margin + 250, y + 5),
                status_icon,
                status_color,
            );
            draw_text(
                Position::new(margin + 270, y + 5),
                proc.status,
                status_color,
            );

            // Memory usage with bar visualization
            let mem_str = if proc.memory > 8192 {
                "VHigh"
            } else if proc.memory > 4096 {
                "High"
            } else if proc.memory > 1024 {
                "Med"
            } else {
                "Low"
            };

            let mem_color = if proc.memory > 8192 {
                colors.accent_red
            } else if proc.memory > 4096 {
                colors.accent_yellow
            } else if proc.memory > 1024 {
                colors.accent_blue
            } else {
                colors.accent_green
            };

            draw_text(Position::new(margin + 350, y + 5), mem_str, mem_color);

            // Memory usage bar
            let bar_width = (proc.memory / 200).min(60) as i32; // Adjusted for higher memory values
            fill_rectangle(Position::new(margin + 390, y + 10), bar_width, 6, mem_color);
            draw_rectangle(
                Position::new(margin + 390, y + 10),
                60,
                6,
                colors.border_color,
            );
        }

        // Show process count and scroll indicator if needed
        if TERMINAL.process_count > max_visible_processes {
            let count_y = 190 + max_visible_processes as i32 * 32 + 20;
            draw_text(
                Position::new(margin + 25, count_y),
                "... and more processes (32 total)",
                colors.text_muted,
            );
        }

        // Instructions
        draw_section_divider(
            Position::new(margin, dim.height - 120),
            content_width,
            colors,
        );
        draw_text(
            Position::new(margin + 20, dim.height - 95),
            "⏎ Press Enter to return to main screen",
            colors.text_muted,
        );
    }
}

fn draw_system_screen(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);

    // Draw main content card
    draw_card(
        Position::new(margin - 20, 40),
        content_width + 40,
        dim.height - 140,
        colors,
    );

    // Header with icon
    draw_text(
        Position::new(margin + 20, 70),
        "🖥️ System Information",
        colors.accent_yellow,
    );

    // System overview section
    draw_section_header(Position::new(margin + 20, 110), "Operating System", colors);

    let os_info = [
        ("OS:", "Agave OS v1.0.0", colors.accent_cyan),
        ("Architecture:", "x86_64", colors.text_primary),
        ("Kernel:", "Custom Rust Kernel", colors.accent_purple),
        ("Runtime:", "WASM + Native", colors.accent_green),
    ];

    for (i, (label, value, color)) in os_info.iter().enumerate() {
        let y = 140 + i as i32 * 25;
        draw_text(Position::new(margin + 40, y), label, colors.text_secondary);
        draw_text(Position::new(margin + 200, y), value, *color);
    }

    // Hardware section
    draw_section_header(
        Position::new(margin + 20, 250),
        "Hardware Resources",
        colors,
    );

    let hw_info = [
        ("Memory:", "512 MB Total", colors.accent_blue),
        ("Graphics:", "Direct Framebuffer", colors.accent_green),
        ("Input:", "VirtIO Mouse/Keyboard", colors.text_primary),
        ("Storage:", "Virtual Disk", colors.text_primary),
    ];

    for (i, (label, value, color)) in hw_info.iter().enumerate() {
        let y = 280 + i as i32 * 25;
        draw_text(Position::new(margin + 40, y), label, colors.text_secondary);
        draw_text(Position::new(margin + 200, y), value, *color);
    }

    // Status indicators
    draw_section_header(Position::new(margin + 20, 390), "System Status", colors);

    let status_items = [
        ("Uptime:", "Running", colors.accent_green, "✓"),
        ("Status:", "Operational", colors.accent_green, "✓"),
        ("Load:", "Normal", colors.accent_yellow, "●"),
        ("Network:", "Not Available", colors.accent_red, "✗"),
    ];

    for (i, (label, value, color, icon)) in status_items.iter().enumerate() {
        let y = 420 + i as i32 * 25;
        draw_text(Position::new(margin + 40, y), label, colors.text_secondary);
        draw_text(Position::new(margin + 150, y), icon, *color);
        draw_text(Position::new(margin + 180, y), value, *color);
    }

    // Instructions
    draw_section_divider(
        Position::new(margin, dim.height - 120),
        content_width,
        colors,
    );
    draw_text(
        Position::new(margin + 20, dim.height - 95),
        "⏎ Press Enter to return to main screen",
        colors.text_muted,
    );
}

fn draw_help_screen(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    let margin = 60;
    let content_width = dim.width - (margin * 2);

    // Draw main content card
    draw_card(
        Position::new(margin - 20, 40),
        content_width + 40,
        dim.height - 140,
        colors,
    );

    // Header with icon
    draw_text(
        Position::new(margin + 20, 70),
        "❓ Agave OS - Help",
        colors.accent_yellow,
    );

    draw_text(
        Position::new(margin + 20, 100),
        "Command Reference Guide",
        colors.text_secondary,
    );

    let col1_x = margin + 30;
    let col2_x = margin + 350;
    let mut current_y = 140;

    // File System Commands
    draw_section_header(Position::new(col1_x, current_y), "📁 File System", colors);
    current_y += 30;

    let file_commands = [("ls", "List files and directories")];

    for (cmd, desc) in file_commands.iter() {
        draw_text(
            Position::new(col1_x + 20, current_y),
            cmd,
            colors.accent_cyan,
        );
        draw_text(
            Position::new(col1_x + 60, current_y),
            "-",
            colors.text_muted,
        );
        draw_text(
            Position::new(col1_x + 80, current_y),
            desc,
            colors.text_primary,
        );
        current_y += 22;
    }

    current_y += 10;

    // System Commands
    draw_section_header(
        Position::new(col1_x, current_y),
        "⚙️ System Commands",
        colors,
    );
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
        draw_text(
            Position::new(col1_x + 20, current_y),
            cmd,
            colors.accent_cyan,
        );
        draw_text(
            Position::new(col1_x + 80, current_y),
            "-",
            colors.text_muted,
        );
        draw_text(
            Position::new(col1_x + 100, current_y),
            desc,
            colors.text_primary,
        );
        current_y += 22;
    }

    // Right column - Utility Commands
    current_y = 170;
    draw_section_header(
        Position::new(col2_x, current_y),
        "🔧 Utility Commands",
        colors,
    );
    current_y += 30;

    let utility_commands = [
        ("help", "Show this help screen"),
        ("clear", "Clear terminal output"),
        ("reset", "Reset terminal state"),
        ("echo", "Echo text to output"),
        ("theme", "Change color themes"),
        ("main", "Return to main screen"),
        ("exit", "Exit the terminal"),
    ];

    for (cmd, desc) in utility_commands.iter() {
        draw_text(
            Position::new(col2_x + 20, current_y),
            cmd,
            colors.accent_cyan,
        );
        draw_text(
            Position::new(col2_x + 70, current_y),
            "-",
            colors.text_muted,
        );
        draw_text(
            Position::new(col2_x + 90, current_y),
            desc,
            colors.text_primary,
        );
        current_y += 22;
    }

    current_y += 20;

    // Theme commands section
    draw_section_header(
        Position::new(col2_x, current_y),
        "🎨 Theme Commands",
        colors,
    );
    current_y += 25;

    let theme_commands = [
        ("theme", "Show current theme"),
        ("theme list", "List all themes"),
        ("theme <name>", "Switch to theme"),
        ("theme next", "Next theme"),
        ("theme prev", "Previous theme"),
    ];

    for (cmd, desc) in theme_commands.iter() {
        draw_text(
            Position::new(col2_x + 20, current_y),
            cmd,
            colors.accent_purple,
        );
        draw_text(
            Position::new(col2_x + 110, current_y),
            "-",
            colors.text_muted,
        );
        draw_text(
            Position::new(col2_x + 130, current_y),
            desc,
            colors.text_secondary,
        );
        current_y += 20;
    }
    // Instructions
    draw_section_divider(
        Position::new(margin, dim.height - 120),
        content_width,
        colors,
    );
    draw_text(
        Position::new(margin + 20, dim.height - 95),
        "⏎ Press Enter to return to main screen",
        colors.text_muted,
    );
}

fn draw_status_bar(dim: agave_lib::Dimensions, colors: &ThemeColors) {
    let status_y = dim.height - 40;
    let status_height = 40;

    // Enhanced status bar background with gradient effect
    fill_rectangle(
        Position::new(0, status_y),
        dim.width,
        status_height,
        colors.bg_secondary,
    );

    // Top border with accent color
    fill_rectangle(Position::new(0, status_y), dim.width, 2, colors.accent_cyan);

    // Subtle inner border
    draw_rectangle(
        Position::new(0, status_y),
        dim.width,
        status_height,
        colors.border_color,
    );

    unsafe {
        // Current screen indicator with enhanced styling
        let (screen_text, screen_color) = match TERMINAL.current_screen {
            Screen::Main => ("● MAIN", colors.accent_green),
            Screen::Files => ("📁 FILES", colors.accent_blue),
            Screen::Processes => ("⚙️ PROCESSES", colors.accent_red),
            Screen::System => ("🖥️ SYSTEM", colors.accent_yellow),
            Screen::Help => ("❓ HELP", colors.accent_purple),
        };

        draw_text(Position::new(25, status_y + 12), screen_text, screen_color);

        // Separator
        draw_text(Position::new(160, status_y + 12), "│", colors.border_color);

        // Application title
        draw_text(
            Position::new(180, status_y + 12),
            "Agave OS",
            colors.text_primary,
        );

        // Separator
        draw_text(Position::new(380, status_y + 12), "│", colors.border_color);

        // Command buffer indicator with better styling
        let (cmd_text, cmd_color) = if TERMINAL.command_length > 0 {
            ("⌨️ TYPING", colors.accent_yellow)
        } else {
            ("✓ READY", colors.accent_green)
        };

        draw_text(Position::new(400, status_y + 12), cmd_text, cmd_color);

        // Theme indicator
        draw_text(Position::new(500, status_y + 12), "│", colors.border_color);

        // Show current theme name
        #[allow(static_mut_refs)]
        let theme_text = TERMINAL.current_theme.name();
        draw_text(
            Position::new(520, status_y + 12),
            "🎨",
            colors.accent_purple,
        );
        draw_text(
            Position::new(545, status_y + 12),
            theme_text,
            colors.accent_purple,
        );

        // Right side indicators
        let right_x = dim.width - 320;

        // Separator
        draw_text(
            Position::new(right_x + 70, status_y + 12),
            "│",
            colors.border_color,
        );

        // Real-time clock (HH:MM:SS)
        let total_seconds = TERMINAL.uptime / 1000;
        let seconds = total_seconds % 60;
        let minutes = (total_seconds / 60) % 60;
        let hours = (total_seconds / 3600) % 24;
        let clock_str = format!("{:02}:{:02}:{:02}", hours, minutes, seconds);
        draw_text(
            Position::new(right_x + 90, status_y + 12),
            &clock_str,
            colors.accent_blue,
        );

        // Separator
        draw_text(
            Position::new(right_x + 160, status_y + 12),
            "│",
            colors.border_color,
        );

        // System heartbeat indicator
        let heartbeat_color = if ANIMATION_FRAME % 120 < 20 {
            colors.accent_red
        } else if ANIMATION_FRAME % 120 < 40 {
            RGBA::new(
                colors.accent_red.r,
                colors.accent_red.g,
                colors.accent_red.b,
                180,
            )
        } else {
            RGBA::new(
                colors.accent_red.r,
                colors.accent_red.g,
                colors.accent_red.b,
                80,
            )
        };

        draw_text(
            Position::new(right_x + 180, status_y + 12),
            "♥",
            heartbeat_color,
        );

        // System status
        draw_text(
            Position::new(right_x + 205, status_y + 12),
            "Online",
            colors.accent_green,
        );
    }
}

// Helper functions for drawing UI components
// IMPORTANT: These functions maintain the visual consistency of the terminal UI
// Do not modify spacing, colors, or layout without updating the design system above

fn draw_card(pos: Position, width: i32, height: i32, colors: &ThemeColors) {
    // Card background with consistent styling
    fill_rectangle(pos, width, height, colors.bg_accent);

    // Card border for definition
    draw_rectangle(pos, width, height, colors.border_color);

    // Subtle inner shadow effect for depth
    draw_rectangle(
        Position::new(pos.x + 1, pos.y + 1),
        width - 2,
        height - 2,
        RGBA::new(255, 255, 255, 10),
    );
}

fn draw_section_divider(pos: Position, width: i32, colors: &ThemeColors) {
    // Main divider line with consistent styling
    fill_rectangle(pos, width, 1, colors.border_color);

    // Subtle highlight above for depth
    fill_rectangle(
        Position::new(pos.x, pos.y - 1),
        width,
        1,
        RGBA::new(255, 255, 255, 20),
    );
}

fn draw_section_header(pos: Position, text: &str, colors: &ThemeColors) {
    // Section header with professional styling
    draw_text(pos, text, colors.accent_purple);

    // Underline for emphasis (maintain 8px character width assumption)
    let text_width = text.len() as i32 * 8;
    fill_rectangle(
        Position::new(pos.x, pos.y + 18),
        text_width,
        2,
        colors.accent_purple,
    );
}
