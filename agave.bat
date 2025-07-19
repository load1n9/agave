@echo off

IF "%1"=="install-stuff" GOTO install_stuff
IF "%1"=="build" GOTO build
IF "%1"=="build-app-terminal" GOTO build_app_terminal
IF "%1"=="run" GOTO run
IF "%1"=="run-qemu" GOTO run_qemu
IF "%1"=="run-all" GOTO run_all
IF "%1"=="qemu" GOTO qemu

:help
echo.
echo Agave Project Tasks (Batch)
echo --------------------------
echo install-stuff      Install cargo-wasi
echo build              Build the project
echo build-app-terminal Build the terminal app (WASM)
echo run                Run the project build
echo run-qemu           Run QEMU BIOS
echo run-all            Build terminal app and run QEMU
echo qemu               Launch QEMU with custom options
echo.
echo Usage: agave.bat ^<command^>
echo Example: agave.bat build-app-terminal
GOTO end

:install_stuff
echo Installing cargo-wasi...
cargo install cargo-wasi
GOTO end

:build
echo Building project...
cargo run --release build
GOTO end

:build_app_terminal
echo Building terminal app (WASM)...
cd apps\terminal
cargo build --release --target wasm32-wasip1
cd ..\..
GOTO end

:run
echo Running project build...
cargo run --release build
GOTO end

:run_qemu
echo Running QEMU BIOS...
cargo run --release --bin qemu-bios
GOTO end

:run_all
echo Building terminal app and running QEMU...
call "%~f0" build-app-terminal
call "%~f0" run-qemu
GOTO end

:qemu
echo Launching QEMU with custom options...
qemu-system-x86_64 -nodefaults -m 2G -smp 2 -device virtio-mouse-pci -device virtio-keyboard-pci -nic user,model=virtio-net-pci -device virtio-vga-gl -display sdl,gl=on -serial stdio -drive format=raw,file=./target/release/uefi.img -bios ovmf
GOTO end

:end
