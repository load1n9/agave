/// System-wide error types for Agave OS
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
            AgaveError::OutOfMemory => write!(f, "Out of memory"),
            AgaveError::InvalidAddress => write!(f, "Invalid memory address"),
            AgaveError::NotFound => write!(f, "Resource not found"),
            AgaveError::PermissionDenied => write!(f, "Permission denied"),
            AgaveError::AlreadyExists => write!(f, "Resource already exists"),
            AgaveError::InvalidInput => write!(f, "Invalid input provided"),
            AgaveError::TimedOut => write!(f, "Operation timed out"),
            AgaveError::Busy => write!(f, "Resource busy"),
            AgaveError::NotReady => write!(f, "Resource not ready"),
            AgaveError::IoError => write!(f, "I/O error"),
            AgaveError::FileSystemError(e) => write!(f, "Filesystem error: {:?}", e),
            AgaveError::WasmError(e) => write!(f, "WASM error: {:?}", e),
            AgaveError::TaskError(e) => write!(f, "Task error: {:?}", e),
            AgaveError::HardwareError(e) => write!(f, "Hardware error: {:?}", e),
            AgaveError::VirtIO(e) => write!(f, "VirtIO error: {:?}", e),
            AgaveError::SecurityViolation => write!(f, "Security violation"),
            AgaveError::InvalidState => write!(f, "Invalid system state"),
            AgaveError::ResourceExhausted => write!(f, "Resource exhausted"),
            AgaveError::NotImplemented => write!(f, "Feature not implemented"),
            AgaveError::Unknown => write!(f, "Unknown error"),
            AgaveError::IpcError(e) => write!(f, "IPC error: {:?}", e),
            AgaveError::InvalidOperation => write!(f, "Invalid operation"),
            AgaveError::InvalidParameter => write!(f, "Invalid parameter"),
            AgaveError::WouldBlock => write!(f, "Operation would block"),
            AgaveError::BrokenPipe => write!(f, "Broken pipe"),
            AgaveError::BufferTooSmall => write!(f, "Buffer too small"),
            AgaveError::MessageTooLarge => write!(f, "Message too large"),
            AgaveError::QueueFull => write!(f, "Queue full"),
            AgaveError::QueueEmpty => write!(f, "Queue empty"),
            AgaveError::Unsupported => write!(f, "Operation not supported"),
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
