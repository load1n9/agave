# <img src="assets/Agave.png" width="70rem" /> Agave OS

Agave OS is a simple operating system written in Rust with WASI support. It is designed to be lightweight, fast, and easy to use. It is also designed to be easy to customize and extend. Started from [Fomos](https://github.com/Ruddle/Fomos) and the second edition of
[Writing an OS in Rust](https://os.phil-opp.com/) by Philipp Oppermann. Also contains code from [Theseus OS](https://github.com/theseus-os/Theseus)

## 🚀 Enhanced Features

### Core Features

- **Lightweight and fast** - Minimal overhead with efficient memory management
- **Hardware support** - Supports a wide range of x86_64 hardware platforms
- **Easy to customize** - Modular architecture for easy extension
- **Built with Rust** - Memory safety and performance with zero-cost abstractions

### 🎨 Enhanced Graphics & WASM

- **Rich Graphics API** - Circles, rectangles, lines, triangles with color support
- **Animation Support** - Time-based animations and smooth transitions
- **Interactive Applications** - Mouse input and real-time interaction
- **WASM Runtime** - Execute WebAssembly applications with comprehensive API

### ⚡ Advanced System Features

- **Priority Task Scheduling** - Multi-level priority queues with fair scheduling
- **Memory Management** - Advanced allocator with statistics and leak detection
- **Error Handling** - Comprehensive error types with recovery strategies
- **System Monitoring** - Real-time performance metrics and health monitoring
- **Profiling Tools** - Built-in profiler for performance analysis

### 🛠️ Development Tools

- **Enhanced Diagnostics** - Detailed panic information with system state
- **Memory Tracking** - Allocation statistics and pressure monitoring
- **Performance Events** - Event logging for optimization insights
- **Build Scripts** - Automated build process with testing support

## WASM Applications

Enhanced WASM applications now support rich graphics and animations:

```rust
use agave_lib::{
    clear_screen, draw_circle, fill_circle, draw_triangle, 
    get_dimensions, get_time_ms, Position, RGBA
};

#[no_mangle]
pub extern "C" fn update(mouse_x: i32, mouse_y: i32) {
    let dim = get_dimensions();
    let time = get_time_ms();
    
    // Clear with animated background
    clear_screen(RGBA::new(10, 10, 20, 255));
    
    // Draw animated elements
    let phase = (time as f32 / 1000.0) * 2.0 * 3.14159;
    let center = Position::new(dim.width / 2, dim.height / 2);
    let radius = 50 + (phase.sin() * 20.0) as i32;
    
    fill_circle(center, radius, RGBA::new(255, 100, 150, 200));
    
    // Interactive mouse following
    fill_circle(
        Position::new(mouse_x, mouse_y),
        15,
        RGBA::new(100, 255, 100, 255)
    );
}
```

### Available Graphics Functions

- `clear_screen(color)` - Clear entire screen
- `set_pixel(pos, color)` - Set individual pixel
- `draw_circle(center, radius, color)` - Draw circle outline
- `fill_circle(center, radius, color)` - Draw filled circle
- `draw_rectangle(pos, width, height, color)` - Draw rectangle outline
- `fill_rectangle(pos, width, height, color)` - Draw filled rectangle
- `draw_line(start, end, color)` - Draw line between points
- `draw_triangle(p1, p2, p3, color)` - Draw triangle outline
- `get_dimensions()` - Get screen dimensions
- `get_time_ms()` - Get system time in milliseconds

### Color Constants

- `RGBA::RED`, `RGBA::GREEN`, `RGBA::BLUE`
- `RGBA::WHITE`, `RGBA::BLACK`, `RGBA::TRANSPARENT`

![Enhanced WASM app with animations](assets/demo.png)

## 🔧 Building and Running

### Prerequisites

- Rust nightly toolchain
- QEMU (for running the OS)

### Quick Start

```powershell
# Build and run in debug mode
deno task build

# Build release version
.\build.ps1 -Release

# Build with tests
.\build.ps1 -Test

# Clean build
.\build.ps1 -Clean

# Verbose logging
.\build.ps1 -Verbose -Run
```

### Manual Building

```bash
# Build test application
cd apps/test-app
cargo build --target wasm32-wasi --release

# Build OS
cd ../..
cargo run --release build

# Run with QEMU
qemu-system-x86_64 -drive format=raw,file=target/release/bios.img -m 512M
```

## 📊 System Monitoring

The OS includes comprehensive monitoring capabilities:

- **Memory Statistics** - Allocation tracking, peak usage, fragmentation analysis
- **Task Metrics** - Spawn/completion rates, execution time, context switches
- **Performance Events** - Memory pressure, queue status, performance anomalies
- **Health Monitoring** - Automatic detection of system issues
- **Profiling** - Function-level performance measurement

## 🏗️ Architecture

### Core Components

- **Kernel** (`crates/kernel`) - Core OS functionality
- **API** (`crates/api`) - System APIs and drivers
- **Library** (`crates/lib`) - WASM application library
- **Applications** (`apps/`) - WASM applications

### Enhanced Systems

- **Error Handling** (`sys/error.rs`) - Comprehensive error types
- **Memory Management** (`sys/allocator.rs`) - Advanced allocation tracking
- **Task Scheduling** (`sys/task/`) - Priority-based async execution
- **System Monitoring** (`sys/monitor.rs`) - Performance and health tracking
- **WASM Runtime** (`sys/wasm.rs`) - Enhanced WebAssembly execution

## 🐛 Debugging

Enhanced panic handler provides detailed system information:

- System uptime and current state
- Memory usage and allocation statistics
- Task execution metrics
- Recent performance events

## 🤝 Contributing

Contributions are welcome! Areas for improvement:

- Additional graphics primitives
- Network stack implementation
- Filesystem enhancements
- Hardware driver support
- Performance optimizations

## Maintainers

- Dean Srebnik ([@load1n9](https://github.com/load1n9))
