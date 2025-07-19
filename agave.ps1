param(
    [string]$Command = ""
)

function Show-Help {
    Write-Host ""
    Write-Host "Agave Project Tasks (PowerShell)"
    Write-Host "-------------------------------"
    Write-Host "install-stuff      Install cargo-wasi"
    Write-Host "build              Build the project"
    Write-Host "build-app-terminal Build the terminal app (WASM)"
    Write-Host "run                Run the project build"
    Write-Host "run-qemu           Run QEMU BIOS"
    Write-Host "run-all            Build terminal app and run QEMU"
    Write-Host "qemu               Launch QEMU with custom options"
    Write-Host ""
    Write-Host "Usage: .\agave.ps1 <command>"
    Write-Host "Example: .\agave.ps1 build-app-terminal"
}

switch ($Command) {
    "install-stuff" {
        Write-Host "Installing cargo-wasi..."
        cargo install cargo-wasi
    }
    "build" {
        Write-Host "Building project..."
        cargo run --release build
    }
    "build-app-terminal" {
        Write-Host "Building terminal app (WASM)..."
        Push-Location apps/terminal
        cargo build --release --target wasm32-wasip1
        Pop-Location
    }
    "run" {
        Write-Host "Running project build..."
        cargo run --release build
    }
    "run-qemu" {
        Write-Host "Running QEMU BIOS..."
        cargo run --release --bin qemu-bios
    }
    "run-all" {
        Write-Host "Building terminal app and running QEMU..."
        & .\agave.ps1 build-app-terminal
        & .\agave.ps1 run-qemu
    }
    "qemu" {
        Write-Host "Launching QEMU with custom options..."
        qemu-system-x86_64 -nodefaults -m 2G -smp 2 -device virtio-mouse-pci -device virtio-keyboard-pci -nic user,model=virtio-net-pci -device virtio-vga-gl -display sdl,gl=on -serial stdio -drive format=raw,file=./target/release/uefi.img -bios ovmf
    }
    Default {
        Show-Help
    }
}
