use alloc::format;

use crate::api::console::Style;
use crate::api::process::ExitCode;

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
        "create" => {
            if args.len() < 3 {
                error!("Invalid command");
                return Err(ExitCode::Failure);
            }
            let realm = args[2];
            if realm == "bin" {
                error!("Invalid realm name");
                return Err(ExitCode::Failure);
            }
            if crate::sys::realm::realm_exists(realm) {
                error!("Realm already exists");
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::create_realm(realm);
            println!("Realm {} created successfully", realm);
            Ok(())
        }
        "enter" => {
            if args.len() < 3 {
                error!("Invalid command");
                return Err(ExitCode::Failure);
            }
            let realm = args[2];
            if realm == "bin" {
                error!("Invalid realm name");
                return Err(ExitCode::Failure);
            }
            if !crate::sys::realm::realm_exists(realm) {
                error!("Realm does not exist");
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::enter_realm(realm);
            println!("Realm {} entered", realm);

            Ok(())
        }
        "exit" => {
            crate::sys::realm::exit_realm();
            println!("Exited realm");
            Ok(())
        }
        "delete" => {
            if args.len() < 3 {
                error!("Invalid command");
                return Err(ExitCode::Failure);
            }
            let realm = args[2];
            if realm == "bin" {
                error!("Invalid realm name");
                return Err(ExitCode::Failure);
            }
            if !crate::sys::realm::realm_exists(realm) {
                error!("Realm does not exist");
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::delete_realm(realm);
            println!("Realm {} deleted", realm);
            Ok(())
        }
        "list" => {
            let realms = crate::sys::realm::get_realms();
            println!("Realms:");
            for realm in realms {
                println!("  {}", realm);
            }
            Ok(())
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
        "{}Usage:{} realm {}<command>{1}",
        csi_title, csi_reset, csi_option
    );
    println!();
    println!("{}Commands:{}", csi_title, csi_reset);
    println!(
        "{}create <realm>{}    Creates a new realm",
        csi_option, csi_reset
    );
    println!(
        "{}enter <realm>{}    Enters the given realm",
        csi_option, csi_reset
    );
    println!(
        "{}exit <realm>{}    Exits the given realm",
        csi_option, csi_reset
    );
    println!(
        "{}delete <realm>{}    Deletes the given realm",
        csi_option, csi_reset
    );
    println!("{}list{}    Lists all the realms", csi_option, csi_reset);
}
