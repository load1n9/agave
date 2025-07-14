// WASI CLI implementation for Agave OS
use super::error::*;
use super::types::*;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;

// Global CLI state
static CLI_STATE: Mutex<CliState> = Mutex::new(CliState::new());

#[derive(Debug)]
pub struct CliState {
    args: Vec<String>,
    env_vars: Vec<(String, String)>,
    exit_code: Option<ExitCode>,
    stdin: super::io::InputStream,
    stdout: super::io::OutputStream,
    stderr: super::io::OutputStream,
}

impl CliState {
    pub const fn new() -> Self {
        Self {
            args: Vec::new(),
            env_vars: Vec::new(),
            exit_code: None,
            stdin: 0,  // Will be set during initialization
            stdout: 1, // Will be set during initialization
            stderr: 2, // Will be set during initialization
        }
    }
}

// Initialize CLI environment
pub fn init_cli() {
    let mut cli = CLI_STATE.lock();

    // Set up default arguments
    cli.args = alloc::vec!["agave-wasm".to_string()];

    // Set up default environment variables
    cli.env_vars = alloc::vec![
        ("PATH".to_string(), "/bin:/usr/bin".to_string()),
        ("HOME".to_string(), "/".to_string()),
        ("USER".to_string(), "agave".to_string()),
        ("SHELL".to_string(), "/bin/sh".to_string()),
        ("TERM".to_string(), "agave".to_string()),
        ("LANG".to_string(), "C.UTF-8".to_string()),
    ];

    // Create standard streams
    cli.stdin = super::io::create_input_stream(Vec::new());
    cli.stdout = super::io::create_output_stream();
    cli.stderr = super::io::create_output_stream();
}

// Preview 1 API implementations

pub fn args_get(_argv_ptr: u32, _argv_buf_ptr: u32) -> WasiResult<()> {
    let cli = CLI_STATE.lock();

    // In a real implementation, we would write argument pointers and strings
    // to WebAssembly memory at argv_ptr and argv_buf_ptr
    // For now, we'll just validate the operation

    if cli.args.is_empty() {
        return Err(WasiError::inval());
    }

    log::debug!("args_get: {} arguments available", cli.args.len());
    Ok(())
}

pub fn args_sizes_get() -> WasiResult<(Size, Size)> {
    let cli = CLI_STATE.lock();

    let argc = cli.args.len() as Size;
    let argv_buf_size = cli
        .args
        .iter()
        .map(|arg| arg.len() + 1) // +1 for null terminator
        .sum::<usize>() as Size;

    Ok((argc, argv_buf_size))
}

pub fn environ_get(_environ_ptr: u32, _environ_buf_ptr: u32) -> WasiResult<()> {
    let cli = CLI_STATE.lock();

    // In a real implementation, we would write environment variable pointers
    // and strings to WebAssembly memory
    // For now, we'll just validate the operation

    log::debug!(
        "environ_get: {} environment variables available",
        cli.env_vars.len()
    );
    Ok(())
}

pub fn environ_sizes_get() -> WasiResult<(Size, Size)> {
    let cli = CLI_STATE.lock();

    let environc = cli.env_vars.len() as Size;
    let environ_buf_size = cli
        .env_vars
        .iter()
        .map(|(key, value)| key.len() + 1 + value.len() + 1) // key=value\0
        .sum::<usize>() as Size;

    Ok((environc, environ_buf_size))
}

pub fn proc_exit(exit_code: ExitCode) -> ! {
    {
        let mut cli = CLI_STATE.lock();
        cli.exit_code = Some(exit_code);
    }

    log::info!("Process exiting with code: {}", exit_code);

    // In a real implementation, this would terminate the process
    // For now, we'll just panic to stop execution
    panic!("Process exited with code: {}", exit_code);
}

pub fn proc_raise(signal: Signal) -> WasiResult<()> {
    log::info!("Signal raised: {}", signal);

    match signal {
        2 => proc_exit(128 + 2),   // SIGINT
        9 => proc_exit(128 + 9),   // SIGKILL
        15 => proc_exit(128 + 15), // SIGTERM
        _ => {
            log::warn!("Unhandled signal: {}", signal);
            Ok(())
        }
    }
}

pub fn sched_yield() -> WasiResult<()> {
    // In a real implementation, this would yield to the scheduler
    // For now, we'll just log the operation
    log::debug!("sched_yield called");
    Ok(())
}

// Preview 2 API implementations

pub fn get_arguments() -> WasiResult<Vec<String>> {
    let cli = CLI_STATE.lock();
    Ok(cli.args.clone())
}

pub fn get_environment() -> WasiResult<Vec<(String, String)>> {
    let cli = CLI_STATE.lock();
    Ok(cli.env_vars.clone())
}

pub fn get_environment_variable(key: &str) -> WasiResult<Option<String>> {
    let cli = CLI_STATE.lock();

    for (env_key, env_value) in &cli.env_vars {
        if env_key == key {
            return Ok(Some(env_value.clone()));
        }
    }

    Ok(None)
}

pub fn get_stdin() -> super::io::InputStream {
    let cli = CLI_STATE.lock();
    cli.stdin
}

pub fn get_stdout() -> super::io::OutputStream {
    let cli = CLI_STATE.lock();
    cli.stdout
}

pub fn get_stderr() -> super::io::OutputStream {
    let cli = CLI_STATE.lock();
    cli.stderr
}

pub fn exit_with_code(exit_code: ExitCode) -> ! {
    proc_exit(exit_code)
}

// Terminal control functions
pub fn get_terminal_size() -> WasiResult<(u32, u32)> {
    // Return default terminal size
    Ok((80, 24)) // 80 columns, 24 rows
}

pub fn is_terminal(stream: super::io::OutputStream) -> bool {
    let cli = CLI_STATE.lock();
    // Only stdout and stderr are considered terminals
    stream == cli.stdout || stream == cli.stderr
}

// Additional CLI functions for Preview 2 compatibility
pub fn initial_cwd() -> WasiResult<String> {
    // Get initial current working directory
    log::debug!("cli::initial_cwd()");
    Ok("/".to_string())
}

pub fn get_terminal_stdin() -> Option<u32> {
    // Get terminal stdin
    log::debug!("cli::get_terminal_stdin()");
    Some(0)
}

pub fn get_terminal_stdout() -> Option<u32> {
    // Get terminal stdout
    log::debug!("cli::get_terminal_stdout()");
    Some(1)
}

pub fn get_terminal_stderr() -> Option<u32> {
    // Get terminal stderr
    log::debug!("cli::get_terminal_stderr()");
    Some(2)
}

// Terminal types for Preview 2 compatibility
pub type TerminalInput = u32;
pub type TerminalOutput = u32;

// Extended CLI functionality
pub fn set_arguments(args: Vec<String>) {
    let mut cli = CLI_STATE.lock();
    cli.args = args;
}

pub fn add_argument(arg: String) {
    let mut cli = CLI_STATE.lock();
    cli.args.push(arg);
}

pub fn set_environment_variable(key: String, value: String) {
    let mut cli = CLI_STATE.lock();

    // Update existing variable or add new one
    for (env_key, env_value) in &mut cli.env_vars {
        if *env_key == key {
            *env_value = value;
            return;
        }
    }

    // Add new environment variable
    cli.env_vars.push((key, value));
}

pub fn unset_environment_variable(key: &str) {
    let mut cli = CLI_STATE.lock();
    cli.env_vars.retain(|(env_key, _)| env_key != key);
}

pub fn clear_environment() {
    let mut cli = CLI_STATE.lock();
    cli.env_vars.clear();
}

pub fn get_current_directory() -> WasiResult<String> {
    // In a real implementation, this would get the actual working directory
    Ok("/".to_string())
}

pub fn set_current_directory(path: &str) -> WasiResult<()> {
    // In a real implementation, this would change the working directory
    log::debug!("Set current directory to: {}", path);
    Ok(())
}

// Process information
pub fn get_process_id() -> u32 {
    // Return a simulated process ID
    1
}

pub fn get_parent_process_id() -> u32 {
    // Return a simulated parent process ID
    0
}

pub fn get_process_group_id() -> u32 {
    // Return a simulated process group ID
    1
}

pub fn get_session_id() -> u32 {
    // Return a simulated session ID
    1
}

// User information
pub fn get_user_id() -> u32 {
    // Return a simulated user ID
    1000
}

pub fn get_group_id() -> u32 {
    // Return a simulated group ID
    1000
}

pub fn get_effective_user_id() -> u32 {
    get_user_id()
}

pub fn get_effective_group_id() -> u32 {
    get_group_id()
}

// Signal handling
pub fn set_signal_handler(signal: Signal, handler: Option<fn(Signal)>) -> WasiResult<()> {
    // In a real implementation, this would set up signal handling
    log::debug!(
        "Set signal handler for signal {}: {:?}",
        signal,
        handler.is_some()
    );
    Ok(())
}

pub fn send_signal(process_id: u32, signal: Signal) -> WasiResult<()> {
    // In a real implementation, this would send a signal to a process
    log::debug!("Send signal {} to process {}", signal, process_id);
    Ok(())
}

// Resource limits
pub fn get_resource_limit(resource: u32) -> WasiResult<(u64, u64)> {
    // Return simulated resource limits (soft, hard)
    match resource {
        0 => Ok((1024 * 1024, 2 * 1024 * 1024)), // CPU time in seconds
        1 => Ok((100 * 1024 * 1024, 200 * 1024 * 1024)), // File size in bytes
        2 => Ok((8 * 1024 * 1024, 16 * 1024 * 1024)), // Data size in bytes
        3 => Ok((8 * 1024 * 1024, 16 * 1024 * 1024)), // Stack size in bytes
        4 => Ok((1024 * 1024 * 1024, 2 * 1024 * 1024 * 1024)), // Core file size in bytes
        7 => Ok((1024, 2048)),                   // Number of file descriptors
        _ => Err(WasiError::inval()),
    }
}

pub fn set_resource_limit(resource: u32, soft_limit: u64, hard_limit: u64) -> WasiResult<()> {
    // In a real implementation, this would set resource limits
    log::debug!(
        "Set resource limit {}: soft={}, hard={}",
        resource,
        soft_limit,
        hard_limit
    );
    Ok(())
}

// Process execution
pub fn spawn_process(program: &str, args: &[String], env: &[(String, String)]) -> WasiResult<u32> {
    // In a real implementation, this would spawn a new process
    log::debug!(
        "Spawn process: {} with {} args and {} env vars",
        program,
        args.len(),
        env.len()
    );

    // Return a simulated process ID
    Ok(get_process_id() + 1)
}

pub fn wait_for_process(process_id: u32) -> WasiResult<ExitCode> {
    // In a real implementation, this would wait for the process to exit
    log::debug!("Wait for process: {}", process_id);

    // Return a simulated exit code
    Ok(0)
}

// Input/Output redirection
pub fn redirect_stdin(stream: super::io::InputStream) {
    let mut cli = CLI_STATE.lock();
    cli.stdin = stream;
}

pub fn redirect_stdout(stream: super::io::OutputStream) {
    let mut cli = CLI_STATE.lock();
    cli.stdout = stream;
}

pub fn redirect_stderr(stream: super::io::OutputStream) {
    let mut cli = CLI_STATE.lock();
    cli.stderr = stream;
}
