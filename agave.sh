#!/usr/bin/env bash

show_help() {
  cat <<EOF
Agave Project Tasks (Bash)
-------------------------
install-stuff      Install cargo-wasi
build              Build the project
build-app-terminal Build the terminal app (WASM)
run                Run the project build
run-qemu           Run QEMU BIOS
run-all            Build terminal app and run QEMU
qemu               Launch QEMU with custom options

Usage: ./agave.sh <command>
Example: ./agave.sh build-app-terminal
EOF
}

install_stuff() {
  echo "Installing cargo-wasi..."
  cargo install cargo-wasi
}

build() {
  echo "Building project..."
  cargo run --release build
}

build_app_terminal() {
  echo "Building terminal app (WASM)..."
  cd apps/terminal || exit 1
  cargo build --release --target wasm32-wasip1
  cd ../.. || exit 1
}

run() {
  echo "Running project build..."
  cargo run --release build
}

run_qemu() {
  echo "Running QEMU BIOS..."
  cargo run --release --bin qemu-bios
}

run_all() {
  echo "Building terminal app and running QEMU..."
  ./agave.sh build-app-terminal
  ./agave.sh run-qemu
}

qemu() {
  echo "Launching QEMU with custom options..."
  qemu-system-x86_64 -nodefaults -m 2G -smp 2 -device virtio-mouse-pci -device virtio-keyboard-pci -nic user,model=virtio-net-pci -device virtio-vga-gl -display sdl,gl=on -serial stdio -drive format=raw,file=./target/release/uefi.img -bios ovmf
}

case "$1" in
  install-stuff) install_stuff ;;
  build) build ;;
  build-app-terminal) build_app_terminal ;;
  run) run ;;
  run-qemu) run_qemu ;;
  run-all) run_all ;;
  qemu) qemu ;;
  *) show_help ;;
esac
