use std::path::PathBuf;

use bootloader::BootConfig;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let kernel = PathBuf::from(std::env::var_os("CARGO_BIN_FILE_AGAVE_kernel").unwrap());

    let uefi_path = out_dir.join("uefi.img");

    let mut conf = BootConfig::default();
    conf.frame_buffer.minimum_framebuffer_width = Some(1200);
    bootloader::UefiBoot::new(&kernel)
        .set_boot_config(&conf)
        .create_disk_image(&uefi_path)
        .unwrap();

    let bios_path = out_dir.join("bios.img");
    bootloader::BiosBoot::new(&kernel)
        .create_disk_image(&bios_path)
        .unwrap();

    // pass the disk image paths as env variables to the `main.rs`
    println!("cargo:rustc-env=UEFI_PATH={}", uefi_path.display());
    println!("cargo:rustc-env=BIOS_PATH={}", bios_path.display());
}