use core::fmt;

/// Main error type for Agave OS operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgaveError {
    /// Memory allocation failed
    OutOfMemory,
    /// Invalid memory address or alignment
    InvalidAddress,
    /// Resource not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// Resource already exists
    AlreadyExists,
    /// Invalid arguments provided
    InvalidInput,
    /// Operation timed out
    TimedOut,
    /// Device or resource busy
    Busy,
    /// Resource not ready
    NotReady,
    /// I/O error occurred
    IoError,
    /// Filesystem error
    FileSystemError(FsError),
    /// WASM runtime error
    WasmError(WasmError),
    /// Task execution error
    TaskError(TaskError),
    /// Hardware error
    HardwareError(HwError),
    /// VirtIO subsystem error
    VirtIO(VirtioError),
    /// Security violation
    SecurityViolation,
    /// Invalid system state
    InvalidState,
    /// Resource exhausted
    ResourceExhausted,
    /// Feature not implemented
    NotImplemented,
    /// Unknown error
    Unknown,
    /// IPC-related error
    IpcError(IpcError),
    /// Invalid operation
    InvalidOperation,
    /// Invalid parameter
    InvalidParameter,
    /// Operation would block
    WouldBlock,
    /// Broken pipe
    BrokenPipe,
    /// Buffer too small
    BufferTooSmall,
    /// Message too large
    MessageTooLarge,
    /// Queue full
    QueueFull,
    /// Queue empty
    QueueEmpty,
    /// Operation not supported
    Unsupported,
}

/// Filesystem-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FsError {
    FileNotFound,
    DirectoryNotFound,
    FileAlreadyExists,
    DirectoryNotEmpty,
    InvalidPath,
    ReadOnlyFilesystem,
    DiskFull,
    CorruptedData,
    IsDirectory,
    NotDirectory,
    InvalidFileDescriptor,
}

/// WASM runtime errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WasmError {
    CompilationFailed,
    InstantiationFailed,
    ExecutionFailed,
    InvalidModule,
    FunctionNotFound,
    MemoryAccessViolation,
}

/// Task execution errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskError {
    SpawnFailed,
    QueueFull,
    DeadlockDetected,
    ResourceContention,
    InvalidPriority,
}

/// VirtIO subsystem errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VirtioError {
    DeviceNotFound,
    DeviceNotResponding,
    ConfigurationError,
    QueueError,
    DescriptorError,
    FeatureNegotiationFailed,
    InvalidConfiguration,
    BufferError,
    InterruptError,
}

/// Hardware-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HwError {
    DeviceNotFound,
    DeviceNotResponding,
    ConfigurationError,
    BusError,
    InterruptError,
}

/// IPC-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcError {
    PipeError,
    SharedMemoryError,
    MessageQueueError,
    SignalError,
    HandleNotFound,
    PermissionDenied,
    ResourceLimitExceeded,
}

impl fmt::Display for AgaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgaveError::OutOfMemory => write!(f, "Memory allocation failed: the system ran out of memory."),
            AgaveError::InvalidAddress => write!(f, "Invalid memory address or alignment encountered."),
            AgaveError::NotFound => write!(f, "Requested resource was not found."),
            AgaveError::PermissionDenied => write!(f, "Permission denied: insufficient privileges for the operation."),
            AgaveError::AlreadyExists => write!(f, "Resource already exists and cannot be recreated."),
            AgaveError::InvalidInput => write!(f, "Invalid arguments provided to the operation."),
            AgaveError::TimedOut => write!(f, "Operation timed out before completion."),
            AgaveError::Busy => write!(f, "Device or resource is currently busy."),
            AgaveError::NotReady => write!(f, "Resource is not ready for the requested operation."),
            AgaveError::IoError => write!(f, "An I/O error occurred during the operation."),
            AgaveError::FileSystemError(e) => write!(f, "Filesystem error: {}", e),
            AgaveError::WasmError(e) => write!(f, "WASM runtime error: {}", e),
            AgaveError::TaskError(e) => write!(f, "Task execution error: {}", e),
            AgaveError::HardwareError(e) => write!(f, "Hardware error: {}", e),
            AgaveError::VirtIO(e) => write!(f, "VirtIO subsystem error: {}", e),
            AgaveError::SecurityViolation => write!(f, "Security violation detected."),
            AgaveError::InvalidState => write!(f, "Invalid system state for this operation."),
            AgaveError::ResourceExhausted => write!(f, "Resource exhausted: no more resources available."),
            AgaveError::NotImplemented => write!(f, "Feature not implemented in this build."),
            AgaveError::Unknown => write!(f, "An unknown error has occurred."),
            AgaveError::IpcError(e) => write!(f, "IPC error: {}", e),
            AgaveError::InvalidOperation => write!(f, "Invalid operation for the current context."),
            AgaveError::InvalidParameter => write!(f, "Invalid parameter supplied to the function."),
            AgaveError::WouldBlock => write!(f, "Operation would block and cannot proceed now."),
            AgaveError::BrokenPipe => write!(f, "Broken pipe: communication channel is no longer available."),
            AgaveError::BufferTooSmall => write!(f, "Buffer too small for the requested operation."),
            AgaveError::MessageTooLarge => write!(f, "Message too large to be processed."),
            AgaveError::QueueFull => write!(f, "Queue is full and cannot accept more items."),
            AgaveError::QueueEmpty => write!(f, "Queue is empty and no items are available."),
            AgaveError::Unsupported => write!(f, "Operation is not supported on this platform or configuration."),
        }
    }
}

impl fmt::Display for FsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use FsError::*;
        match self {
            FileNotFound => write!(f, "File not found in the filesystem."),
            DirectoryNotFound => write!(f, "Directory not found in the filesystem."),
            FileAlreadyExists => write!(f, "File already exists and cannot be created."),
            DirectoryNotEmpty => write!(f, "Directory is not empty and cannot be removed."),
            InvalidPath => write!(f, "Invalid path specified for filesystem operation."),
            ReadOnlyFilesystem => write!(f, "Filesystem is read-only and cannot be modified."),
            DiskFull => write!(f, "Disk is full and cannot store more data."),
            CorruptedData => write!(f, "Filesystem data is corrupted."),
            IsDirectory => write!(f, "Expected a file but found a directory."),
            NotDirectory => write!(f, "Expected a directory but found a file."),
            InvalidFileDescriptor => write!(f, "Invalid file descriptor used in operation."),
        }
    }
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use WasmError::*;
        match self {
            CompilationFailed => write!(f, "WASM module compilation failed."),
            InstantiationFailed => write!(f, "WASM module instantiation failed."),
            ExecutionFailed => write!(f, "WASM module execution failed."),
            InvalidModule => write!(f, "Invalid WASM module provided."),
            FunctionNotFound => write!(f, "Requested function not found in WASM module."),
            MemoryAccessViolation => write!(f, "WASM memory access violation occurred."),
        }
    }
}

impl fmt::Display for TaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TaskError::*;
        match self {
            SpawnFailed => write!(f, "Failed to spawn a new task."),
            QueueFull => write!(f, "Task queue is full and cannot accept new tasks."),
            DeadlockDetected => write!(f, "Deadlock detected in task execution."),
            ResourceContention => write!(f, "Resource contention detected among tasks."),
            InvalidPriority => write!(f, "Invalid priority specified for task."),
        }
    }
}

impl fmt::Display for VirtioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use VirtioError::*;
        match self {
            DeviceNotFound => write!(f, "VirtIO device not found."),
            DeviceNotResponding => write!(f, "VirtIO device is not responding."),
            ConfigurationError => write!(f, "VirtIO device configuration error."),
            QueueError => write!(f, "VirtIO queue error occurred."),
            DescriptorError => write!(f, "VirtIO descriptor error occurred."),
            FeatureNegotiationFailed => write!(f, "VirtIO feature negotiation failed."),
            InvalidConfiguration => write!(f, "VirtIO device has invalid configuration."),
            BufferError => write!(f, "VirtIO buffer error occurred."),
            InterruptError => write!(f, "VirtIO interrupt error occurred."),
        }
    }
}

impl fmt::Display for HwError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use HwError::*;
        match self {
            DeviceNotFound => write!(f, "Hardware device not found."),
            DeviceNotResponding => write!(f, "Hardware device is not responding."),
            ConfigurationError => write!(f, "Hardware configuration error occurred."),
            BusError => write!(f, "Hardware bus error occurred."),
            InterruptError => write!(f, "Hardware interrupt error occurred."),
        }
    }
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use IpcError::*;
        match self {
            PipeError => write!(f, "IPC pipe error occurred."),
            SharedMemoryError => write!(f, "IPC shared memory error occurred."),
            MessageQueueError => write!(f, "IPC message queue error occurred."),
            SignalError => write!(f, "IPC signal error occurred."),
            HandleNotFound => write!(f, "IPC handle not found."),
            PermissionDenied => write!(f, "IPC permission denied."),
            ResourceLimitExceeded => write!(f, "IPC resource limit exceeded."),
        }
    }
}

/// Result type alias for Agave OS operations
pub type AgaveResult<T> = Result<T, AgaveError>;

/// From implementations for error conversion
impl From<VirtioError> for AgaveError {
    fn from(err: VirtioError) -> Self {
        AgaveError::VirtIO(err)
    }
}

impl From<&str> for AgaveError {
    fn from(_err: &str) -> Self {
        AgaveError::VirtIO(VirtioError::ConfigurationError)
    }
}

impl<S> From<x86_64::structures::paging::mapper::MapToError<S>> for AgaveError
where
    S: x86_64::structures::paging::page::PageSize,
{
    fn from(_err: x86_64::structures::paging::mapper::MapToError<S>) -> Self {
        AgaveError::HardwareError(HwError::ConfigurationError)
    }
}

/// Error recovery strategies
#[derive(Debug, Clone)]
pub enum RecoveryStrategy {
    Retry { max_attempts: u32, delay_ms: u64 },
    Fallback,
    Abort,
    Ignore,
}

/// Error context for better debugging
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error: AgaveError,
    pub location: &'static str,
    pub message: Option<&'static str>,
    pub recovery: RecoveryStrategy,
}

impl ErrorContext {
    pub fn new(error: AgaveError, location: &'static str) -> Self {
        Self {
            error,
            location,
            message: None,
            recovery: RecoveryStrategy::Abort,
        }
    }

    pub fn with_message(mut self, message: &'static str) -> Self {
        self.message = Some(message);
        self
    }

    pub fn with_recovery(mut self, recovery: RecoveryStrategy) -> Self {
        self.recovery = recovery;
        self
    }
}

/// Macro for creating error contexts
#[macro_export]
macro_rules! agave_error {
    ($error:expr) => {
        ErrorContext::new($error, concat!(file!(), ":", line!()))
    };
    ($error:expr, $msg:expr) => {
        ErrorContext::new($error, concat!(file!(), ":", line!())).with_message($msg)
    };
    ($error:expr, $msg:expr, $recovery:expr) => {
        ErrorContext::new($error, concat!(file!(), ":", line!()))
            .with_message($msg)
            .with_recovery($recovery)
    };
}

/// Convert panic messages to errors instead of crashing
pub fn handle_panic_as_error<T, F>(f: F) -> AgaveResult<T>
where
    F: FnOnce() -> T,
{
    // In a real implementation, this would use panic hooks
    // For now, just execute the function
    Ok(f())
}

/// Logging wrapper for errors
pub fn log_error(context: &ErrorContext) {
    match context.error {
        AgaveError::OutOfMemory | AgaveError::HardwareError(_) => {
            log::error!(
                "Critical error at {}: {} - {:?}",
                context.location,
                context.error,
                context.message
            );
        }
        _ => {
            log::warn!(
                "Error at {}: {} - {:?}",
                context.location,
                context.error,
                context.message
            );
        }
    }
}
