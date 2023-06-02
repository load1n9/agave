use alloc::{format, string::ToString};

use crate::{
    api::{console::Style, fs, process::ExitCode, wasm::WasmInstance},
    sys,
};

pub fn read_wasm_string(offset: u32, length: u32, wasm_mem: &[u8]) -> &str {
    ::core::str::from_utf8(&wasm_mem[offset as usize..offset as usize + length as usize])
        .expect("read_wasm_cstring failed to parse invalid utf-8 string")
}

pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    if args.len() != 2 {
        help();
        return Err(ExitCode::UsageError);
    }

    if args[1] == "-h" || args[1] == "--help" {
        help();
        return Ok(());
    }

    let filename = if args[1].starts_with("./") {
        format!("/{}", args[1].strip_prefix("./").unwrap())
    } else if args[1].starts_with("/") {
        args[1].to_string()
    } else {
        format!("/{}", args[1])
    };

    let pathname = format!("{}{}", &sys::process::dir(), filename);

    if let Ok(buf) = fs::read_to_bytes(&pathname) {
        let mut instance = WasmInstance::new(buf);
        instance.start();
        Ok(())
    } else {
        error!("File not found '{}'", pathname);
        Err(ExitCode::Failure)
    }
}

fn help() {
    let csi_option = Style::color("LightCyan");
    let csi_title = Style::color("Yellow");
    let csi_reset = Style::reset();
    println!(
        "{}Usage:{} run {}<file>{}",
        csi_title, csi_reset, csi_option, csi_reset
    );
}
