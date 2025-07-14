# WASI Implementation for Agave OS

This directory contains a comprehensive implementation of the WebAssembly System Interface (WASI) for Agave OS, providing support for both WASI Preview 1 (legacy) and Preview 2 (component model) APIs.

## Overview

WASI is a system interface for WebAssembly that provides portable, capability-based APIs for WebAssembly applications to interact with the host operating system. This implementation allows WASI-compiled WebAssembly programs to run natively on Agave OS.

## Features

### WASI Preview 1 (Legacy) Support
- **Complete API Coverage**: All WASI snapshot_preview1 functions implemented
- **File System Operations**: File I/O, directory operations, path manipulation
- **Process Control**: Arguments, environment variables, exit handling
- **Time and Clocks**: System time access and clock resolution
- **Random Number Generation**: Cryptographically secure randomness
- **Socket Operations**: Network communication APIs
- **Polling**: Event-based I/O with poll_oneoff

### WASI Preview 2 (Component Model) Support
- **Modern Architecture**: Component-based design with WIT interface definitions
- **Enhanced APIs**: Improved versions of all Preview 1 functionality
- **Stream-based I/O**: Advanced input/output stream handling
- **HTTP Support**: Built-in HTTP client and server capabilities
- **WebSocket Support**: Real-time bidirectional communication
- **Enhanced Networking**: Advanced socket operations and address handling

## Module Structure

### Core Modules
- **`types.rs`**: Fundamental WASI types, constants, and data structures
- **`error.rs`**: Comprehensive error handling and result types
- **`preview1.rs`**: Legacy WASI Preview 1 function implementations
- **`preview2.rs`**: Modern WASI Preview 2 component model bindings

### API Modules
- **`io.rs`**: Core I/O streams implementation (wasi-io)
- **`clocks.rs`**: Time and clock APIs (wasi-clocks)
- **`random.rs`**: Random number generation (wasi-random)
- **`filesystem.rs`**: File system operations (wasi-filesystem)
- **`sockets.rs`**: Network socket operations (wasi-sockets)
- **`cli.rs`**: Command-line interface APIs (wasi-cli)
- **`http.rs`**: HTTP client and server APIs (wasi-http)

### Utility Modules
- **`demo.rs`**: Demonstration and testing utilities
- **`mod.rs`**: Module coordination and public exports

## Implementation Details

### Design Principles
1. **Capability-based Security**: All operations are capability-based for security
2. **Resource Management**: Proper resource cleanup and lifecycle management
3. **Error Handling**: Comprehensive error propagation and conversion
4. **Performance**: Optimized for bare-metal execution on Agave OS
5. **Compatibility**: Full compatibility with WASI specifications

### Key Features
- **No Standard Library**: Designed for `no_std` environment with `alloc`
- **Memory Safety**: Rust's memory safety guarantees maintained throughout
- **Async Support**: Ready for future async/await integration
- **Extensibility**: Modular design allows easy addition of new APIs

## Usage Examples

### Basic WASI Program
```rust
use agave_api::sys::wasi;

// Get command line arguments
let args = wasi::cli::get_arguments()?;
println!("Arguments: {:?}", args);

// Read from stdin
let stdin = wasi::io::get_stdin();
let mut buffer = vec![0; 1024];
let (data, _) = wasi::io::read(stdin, 1024)?;

// Write to stdout
let stdout = wasi::io::get_stdout();
wasi::io::write(stdout, &data)?;
```

### File Operations
```rust
// Open a file
let fd = wasi::filesystem::open_at(3, "test.txt", 0, 0)?;

// Read file contents
let (data, _) = wasi::filesystem::read(fd.0, 1024, 0)?;

// Write to file
wasi::filesystem::write(fd.0, &data, 0)?;

// Close file
wasi::filesystem::fd_close(fd.0)?;
```

### Network Operations
```rust
// Create a TCP socket
let socket = wasi::sockets::create_tcp_socket(IpAddressFamily::Ipv4)?;

// Connect to a server
let addr = IpSocketAddress::Ipv4(/* ... */);
wasi::sockets::start_connect(socket, network, addr)?;
let (input, output) = wasi::sockets::finish_connect(socket)?;

// Send data
wasi::io::write(output, b"Hello, World!")?;
```

### HTTP Client
```rust
// Create HTTP request
let request = wasi::http::new_outgoing_request(headers);
let response = wasi::http::handle(request, None)?;

// Process response
let status = wasi::http::incoming_response_status(response);
let body = wasi::http::incoming_response_consume(response)?;
```

## Integration with Agave OS

The WASI implementation is integrated into Agave OS through the WebAssembly runtime in `wasm.rs`. The integration provides:

1. **Function Linking**: All WASI functions are automatically linked to WebAssembly modules
2. **Resource Management**: Proper cleanup of WASI resources when modules terminate
3. **Security**: Capability-based access control enforced by the OS
4. **Performance**: Optimized host function calls with minimal overhead

## Testing

Run the demonstration to verify WASI functionality:

```rust
use agave_api::sys::wasi;

wasi::demo::wasi_demo();
```

This exercises all major WASI APIs and verifies proper operation.

## Standards Compliance

This implementation follows the official WASI specifications:

- **WASI Preview 1**: Based on the `wasi_snapshot_preview1` interface
- **WASI Preview 2**: Implements the component model with WIT definitions
- **WebAssembly**: Compatible with the WebAssembly specification
- **Component Model**: Supports the WebAssembly component model

## Future Enhancements

Planned improvements include:

1. **Async Support**: Full async/await support for I/O operations
2. **Additional APIs**: Support for more WASI extensions
3. **Performance**: Further optimizations for bare-metal execution
4. **Security**: Enhanced capability-based security features
5. **Debugging**: Improved debugging and profiling support

## Contributing

When contributing to the WASI implementation:

1. Maintain compatibility with both Preview 1 and Preview 2
2. Follow Rust best practices and maintain memory safety
3. Add comprehensive error handling for all operations
4. Update tests and documentation for new features
5. Ensure no_std compatibility is maintained

## References

- [WASI GitHub Repository](https://github.com/WebAssembly/WASI)
- [WASI Preview 1 Specification](https://github.com/WebAssembly/WASI/blob/main/legacy/preview1/docs.md)
- [WASI Preview 2 Specification](https://github.com/WebAssembly/WASI/tree/main/wit)
- [WebAssembly Component Model](https://github.com/WebAssembly/component-model)
