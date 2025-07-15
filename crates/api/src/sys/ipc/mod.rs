/// Inter-Process Communication (IPC) system for Agave OS
/// Provides pipes, shared memory, message queues, and signals
use crate::sys::error::{AgaveError, AgaveResult};
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

pub mod message_queue;
pub mod pipes;
pub mod shared_memory;
pub mod signals;

/// IPC handle types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IpcHandle(u64);

impl IpcHandle {
    pub fn new() -> Self {
        static NEXT_HANDLE: AtomicU64 = AtomicU64::new(1);
        Self(NEXT_HANDLE.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// IPC resource types
#[derive(Debug)]
pub enum IpcResource {
    Pipe(pipes::Pipe),
    SharedMemory(shared_memory::SharedMemorySegment),
    MessageQueue(message_queue::MessageQueue),
    Signal(signals::SignalHandler),
}

/// Process ID type
pub type ProcessId = u32;

/// IPC permission modes
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IpcPermissions {
    pub owner_read: bool,
    pub owner_write: bool,
    pub group_read: bool,
    pub group_write: bool,
    pub other_read: bool,
    pub other_write: bool,
}

impl Default for IpcPermissions {
    fn default() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            group_read: true,
            group_write: false,
            other_read: false,
            other_write: false,
        }
    }
}

/// Global IPC manager
pub struct IpcManager {
    resources: BTreeMap<IpcHandle, IpcResource>,
    process_resources: BTreeMap<ProcessId, Vec<IpcHandle>>,
    named_resources: BTreeMap<String, IpcHandle>,
}

impl IpcManager {
    const fn new() -> Self {
        Self {
            resources: BTreeMap::new(),
            process_resources: BTreeMap::new(),
            named_resources: BTreeMap::new(),
        }
    }

    /// Create a new pipe
    pub fn create_pipe(&mut self, owner: ProcessId) -> AgaveResult<(IpcHandle, IpcHandle)> {
        let pipe = pipes::Pipe::new()?;
        let read_handle = IpcHandle::new();
        let write_handle = IpcHandle::new();

        // Create read and write ends
        let read_pipe = pipes::Pipe {
            read_end: pipe.read_end.clone(),
            write_end: None,
            permissions: pipe.permissions,
            owner: pipe.owner,
            buffer_size: pipe.buffer_size,
        };

        let write_pipe = pipes::Pipe {
            read_end: None,
            write_end: pipe.write_end.clone(),
            permissions: pipe.permissions,
            owner: pipe.owner,
            buffer_size: pipe.buffer_size,
        };

        self.resources
            .insert(read_handle, IpcResource::Pipe(read_pipe));
        self.resources
            .insert(write_handle, IpcResource::Pipe(write_pipe));

        // Track resources for the process
        self.process_resources
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(read_handle);
        self.process_resources
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(write_handle);

        log::debug!(
            "Created pipe for process {}: read={:?}, write={:?}",
            owner,
            read_handle,
            write_handle
        );
        Ok((read_handle, write_handle))
    }

    /// Create shared memory segment
    pub fn create_shared_memory(
        &mut self,
        owner: ProcessId,
        size: usize,
        name: Option<String>,
        permissions: IpcPermissions,
    ) -> AgaveResult<IpcHandle> {
        let shmem = shared_memory::SharedMemorySegment::new(size, owner, permissions)?;
        let handle = IpcHandle::new();

        self.resources
            .insert(handle, IpcResource::SharedMemory(shmem));
        self.process_resources
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(handle);

        if let Some(name) = name {
            self.named_resources.insert(name.clone(), handle);
            log::debug!(
                "Created named shared memory '{}' for process {}: {:?}",
                name,
                owner,
                handle
            );
        } else {
            log::debug!(
                "Created anonymous shared memory for process {}: {:?}",
                owner,
                handle
            );
        }

        Ok(handle)
    }

    /// Create message queue
    pub fn create_message_queue(
        &mut self,
        owner: ProcessId,
        max_messages: usize,
        max_message_size: usize,
        name: Option<String>,
    ) -> AgaveResult<IpcHandle> {
        let mq = message_queue::MessageQueue::new(max_messages, max_message_size, owner)?;
        let handle = IpcHandle::new();

        self.resources.insert(handle, IpcResource::MessageQueue(mq));
        self.process_resources
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(handle);

        if let Some(name) = name {
            self.named_resources.insert(name.clone(), handle);
            log::debug!(
                "Created named message queue '{}' for process {}: {:?}",
                name,
                owner,
                handle
            );
        } else {
            log::debug!(
                "Created anonymous message queue for process {}: {:?}",
                owner,
                handle
            );
        }

        Ok(handle)
    }

    /// Get resource by handle
    pub fn get_resource(&self, handle: IpcHandle) -> AgaveResult<&IpcResource> {
        self.resources.get(&handle).ok_or(AgaveError::NotFound)
    }

    /// Get mutable resource by handle
    pub fn get_resource_mut(&mut self, handle: IpcHandle) -> AgaveResult<&mut IpcResource> {
        self.resources.get_mut(&handle).ok_or(AgaveError::NotFound)
    }

    /// Find named resource
    pub fn find_named_resource(&self, name: &str) -> AgaveResult<IpcHandle> {
        self.named_resources
            .get(name)
            .copied()
            .ok_or(AgaveError::NotFound)
    }

    /// Close resource
    pub fn close_resource(&mut self, handle: IpcHandle, process: ProcessId) -> AgaveResult<()> {
        if let Some(handles) = self.process_resources.get_mut(&process) {
            if let Some(pos) = handles.iter().position(|&h| h == handle) {
                handles.remove(pos);
            }
        }

        self.resources.remove(&handle);
        log::debug!("Closed IPC resource {:?} for process {}", handle, process);
        Ok(())
    }

    /// Cleanup resources for a terminated process
    pub fn cleanup_process_resources(&mut self, process: ProcessId) {
        if let Some(handles) = self.process_resources.remove(&process) {
            let handle_count = handles.len();
            for handle in handles {
                self.resources.remove(&handle);
            }
            log::debug!(
                "Cleaned up {} IPC resources for terminated process {}",
                handle_count,
                process
            );
        }
    }

    /// List resources for a process
    pub fn list_process_resources(&self, process: ProcessId) -> Vec<IpcHandle> {
        self.process_resources
            .get(&process)
            .cloned()
            .unwrap_or_default()
    }

    /// Get IPC statistics
    pub fn get_statistics(&self) -> IpcStatistics {
        let mut stats = IpcStatistics::default();

        for resource in self.resources.values() {
            match resource {
                IpcResource::Pipe(_) => stats.pipe_count += 1,
                IpcResource::SharedMemory(shmem) => {
                    stats.shared_memory_count += 1;
                    stats.shared_memory_bytes += shmem.size();
                }
                IpcResource::MessageQueue(_) => stats.message_queue_count += 1,
                IpcResource::Signal(_) => stats.signal_handler_count += 1,
            }
        }

        stats.total_resources = self.resources.len();
        stats.named_resources = self.named_resources.len();
        stats.active_processes = self.process_resources.len();

        stats
    }
}

/// IPC statistics
#[derive(Debug, Clone, Default)]
pub struct IpcStatistics {
    pub total_resources: usize,
    pub pipe_count: usize,
    pub shared_memory_count: usize,
    pub shared_memory_bytes: usize,
    pub message_queue_count: usize,
    pub signal_handler_count: usize,
    pub named_resources: usize,
    pub active_processes: usize,
}

/// Global IPC manager instance
static IPC_MANAGER: Mutex<IpcManager> = Mutex::new(IpcManager::new());

/// Public API functions
pub fn init_ipc() -> AgaveResult<()> {
    log::info!("Initializing IPC system...");
    // IPC manager is already initialized as a static
    log::info!("IPC system initialized");
    Ok(())
}

pub fn create_pipe(owner: ProcessId) -> AgaveResult<(IpcHandle, IpcHandle)> {
    let mut manager = IPC_MANAGER.lock();
    manager.create_pipe(owner)
}

pub fn create_shared_memory(
    owner: ProcessId,
    size: usize,
    name: Option<String>,
    permissions: IpcPermissions,
) -> AgaveResult<IpcHandle> {
    let mut manager = IPC_MANAGER.lock();
    manager.create_shared_memory(owner, size, name, permissions)
}

pub fn create_message_queue(
    owner: ProcessId,
    max_messages: usize,
    max_message_size: usize,
    name: Option<String>,
) -> AgaveResult<IpcHandle> {
    let mut manager = IPC_MANAGER.lock();
    manager.create_message_queue(owner, max_messages, max_message_size, name)
}

pub fn find_named_resource(name: &str) -> AgaveResult<IpcHandle> {
    let manager = IPC_MANAGER.lock();
    manager.find_named_resource(name)
}

pub fn close_resource(handle: IpcHandle, process: ProcessId) -> AgaveResult<()> {
    let mut manager = IPC_MANAGER.lock();
    manager.close_resource(handle, process)
}

pub fn cleanup_process_resources(process: ProcessId) {
    let mut manager = IPC_MANAGER.lock();
    manager.cleanup_process_resources(process);
}

pub fn get_ipc_statistics() -> IpcStatistics {
    let manager = IPC_MANAGER.lock();
    manager.get_statistics()
}

/// Read from a pipe
pub fn pipe_read(handle: IpcHandle, buffer: &mut [u8]) -> AgaveResult<usize> {
    let mut manager = IPC_MANAGER.lock();
    match manager.get_resource_mut(handle)? {
        IpcResource::Pipe(pipe) => pipe.read(buffer),
        _ => Err(AgaveError::InvalidOperation),
    }
}

/// Write to a pipe
pub fn pipe_write(handle: IpcHandle, data: &[u8]) -> AgaveResult<usize> {
    let mut manager = IPC_MANAGER.lock();
    match manager.get_resource_mut(handle)? {
        IpcResource::Pipe(pipe) => pipe.write(data),
        _ => Err(AgaveError::InvalidOperation),
    }
}

/// Write to shared memory
pub fn shmem_write(handle: IpcHandle, offset: usize, data: &[u8]) -> AgaveResult<()> {
    let mut manager = IPC_MANAGER.lock();
    match manager.get_resource_mut(handle)? {
        IpcResource::SharedMemory(shmem) => shmem.write(offset, data),
        _ => Err(AgaveError::InvalidOperation),
    }
}

/// Read from shared memory
pub fn shmem_read(handle: IpcHandle, offset: usize, buffer: &mut [u8]) -> AgaveResult<usize> {
    let manager = IPC_MANAGER.lock();
    match manager.get_resource(handle)? {
        IpcResource::SharedMemory(shmem) => shmem.read(offset, buffer),
        _ => Err(AgaveError::InvalidOperation),
    }
}

/// Send message to queue
pub fn mq_send(handle: IpcHandle, data: &[u8], priority: u32) -> AgaveResult<()> {
    let mut manager = IPC_MANAGER.lock();
    match manager.get_resource_mut(handle)? {
        IpcResource::MessageQueue(mq) => mq.send(data, priority),
        _ => Err(AgaveError::InvalidOperation),
    }
}

/// Receive message from queue
pub fn mq_receive(handle: IpcHandle, buffer: &mut [u8]) -> AgaveResult<(usize, u32)> {
    let mut manager = IPC_MANAGER.lock();
    match manager.get_resource_mut(handle)? {
        IpcResource::MessageQueue(mq) => mq.receive(buffer),
        _ => Err(AgaveError::InvalidOperation),
    }
}
