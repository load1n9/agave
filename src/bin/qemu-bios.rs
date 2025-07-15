use ovmf_prebuilt::{Arch, FileType, Prebuilt, Source};
use std::{
    env,
    process::{self, Command},
};

fn main() {
    // Get the workspace root directory
    let workspace_root = env::current_dir().expect("Failed to get current directory");
    let ovmf_cache_dir = workspace_root.join("target").join("ovmf");

    // Fetch and cache the OVMF prebuilt
    println!("Fetching OVMF prebuilt to: {}", ovmf_cache_dir.display());
    let prebuilt =
        Prebuilt::fetch(Source::LATEST, &ovmf_cache_dir).expect("failed to fetch OVMF prebuilt");

    // Get the path to the OVMF code file for x64 architecture
    let ovmf_code_path = prebuilt.get_file(Arch::X64, FileType::Code);

    // Debug: Print the OVMF path to see what we're getting
    println!("OVMF code path: {}", ovmf_code_path.display());

    // Verify the file exists
    if !ovmf_code_path.exists() {
        eprintln!(
            "Error: OVMF code file does not exist at: {}",
            ovmf_code_path.display()
        );
        process::exit(1);
    }

    // Convert to string and normalize path separators for QEMU
    let ovmf_path_str = ovmf_code_path.to_string_lossy().replace('\\', "/");
    println!("OVMF path for QEMU: {ovmf_path_str}");

    let mut qemu = Command::new("qemu-system-x86_64");
    qemu.arg("-nodefaults");
    qemu.arg("-m").arg("600M");
    qemu.arg("-smp").arg("2");
    qemu.arg("-device").arg("virtio-mouse-pci");
    qemu.arg("-device").arg("virtio-keyboard-pci");
    qemu.arg("-nic").arg("user,model=virtio-net-pci");
    qemu.arg("-device").arg("virtio-vga-gl");
    qemu.arg("-display").arg("sdl,gl=on");
    qemu.arg("-serial").arg("stdio");
    qemu.arg("-drive");
    qemu.arg(format!("format=raw,file={}", env!("UEFI_PATH")));
    // Use pflash for UEFI firmware instead of -bios
    qemu.arg("-drive");
    qemu.arg(format!(
        "if=pflash,format=raw,file={ovmf_path_str},readonly=on"
    ));
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}
