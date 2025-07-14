/// Enhanced process management for Agave OS
/// Provides process isolation, scheduling, and inter-process communication
use crate::sys::{
    error::{AgaveError, AgaveResult},
    memory,
    task::{Task, TaskId},
    diagnostics::{add_diagnostic, DiagnosticLevel, DiagnosticCategory},
};
use alloc::{
    vec::Vec,
    string::{String, ToString},
    collections::{BTreeMap, VecDeque},
    boxed::Box,
    format,
};
use core::{
    sync::atomic::{AtomicU64, AtomicU32, Ordering},
    future::Future,
    task::{Context, Poll},
    pin::Pin,
};
use spin::Mutex;

/// Process ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessId(u64);

impl ProcessId {
    fn new() -> Self {
        static NEXT_PID: AtomicU64 = AtomicU64::new(1);
        ProcessId(NEXT_PID.fetch_add(1, Ordering::Relaxed))
    }
    
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Process state
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProcessState {
    Created,
    Running,
    Sleeping,
    Waiting,
    Zombie,
    Terminated,
}

/// Process priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Critical = 0,  // System critical processes
    High = 1,      // High priority system processes
    Normal = 2,    // Normal user processes
    Low = 3,       // Background processes
    Idle = 4,      // Idle/cleanup processes
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Process context and metadata
#[derive(Debug, Clone)]
pub struct ProcessContext {
    pub pid: ProcessId,
    pub parent_pid: Option<ProcessId>,
    pub name: String,
    pub state: ProcessState,
    pub priority: Priority,
    pub created_time: u64,
    pub cpu_time_ms: u64,
    pub memory_usage: usize,
    pub exit_code: Option<i32>,
    pub children: Vec<ProcessId>,
}

/// Inter-process communication message
#[derive(Debug, Clone)]
pub struct IpcMessage {
    pub from: ProcessId,
    pub to: ProcessId,
    pub message_type: IpcMessageType,
    pub data: Vec<u8>,
    pub timestamp: u64,
}

/// IPC message types
#[derive(Debug, Clone, PartialEq)]
pub enum IpcMessageType {
    Signal,
    Data,
    Resource,
    Synchronization,
    Event,
}

/// Process-safe communication channel
pub struct IpcChannel {
    messages: VecDeque<IpcMessage>,
    max_messages: usize,
}

impl IpcChannel {
    fn new(max_messages: usize) -> Self {
        Self {
            messages: VecDeque::new(),
            max_messages,
        }
    }
    
    fn send(&mut self, message: IpcMessage) -> AgaveResult<()> {
        if self.messages.len() >= self.max_messages {
            return Err(AgaveError::ResourceExhausted);
        }
        self.messages.push_back(message);
        Ok(())
    }
    
    fn receive(&mut self) -> Option<IpcMessage> {
        self.messages.pop_front()
    }
    
    fn peek(&self) -> Option<&IpcMessage> {
        self.messages.front()
    }
    
    fn len(&self) -> usize {
        self.messages.len()
    }
}

/// Process control block
pub struct ProcessControlBlock {
    pub context: ProcessContext,
    pub task: Option<Pin<Box<dyn Future<Output = i32> + Send>>>,
    pub ipc_inbox: IpcChannel,
    pub waiting_for: Option<ProcessId>,
    pub resource_limits: ResourceLimits,
    pub statistics: ProcessStatistics,
}

/// Resource limits for processes
#[derive(Debug, Clone)]
pub struct ResourceLimits {
    pub max_memory: usize,
    pub max_cpu_time_ms: u64,
    pub max_children: usize,
    pub max_open_files: usize,
    pub max_ipc_messages: usize,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory: 16 * 1024 * 1024,  // 16MB
            max_cpu_time_ms: 60 * 1000,    // 1 minute
            max_children: 10,
            max_open_files: 100,
            max_ipc_messages: 1000,
        }
    }
}

/// Process execution statistics
#[derive(Debug, Clone, Default)]
pub struct ProcessStatistics {
    pub context_switches: u64,
    pub page_faults: u64,
    pub system_calls: u64,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_read: u64,
    pub bytes_written: u64,
}

/// Process scheduler with multiple priority queues
pub struct ProcessScheduler {
    processes: BTreeMap<ProcessId, ProcessControlBlock>,
    ready_queues: [VecDeque<ProcessId>; 5], // One queue per priority level
    current_process: Option<ProcessId>,
    next_schedule_time: u64,
    schedule_quantum_ms: u64,
    total_processes: AtomicU32,
    active_processes: AtomicU32,
}

impl ProcessScheduler {
    fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            ready_queues: [
                VecDeque::new(), VecDeque::new(), VecDeque::new(), 
                VecDeque::new(), VecDeque::new()
            ],
            current_process: None,
            next_schedule_time: 0,
            schedule_quantum_ms: 10, // 10ms time slice
            total_processes: AtomicU32::new(0),
            active_processes: AtomicU32::new(0),
        }
    }
    
    /// Spawn a new process
    pub fn spawn_process<F>(&mut self, name: String, priority: Priority, future: F, parent_pid: Option<ProcessId>) -> AgaveResult<ProcessId>
    where
        F: Future<Output = i32> + Send + 'static,
    {
        let pid = ProcessId::new();
        let now = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
        
        // Check parent process limits
        if let Some(parent_id) = parent_pid {
            if let Some(parent) = self.processes.get_mut(&parent_id) {
                if parent.context.children.len() >= parent.resource_limits.max_children {
                    return Err(AgaveError::ResourceExhausted);
                }
                parent.context.children.push(pid);
            }
        }
        
        let context = ProcessContext {
            pid,
            parent_pid,
            name: name.clone(),
            state: ProcessState::Created,
            priority,
            created_time: now,
            cpu_time_ms: 0,
            memory_usage: 0,
            exit_code: None,
            children: Vec::new(),
        };
        
        let pcb = ProcessControlBlock {
            context,
            task: Some(Box::pin(future)),
            ipc_inbox: IpcChannel::new(1000),
            waiting_for: None,
            resource_limits: ResourceLimits::default(),
            statistics: ProcessStatistics::default(),
        };
        
        self.processes.insert(pid, pcb);
        self.ready_queues[priority as usize].push_back(pid);
        
        self.total_processes.fetch_add(1, Ordering::Relaxed);
        self.active_processes.fetch_add(1, Ordering::Relaxed);
        
        add_diagnostic(
            DiagnosticLevel::Info,
            DiagnosticCategory::Tasks,
            format!("Process spawned: {} (PID: {})", name, pid.as_u64()),
            Some(format!("Priority: {:?}, Parent: {:?}", priority, parent_pid))
        );
        
        log::info!("Spawned process '{}' with PID {}", name, pid.as_u64());
        Ok(pid)
    }
    
    /// Schedule and run processes
    pub fn schedule(&mut self) -> Option<ProcessId> {
        let now = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
        
        // Check if we need to preempt current process
        if let Some(current_pid) = self.current_process {
            if now >= self.next_schedule_time {
                // Time slice expired, move to back of queue
                if let Some(process) = self.processes.get(&current_pid) {
                    if process.context.state == ProcessState::Running {
                        let priority = process.context.priority as usize;
                        self.ready_queues[priority].push_back(current_pid);
                    }
                }
                self.current_process = None;
            }
        }
        
        // If no current process, select next one
        if self.current_process.is_none() {
            // Find highest priority non-empty queue
            for (priority, queue) in self.ready_queues.iter_mut().enumerate() {
                if let Some(pid) = queue.pop_front() {
                    if let Some(process) = self.processes.get_mut(&pid) {
                        if process.context.state != ProcessState::Terminated {
                            process.context.state = ProcessState::Running;
                            process.statistics.context_switches += 1;
                            self.current_process = Some(pid);
                            self.next_schedule_time = now + self.schedule_quantum_ms;
                            
                            log::trace!("Scheduled process {} (priority {})", pid.as_u64(), priority);
                            return Some(pid);
                        }
                    }
                }
            }
        }
        
        self.current_process
    }
    
    /// Run the current scheduled process
    pub fn run_current_process(&mut self, cx: &mut Context<'_>) -> Poll<()> {
        if let Some(current_pid) = self.current_process {
            if let Some(pcb) = self.processes.get_mut(&current_pid) {
                if let Some(task) = &mut pcb.task {
                    let start_time = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
                    
                    match task.as_mut().poll(cx) {
                        Poll::Ready(exit_code) => {
                            let end_time = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
                            pcb.context.cpu_time_ms += end_time - start_time;
                            pcb.context.state = ProcessState::Terminated;
                            pcb.context.exit_code = Some(exit_code);
                            pcb.task = None;
                            
                            self.active_processes.fetch_sub(1, Ordering::Relaxed);
                            self.current_process = None;
                            
                            add_diagnostic(
                                DiagnosticLevel::Info,
                                DiagnosticCategory::Tasks,
                                format!("Process terminated: {} (exit code: {})", current_pid.as_u64(), exit_code),
                                Some(format!("CPU time: {}ms", pcb.context.cpu_time_ms))
                            );
                            
                            log::info!("Process {} terminated with exit code {}", current_pid.as_u64(), exit_code);
                            Poll::Ready(())
                        }
                        Poll::Pending => {
                            let end_time = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
                            pcb.context.cpu_time_ms += end_time - start_time;
                            Poll::Pending
                        }
                    }
                } else {
                    Poll::Ready(())
                }
            } else {
                Poll::Ready(())
            }
        } else {
            Poll::Ready(())
        }
    }
    
    /// Send IPC message between processes
    pub fn send_message(&mut self, from: ProcessId, to: ProcessId, message_type: IpcMessageType, data: Vec<u8>) -> AgaveResult<()> {
        if !self.processes.contains_key(&from) || !self.processes.contains_key(&to) {
            return Err(AgaveError::NotFound);
        }
        
        let message = IpcMessage {
            from,
            to,
            message_type,
            data,
            timestamp: crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed),
        };
        
        if let Some(target_process) = self.processes.get_mut(&to) {
            target_process.ipc_inbox.send(message)?;
            target_process.statistics.messages_received += 1;
        }
        
        if let Some(sender_process) = self.processes.get_mut(&from) {
            sender_process.statistics.messages_sent += 1;
        }
        
        Ok(())
    }
    
    /// Receive IPC message for a process
    pub fn receive_message(&mut self, pid: ProcessId) -> Option<IpcMessage> {
        if let Some(process) = self.processes.get_mut(&pid) {
            process.ipc_inbox.receive()
        } else {
            None
        }
    }
    
    /// Kill a process
    pub fn kill_process(&mut self, pid: ProcessId, signal: i32) -> AgaveResult<()> {
        if let Some(process) = self.processes.get_mut(&pid) {
            if process.context.state != ProcessState::Terminated {
                process.context.state = ProcessState::Terminated;
                process.context.exit_code = Some(signal);
                process.task = None;
                
                self.active_processes.fetch_sub(1, Ordering::Relaxed);
                
                // Remove from ready queues
                for queue in &mut self.ready_queues {
                    queue.retain(|&p| p != pid);
                }
                
                if self.current_process == Some(pid) {
                    self.current_process = None;
                }
                
                add_diagnostic(
                    DiagnosticLevel::Info,
                    DiagnosticCategory::Tasks,
                    format!("Process killed: {} (signal: {})", pid.as_u64(), signal),
                    None
                );
                
                log::info!("Killed process {} with signal {}", pid.as_u64(), signal);
                Ok(())
            } else {
                Err(AgaveError::InvalidState)
            }
        } else {
            Err(AgaveError::NotFound)
        }
    }
    
    /// Get process information
    pub fn get_process_info(&self, pid: ProcessId) -> Option<&ProcessContext> {
        self.processes.get(&pid).map(|pcb| &pcb.context)
    }
    
    /// List all processes
    pub fn list_processes(&self) -> Vec<&ProcessContext> {
        self.processes.values().map(|pcb| &pcb.context).collect()
    }
    
    /// Get system statistics
    pub fn get_statistics(&self) -> ProcessSystemStats {
        let total = self.total_processes.load(Ordering::Relaxed);
        let active = self.active_processes.load(Ordering::Relaxed);
        
        let mut by_state = BTreeMap::new();
        let mut by_priority = BTreeMap::new();
        let mut total_memory = 0;
        let mut total_cpu_time = 0;
        
        for pcb in self.processes.values() {
            *by_state.entry(pcb.context.state.clone()).or_insert(0) += 1;
            *by_priority.entry(pcb.context.priority).or_insert(0) += 1;
            total_memory += pcb.context.memory_usage;
            total_cpu_time += pcb.context.cpu_time_ms;
        }
        
        ProcessSystemStats {
            total_processes: total,
            active_processes: active,
            processes_by_state: by_state,
            processes_by_priority: by_priority,
            total_memory_usage: total_memory,
            total_cpu_time_ms: total_cpu_time,
        }
    }
    
    /// Cleanup terminated processes
    pub fn cleanup_terminated(&mut self) {
        let terminated_pids: Vec<ProcessId> = self.processes
            .iter()
            .filter(|(_, pcb)| pcb.context.state == ProcessState::Terminated)
            .map(|(&pid, _)| pid)
            .collect();
        
        for pid in terminated_pids {
            self.processes.remove(&pid);
            log::trace!("Cleaned up terminated process {}", pid.as_u64());
        }
    }
}

/// System-wide process statistics
#[derive(Debug, Clone)]
pub struct ProcessSystemStats {
    pub total_processes: u32,
    pub active_processes: u32,
    pub processes_by_state: BTreeMap<ProcessState, u32>,
    pub processes_by_priority: BTreeMap<Priority, u32>,
    pub total_memory_usage: usize,
    pub total_cpu_time_ms: u64,
}

/// Global process manager
static PROCESS_MANAGER: Mutex<ProcessScheduler> = Mutex::new(ProcessScheduler {
    processes: BTreeMap::new(),
    ready_queues: [
        VecDeque::new(), VecDeque::new(), VecDeque::new(),
        VecDeque::new(), VecDeque::new()
    ],
    current_process: None,
    next_schedule_time: 0,
    schedule_quantum_ms: 10,
    total_processes: AtomicU32::new(0),
    active_processes: AtomicU32::new(0),
});

/// Public API functions
pub fn spawn_process<F>(name: String, priority: Priority, future: F, parent_pid: Option<ProcessId>) -> AgaveResult<ProcessId>
where
    F: Future<Output = i32> + Send + 'static,
{
    let mut manager = PROCESS_MANAGER.lock();
    manager.spawn_process(name, priority, future, parent_pid)
}

pub fn schedule_processes() -> Option<ProcessId> {
    let mut manager = PROCESS_MANAGER.lock();
    manager.schedule()
}

pub fn run_current_process(cx: &mut Context<'_>) -> Poll<()> {
    let mut manager = PROCESS_MANAGER.lock();
    manager.run_current_process(cx)
}

pub fn send_ipc_message(from: ProcessId, to: ProcessId, message_type: IpcMessageType, data: Vec<u8>) -> AgaveResult<()> {
    let mut manager = PROCESS_MANAGER.lock();
    manager.send_message(from, to, message_type, data)
}

pub fn receive_ipc_message(pid: ProcessId) -> Option<IpcMessage> {
    let mut manager = PROCESS_MANAGER.lock();
    manager.receive_message(pid)
}

pub fn kill_process(pid: ProcessId, signal: i32) -> AgaveResult<()> {
    let mut manager = PROCESS_MANAGER.lock();
    manager.kill_process(pid, signal)
}

pub fn get_process_info(pid: ProcessId) -> Option<ProcessContext> {
    let manager = PROCESS_MANAGER.lock();
    manager.get_process_info(pid).cloned()
}

pub fn list_processes() -> Vec<ProcessContext> {
    let manager = PROCESS_MANAGER.lock();
    manager.list_processes().into_iter().cloned().collect()
}

pub fn get_process_statistics() -> ProcessSystemStats {
    let manager = PROCESS_MANAGER.lock();
    manager.get_statistics()
}

pub fn cleanup_terminated_processes() {
    let mut manager = PROCESS_MANAGER.lock();
    manager.cleanup_terminated();
}

/// Initialize process management system
pub fn init_process_management() {
    log::info!("Process management system initialized");
    add_diagnostic(
        DiagnosticLevel::Info,
        DiagnosticCategory::System,
        "Process management initialized".to_string(),
        Some("Multi-priority scheduling and IPC enabled".to_string())
    );
}
