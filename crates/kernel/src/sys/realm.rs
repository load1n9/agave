use alloc::{format, string::String, vec::Vec};

/// Get the current realm path if it exists otherwise return the bin path
pub fn get_current_realm() -> String {
    match crate::sys::process::env("$SYS:REALM") {
        Some(name) => name,
        None => String::from("bin"),
    }
}

pub fn realm_exists(realm: &str) -> bool {
    if realm == "bin" {
        return false;
    }
    crate::api::fs::exists(format!("/realms/{}", realm).as_str())
}

pub fn get_realm_path(realm: &str) -> String {
    if realm == "bin" {
        String::from("/bin")
    } else {
        format!("/realms/{}", realm)
    }
}

pub fn create_realm(realm: &str) {
    if !(realm_exists(realm) || realm == "bin") {
        let pathname = format!("/realms/{}", realm);
        crate::api::fs::create_dir(pathname.as_str());
    }
}

pub fn delete_realm(realm: &str) -> bool {
    if !realm_exists(realm) {
        false
    } else {
        crate::api::fs::delete(format!("/realms/{}", realm).as_str()).is_ok()
    }
}

pub fn enter_realm(realm: &str) -> bool {
    if !realm_exists(realm) {
        false
    } else {
        crate::sys::process::set_env("$SYS:REALM", realm);
        true
    }
}

pub fn exit_realm() {
    crate::sys::process::delete_env("$SYS:REALM")
}

pub fn get_realms() -> Vec<String> {
    let mut realms = Vec::new();
    for entry in crate::api::fs::read_dir("/realms").unwrap() {
        let entry = entry;
        let path = entry.name();
        let path = path.as_str();
        let realm = path.replace("/realms/", "");
        realms.push(realm);
    }
    realms
}
