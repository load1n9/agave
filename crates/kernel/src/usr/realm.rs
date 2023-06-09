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
                error!("\nInvalid command");
                return Err(ExitCode::Failure);
            }
            let realm = args[2];
            if realm == "bin" {
                error!("\nInvalid realm name");
                return Err(ExitCode::Failure);
            }
            if crate::sys::realm::realm_exists(realm) {
                error!("\nRealm already exists");
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::create_realm(realm);
            println!(
                "\n {}created realm {}{}{} {}successfully{}",
                Style::color("LightGreen"),
                Style::color("Green"),
                realm,
                Style::reset(),
                Style::color("LightGreen"),
                Style::reset()
            );
            Ok(())
        }
        "enter" => {
            if args.len() < 3 {
                error!("\nInvalid command");
                return Err(ExitCode::Failure);
            }
            let realm = args[2];
            if realm == "bin" {
                error!("\nInvalid realm name `{}`", realm);
                return Err(ExitCode::Failure);
            }
            if !crate::sys::realm::realm_exists(realm) {
                error!("\nRealm `{}` does not exist", realm);
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::enter_realm(realm);
            println!(
                "\n{}entered {}`{}`{} {}realm {}",
                Style::color("LightGreen"),
                Style::color("Green"),
                realm,
                Style::reset(),
                Style::color("LightGreen"),
                Style::reset()
            );

            Ok(())
        }
        "exit" => {
            let realm = crate::sys::realm::get_current_realm().clone();
            if realm == "bin" {
                error!("\nNo active realm");
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::exit_realm();

            println!(
                "{}\nExited {}`{}`{} {}realm{}",
                Style::color("LightGreen"),
                Style::color("Green"),
                realm,
                Style::reset(),
                Style::color("LightGreen"),
                Style::reset(),
            );
            Ok(())
        }
        "delete" => {
            if args.len() < 3 {
                error!("\nInvalid command");
                return Err(ExitCode::Failure);
            }
            let realm = args[2];
            if realm == "bin" {
                error!("\nInvalid realm name");
                return Err(ExitCode::Failure);
            }
            if !crate::sys::realm::realm_exists(realm) {
                error!("\nRealm does not exist");
                return Err(ExitCode::Failure);
            }
            crate::sys::realm::delete_realm(realm);
            println!("Realm {} deleted", realm);
            Ok(())
        }
        "active" => {
            let realm = crate::sys::realm::get_current_realm();
            if realm == "bin" {
                println!(
                    "\n{}No active realm{}",
                    Style::color("LightGray"),
                    Style::reset()
                );
                Ok(())
            } else {
                println!(
                    "\n{}Active realm: {}{}{}",
                    Style::color("LightGreen"),
                    Style::color("Green"),
                    realm,
                    Style::reset()
                );
                Ok(())
            }
        }
        "list" => {
            let realms = crate::sys::realm::get_realms();
            println!("\nRealms:");
            for realm in realms {
                println!("-  {}", realm);
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
