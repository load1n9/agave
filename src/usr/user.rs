use crate::{api, sys, usr};
use crate::api::console::Style;
use crate::api::fs;
use crate::api::io;
use crate::api::random;
use crate::api::process::ExitCode;
use crate::api::syscall;
use alloc::collections::btree_map::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::convert::TryInto;
use core::str;
use sha2::Sha256;

const USERS: &str = "/ini/users.csv";
const DISABLE_EMPTY_PASSWORD: bool = false;

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    match *args.get(1).unwrap_or(&"invalid") {
        "create" => {},
        "login" => {},
        "-h" | "--help" => {
            help();
            return Ok(());
        }
        _ => {
            help();
            return Err(ExitCode::UsageError);
        }
    }

    let username: String = if args.len() == 2 {
        print!("Username: ");
        io::stdin().read_line().trim_end().to_string()
    } else {
        args[2].to_string()
    };

    match args[1] {
        "create" => create(&username),
        "login" => login(&username),
        _ => unreachable!(),
    }
}

// TODO: Add max number of attempts
pub fn login(username: &str) -> Result<(), ExitCode> {
    if !fs::exists(USERS) {
        error!("Could not read '{}'", USERS);
        return Err(ExitCode::Failure);
    }

    if username.is_empty() {
        println!();
        syscall::sleep(1.0);
        return main(&["user", "login"]);
    }

    match hashed_password(username) {
        Some(hash) => {
            print!("Password: ");
            print!("\x1b[12l"); // Disable echo
            let password = io::stdin().read_line().trim_end().to_string();
            print!("\x1b[12h"); // Enable echo
            println!();
            if !check(&password, &hash) {
                println!();
                syscall::sleep(1.0);
                return main(&["user", "login"]);
            }
        },
        None => {
            println!();
            syscall::sleep(1.0);
            return main(&["user", "login"]);
        },
    }

    let home = format!("/usr/{}", username);
    sys::process::set_user(username);
    sys::process::set_dir(&home);
    sys::process::set_env("USER", username);
    sys::process::set_env("HOME", &home);

    // TODO: load shell
    Ok(())
}

pub fn create(username: &str) -> Result<(), ExitCode> {
    if username.is_empty() {
        return Err(ExitCode::Failure);
    }

    if hashed_password(username).is_some() {
        error!("Username exists");
        return Err(ExitCode::Failure);
    }

    print!("Password: ");
    print!("\x1b[12l"); // Disable echo
    let password = io::stdin().read_line().trim_end().to_string();
    print!("\x1b[12h"); // Enable echo
    println!();

    if password.is_empty() && DISABLE_EMPTY_PASSWORD {
        return Err(ExitCode::Failure);
    }

    print!("Confirm: ");
    print!("\x1b[12l"); // Disable echo
    let confirm = io::stdin().read_line().trim_end().to_string();
    print!("\x1b[12h"); // Enable echo
    println!();

    if password != confirm {
        error!("Password confirmation failed");
        return Err(ExitCode::Failure);
    }

    if save_hashed_password(username, &hash(&password)).is_err() {
        error!("Could not save user");
        return Err(ExitCode::Failure);
    }

    // Create home dir
    if let Some(handle) = api::fs::create_dir(&format!("/usr/{}", username)) {
        api::syscall::close(handle);
    } else {
        error!("Could not create home dir");
        return Err(ExitCode::Failure);
    }

    Ok(())
}

pub fn check(password: &str, hashed_password: &str) -> bool {
    let fields: Vec<_> = hashed_password.split('$').collect();
    if fields.len() != 4 || fields[0] != "1" {
        return false;
    }

    let decoded_field = usr::base64::decode(fields[1].as_bytes());
    let c = u32::from_be_bytes(decoded_field[0..4].try_into().unwrap());

    let decoded_field = usr::base64::decode(fields[2].as_bytes());
    let salt: [u8; 16] = decoded_field[0..16].try_into().unwrap();

    let mut hash = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, c, &mut hash);
    let encoded_hash = String::from_utf8(usr::base64::encode(&hash)).unwrap();

    encoded_hash == fields[3]
}

// Password hashing version 1 => PBKDF2-HMAC-SHA256 + BASE64
// Fields: "<version>$<c>$<salt>$<hash>"
// Example: "1$AAAQAA$PDkXP0I8O7SxNOxvUKmHHQ$BwIUWBxKs50BTpH6i4ImF3SZOxADv7dh4xtu3IKc3o8"
pub fn hash(password: &str) -> String {
    let v = "1"; // Password hashing version
    let c = 4096u32; // Number of iterations
    let mut salt = [0u8; 16];
    let mut hash = [0u8; 32];

    // Generating salt
    for i in 0..2 {
        let num = random::get_u64();
        let buf = num.to_be_bytes();
        let n = buf.len();
        for j in 0..n {
            salt[i * n + j] = buf[j];
        }
    }

    // Hashing password with PBKDF2-HMAC-SHA256
    pbkdf2::pbkdf2_hmac::<Sha256>(password.as_bytes(), &salt, c, &mut hash);

    // Encoding in Base64 standard without padding
    let c = c.to_be_bytes();
    let mut res: String = String::from(v);
    res.push('$');
    res.push_str(&String::from_utf8(usr::base64::encode(&c)).unwrap());
    res.push('$');
    res.push_str(&String::from_utf8(usr::base64::encode(&salt)).unwrap());
    res.push('$');
    res.push_str(&String::from_utf8(usr::base64::encode(&hash)).unwrap());
    res
}

fn read_hashed_passwords() -> BTreeMap<String, String> {
    let mut hashed_passwords = BTreeMap::new();
    if let Ok(csv) = api::fs::read_to_string(USERS) {
        for line in csv.split('\n') {
            let mut rows = line.split(',');
            if let Some(username) = rows.next() {
                if let Some(hash) = rows.next() {
                    hashed_passwords.insert(username.into(), hash.into());
                }
            }
        }
    }
    hashed_passwords
}

fn hashed_password(username: &str) -> Option<String> {
    read_hashed_passwords().get(username).map(|hash| hash.into())
}

fn save_hashed_password(username: &str, hash: &str) -> Result<usize, ()> {
    let mut hashed_passwords = read_hashed_passwords();
    hashed_passwords.remove(username);
    hashed_passwords.insert(username.into(), hash.into());

    let mut csv = String::new();
    for (u, h) in hashed_passwords {
        csv.push_str(&format!("{},{}\n", u, h));
    }

    fs::write(USERS, csv.as_bytes())
}

fn help() {
    let csi_option = Style::color("LightCyan");
    let csi_title = Style::color("Yellow");
    let csi_reset = Style::reset();
    println!("{}Usage:{} user {}<command>{}", csi_title, csi_reset, csi_option, csi_reset);
    println!();
    println!("{}Commands:{}", csi_title, csi_reset);
    println!("  {}create [<user>]{}    Create user", csi_option, csi_reset);
    println!("  {}login [<user>]{}     Login user", csi_option, csi_reset);
}
