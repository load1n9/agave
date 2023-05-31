use crate::api::fs;
use crate::api::console::Style;
use crate::api::process::ExitCode;

// TODO: add `--skip` and `--length` params
pub fn main(args: &[&str]) -> Result<(), ExitCode> {
    if args.len() != 2 {
        help();
        return Err(ExitCode::UsageError);
    }
    if args[1] == "-h" || args[1] == "--help" {
        help();
        return Ok(());
    }
    let pathname = args[1];
    if let Ok(buf) = fs::read_to_bytes(pathname) { // TODO: read chunks
        print_hex(&buf);
        Ok(())
    } else {
        error!("File not found '{}'", pathname);
        Err(ExitCode::Failure)
    }
}

// TODO: move this to api::hex::print_hex
pub fn print_hex(buf: &[u8]) {
    let n = buf.len() / 2;
    for i in 0..n {
        print!("{}", Style::color("LightCyan"));
        if i % 8 == 0 {
            print!("{:08X}: ", i * 2);
        }
        print!("{}", Style::color("Pink"));
        print!("{:02X}{:02X} ", buf[i * 2], buf[i * 2 + 1]);
        print!("{}", Style::reset());
        if i % 8 == 7 || i == n - 1 {
            for _ in 0..(7 - (i % 8)) {
                print!("     ");
            }
            let m = ((i % 8) + 1) * 2;
            for j in 0..m {
                let c = buf[(i * 2 + 1) - (m - 1) + j] as char;
                if c.is_ascii_graphic() {
                    print!("{}", c);
                } else if c.is_ascii_whitespace() {
                    print!(" ");
                } else {
                    print!(".");
                }
            }
            println!();
        }
    }
}

fn help() {
    let csi_option = Style::color("LightCyan");
    let csi_title = Style::color("Yellow");
    let csi_reset = Style::reset();
    println!("{}Usage:{} hex {}<file>{}", csi_title, csi_reset, csi_option, csi_reset);
}
