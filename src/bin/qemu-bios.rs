use std::{
    env,
    process::{self, Command},
};

fn main() {
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
    qemu.arg(format!("format=raw,file={}", env!("UEFI_IMAGE")));
    qemu.arg("-bios").arg(ovmf_prebuilt::ovmf_pure_efi());
    let exit_status = qemu.status().unwrap();
    process::exit(exit_status.code().unwrap_or(-1));
}
