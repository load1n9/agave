use alloc::format;

use crate::api::console::Style;
use crate::api::fs;
use crate::api::process::ExitCode;
use crate::api::vga::palette;
use crate::sys;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    if args.len() == 1 {
        help();
        return Err(ExitCode::UsageError);
    }

    match args[1] {
        "-h" | "--help" => {
            help();
            Ok(())
        }
        "set" => {
            if let Ok(csv) = fs::read_to_string(format!("/ini/palettes/{}.csv", args[2]).as_str()) {
                if let Ok(palette) = palette::from_csv(&csv) {
                    sys::vga::set_palette(palette);
                    Ok(())
                } else {
                    error!("Could not parse palette file");
                    Err(ExitCode::Failure)
                }
            } else {
                error!("Could not read palette file");
                Err(ExitCode::Failure)
            }
        }
        _ => {
            error!("Invalid command");
            Err(ExitCode::Failure)
        }
    }
}

fn help() {
    let csi_option = Style::color("LightCyan");
    let csi_title = Style::color("Yellow");
    let csi_reset = Style::reset();
    println!(
        "{}Usage:{} palette {}<command>{1}",
        csi_title, csi_reset, csi_option
    );
    println!();
    println!("{}Commands:{}", csi_title, csi_reset);
    println!(
        "  {}set <theme>{}    Set color palette",
        csi_option, csi_reset
    );
}
