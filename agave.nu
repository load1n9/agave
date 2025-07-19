def "main install-stuff" [] {
  print "Installing cargo-wasi..."
  cargo install cargo-wasi
}

def "main build" [] {
  print "Building project..."
  cargo run --release build
}

def "main build-app-terminal" [] {
  print "Building terminal app (WASM)..."
  cd apps/terminal
  cargo build --release --target wasm32-wasip1
  cd ../..
}

def "main run" [] {
  print "Running project build..."
  cargo run --release build
}

def "main run-qemu" [] {
  print "Running QEMU BIOS..."
  cargo run --release --bin qemu-bios
}

def "main run-all" [] {
  print "Building terminal app and running QEMU..."
  nu agave.nu build-app-terminal
  nu agave.nu run-qemu
}

def "main qemu" [] {
  print "Launching QEMU with custom options..."
  qemu-system-x86_64 -nodefaults -m 2G -smp 2 -device virtio-mouse-pci -device virtio-keyboard-pci -nic user,model=virtio-net-pci -device virtio-vga-gl -display sdl,gl=on -serial stdio -drive format=raw,file=./target/release/uefi.img -bios ovmf
}

def main [] {
  print "
Agave Project Tasks (Nushell)
-----------------------------
install-stuff      Install cargo-wasi
build              Build the project
build-app-terminal Build the terminal app (WASM)
run                Run the project build
run-qemu           Run QEMU BIOS
run-all            Build terminal app and run QEMU
qemu               Launch QEMU with custom options

Usage: nu agave.nu <command>
Example: nu agave.nu build-app-terminal
"
}