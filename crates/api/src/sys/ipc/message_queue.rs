/// Message queue implementation for IPC
use crate::sys::error::{AgaveError, AgaveResult};
use crate::sys::process::ProcessId;
use alloc::{collections::VecDeque, vec::Vec};
use core::cmp::Ordering;
use spin::Mutex;

/// Message in a message queue
#[derive(Debug, Clone)]
pub struct Message {
    pub data: Vec<u8>,
    pub priority: u32,
    pub timestamp: u64,
    pub sender: ProcessId,
}

impl Message {
    pub fn new(data: Vec<u8>, priority: u32, sender: ProcessId) -> Self {
        let timestamp = crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);
        Self {
            data,
            priority,
            timestamp,
            sender,
        }
    }

    pub fn size(&self) -> usize {
        self.data.len()
    }
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.timestamp == other.timestamp
    }
}

impl Eq for Message {}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Message {
    fn cmp(&self, other: &Self) -> Ordering {
        // Higher priority messages come first
        // If priority is equal, earlier timestamp comes first
        match other.priority.cmp(&self.priority) {
            Ordering::Equal => self.timestamp.cmp(&other.timestamp),
            other => other,
        }
    }
}

/// Message queue for inter-process communication
#[derive(Debug)]
pub struct MessageQueue {
    messages: Mutex<VecDeque<Message>>,
    max_messages: usize,
    max_message_size: usize,
    total_size: Mutex<usize>,
    owner: ProcessId,
    creation_time: u64,
    stats: Mutex<MessageQueueStats>,
    wait_queue: Mutex<Vec<ProcessId>>,
    nonblocking: bool,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new(
        max_messages: usize,
        max_message_size: usize,
        owner: ProcessId,
    ) -> AgaveResult<Self> {
        if max_messages == 0 || max_messages > MAX_QUEUE_MESSAGES {
            return Err(AgaveError::InvalidParameter);
        }

        if max_message_size == 0 || max_message_size > MAX_MESSAGE_SIZE {
            return Err(AgaveError::InvalidParameter);
        }

        let current_time =
            crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);

        Ok(Self {
            messages: Mutex::new(VecDeque::with_capacity(max_messages)),
            max_messages,
            max_message_size,
            total_size: Mutex::new(0),
            owner,
            creation_time: current_time,
            stats: Mutex::new(MessageQueueStats::default()),
            wait_queue: Mutex::new(Vec::new()),
            nonblocking: false,
        })
    }

    /// Send a message to the queue (blocking/non-blocking)
    pub fn send(&self, data: &[u8], priority: u32) -> AgaveResult<()> {
        if data.len() > self.max_message_size {
            return Err(AgaveError::MessageTooLarge);
        }

        let mut messages = self.messages.lock();
        let mut total_size = self.total_size.lock();
        let mut stats = self.stats.lock();

        // Wait if full and blocking
        while messages.len() >= self.max_messages {
            if self.nonblocking {
                stats.messages_dropped += 1;
                return Err(AgaveError::QueueFull);
            }
            drop(messages);
            self.wait_current_process();
            messages = self.messages.lock();
        }

        let message = Message::new(data.to_vec(), priority, self.owner);
        let message_size = message.size();

        // Insert message in priority order
        let insert_pos = messages
            .iter()
            .position(|m| message < *m)
            .unwrap_or(messages.len());

        messages.insert(insert_pos, message);
        *total_size += message_size;

        // Update statistics
        stats.messages_sent += 1;
        stats.bytes_sent += message_size;
        if priority > 0 {
            stats.priority_messages += 1;
        }

        log::trace!(
            "Message sent to queue: {} bytes, priority {}, queue size: {}/{}",
            message_size,
            priority,
            messages.len(),
            self.max_messages
        );

        self.wake_one_waiter();
        Ok(())
    }

    /// Receive a message from the queue (blocking/non-blocking)
    pub fn receive(&self, buffer: &mut [u8]) -> AgaveResult<(usize, u32)> {
        let mut messages = self.messages.lock();
        let mut total_size = self.total_size.lock();
        let mut stats = self.stats.lock();

        // Wait if empty and blocking
        while messages.is_empty() {
            if self.nonblocking {
                return Err(AgaveError::QueueEmpty);
            }
            drop(messages);
            self.wait_current_process();
            messages = self.messages.lock();
        }

        let message = messages.pop_front().ok_or(AgaveError::QueueEmpty)?;

        if buffer.len() < message.data.len() {
            // Put the message back at the front
            messages.push_front(message);
            return Err(AgaveError::BufferTooSmall);
        }

        let message_size = message.data.len();
        let priority = message.priority;

        buffer[..message_size].copy_from_slice(&message.data);
        *total_size -= message_size;

        // Update statistics
        stats.messages_received += 1;
        stats.bytes_received += message_size;

        log::trace!(
            "Message received from queue: {} bytes, priority {}, queue size: {}/{}",
            message_size,
            priority,
            messages.len(),
            self.max_messages
        );

        self.wake_one_waiter();
        Ok((message_size, priority))
    }
    /// Add current process to wait queue and sleep (integrated with scheduler)
    fn wait_current_process(&self) {
        let pid = crate::sys::process::get_current_pid();
        let mut queue = self.wait_queue.lock();
        queue.push(pid);
        // Set process state to Waiting and remove from ready queue
    crate::sys::process::set_process_state(pid, crate::sys::process::ProcessState::Waiting);
        // No direct access to ready_queues; rely on scheduler to skip Waiting processes
    }

    /// Wake one waiting process (integrated with scheduler)
    fn wake_one_waiter(&self) {
        let mut queue = self.wait_queue.lock();
        if let Some(pid) = queue.pop() {
            crate::sys::process::set_process_state(pid, crate::sys::process::ProcessState::Created);
            // To re-enqueue, rely on scheduler to pick up Created processes
        }
    }
    /// Set non-blocking mode for send/receive
    pub fn set_nonblocking(&mut self, nonblocking: bool) {
        self.nonblocking = nonblocking;
    }

    /// Peek at the next message without removing it
    pub fn peek(&self) -> AgaveResult<(usize, u32, u64)> {
        let messages = self.messages.lock();
        let message = messages.front().ok_or(AgaveError::QueueEmpty)?;
        Ok((message.data.len(), message.priority, message.timestamp))
    }

    /// Get the number of messages in the queue
    pub fn len(&self) -> usize {
        self.messages.lock().len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.messages.lock().is_empty()
    }

    /// Check if the queue is full
    pub fn is_full(&self) -> bool {
        self.messages.lock().len() >= self.max_messages
    }

    /// Get the total size of all messages in bytes
    pub fn total_size(&self) -> usize {
        *self.total_size.lock()
    }

    /// Get queue capacity information
    pub fn capacity(&self) -> (usize, usize) {
        (self.max_messages, self.max_message_size)
    }

    /// Get queue owner
    pub fn owner(&self) -> ProcessId {
        self.owner
    }

    /// Clear all messages from the queue
    pub fn clear(&self) -> AgaveResult<()> {
        let mut messages = self.messages.lock();
        let mut total_size = self.total_size.lock();
        let mut stats = self.stats.lock();

        let cleared_count = messages.len();
        let cleared_bytes = *total_size;

        messages.clear();
        *total_size = 0;

        stats.messages_dropped += cleared_count;
        stats.queue_clears += 1;

        log::debug!(
            "Message queue cleared: {} messages, {} bytes",
            cleared_count,
            cleared_bytes
        );
        Ok(())
    }

    /// Get queue statistics
    pub fn get_stats(&self) -> MessageQueueStats {
        let stats = self.stats.lock();
        let mut result = stats.clone();

        // Update current state
        result.current_messages = self.len();
        result.current_bytes = self.total_size();
        result.max_messages = self.max_messages;
        result.max_message_size = self.max_message_size;
        result.creation_time = self.creation_time;

        result
    }

    /// Set queue attributes (if supported)
    pub fn set_attributes(
        &mut self,
        max_messages: Option<usize>,
        max_message_size: Option<usize>,
    ) -> AgaveResult<()> {
        if let Some(max_msg) = max_messages {
            if max_msg == 0 || max_msg > MAX_QUEUE_MESSAGES {
                return Err(AgaveError::InvalidParameter);
            }

            let current_len = self.messages.lock().len();
            if max_msg < current_len {
                return Err(AgaveError::InvalidOperation); // Would lose messages
            }

            self.max_messages = max_msg;
        }

        if let Some(max_size) = max_message_size {
            if max_size == 0 || max_size > MAX_MESSAGE_SIZE {
                return Err(AgaveError::InvalidParameter);
            }

            self.max_message_size = max_size;
        }

        log::debug!(
            "Queue attributes updated: max_messages={}, max_message_size={}",
            self.max_messages,
            self.max_message_size
        );
        Ok(())
    }
}

/// Message queue statistics
#[derive(Debug, Clone, Default)]
pub struct MessageQueueStats {
    pub current_messages: usize,
    pub current_bytes: usize,
    pub max_messages: usize,
    pub max_message_size: usize,
    pub messages_sent: usize,
    pub messages_received: usize,
    pub messages_dropped: usize,
    pub bytes_sent: usize,
    pub bytes_received: usize,
    pub priority_messages: usize,
    pub queue_clears: usize,
    pub creation_time: u64,
}

impl MessageQueueStats {
    pub fn utilization_percent(&self) -> f32 {
        if self.max_messages == 0 {
            0.0
        } else {
            (self.current_messages as f32 / self.max_messages as f32) * 100.0
        }
    }

    pub fn average_message_size(&self) -> f32 {
        if self.messages_sent == 0 {
            0.0
        } else {
            self.bytes_sent as f32 / self.messages_sent as f32
        }
    }

    pub fn throughput_messages_per_sec(&self) -> f32 {
        let current_time =
            crate::sys::interrupts::TIME_MS.load(core::sync::atomic::Ordering::Relaxed);
        let elapsed_seconds = (current_time - self.creation_time) as f32 / 1000.0;

        if elapsed_seconds <= 0.0 {
            0.0
        } else {
            (self.messages_sent + self.messages_received) as f32 / elapsed_seconds
        }
    }
}

/// Constants for message queue limits
pub const MAX_QUEUE_MESSAGES: usize = 10000; // Maximum messages per queue
pub const MAX_MESSAGE_SIZE: usize = 8192; // 8KB maximum message size
pub const DEFAULT_MAX_MESSAGES: usize = 100; // Default queue capacity
pub const DEFAULT_MAX_MESSAGE_SIZE: usize = 1024; // Default message size limit

/// POSIX-style message queue attributes
#[derive(Debug, Clone)]
pub struct MqAttr {
    pub flags: i32,            // Message queue flags
    pub max_messages: i32,     // Maximum number of messages
    pub max_message_size: i32, // Maximum message size
    pub current_messages: i32, // Current number of messages
}

impl From<&MessageQueue> for MqAttr {
    fn from(mq: &MessageQueue) -> Self {
        Self {
            flags: 0, // Non-blocking = 0, blocking = O_NONBLOCK
            max_messages: mq.max_messages as i32,
            max_message_size: mq.max_message_size as i32,
            current_messages: mq.len() as i32,
        }
    }
}

/// Message queue notification (for advanced features)
#[derive(Debug, Clone)]
pub enum MqNotification {
    None,
    Signal(i32), // Send signal to process
    Thread(u64), // Create thread with function pointer
}

/// Advanced message queue with notification support
#[derive(Debug)]
pub struct AdvancedMessageQueue {
    basic_queue: MessageQueue,
    notification: Option<MqNotification>,
    notify_process: Option<ProcessId>,
}

impl AdvancedMessageQueue {
    pub fn new(
        max_messages: usize,
        max_message_size: usize,
        owner: ProcessId,
    ) -> AgaveResult<Self> {
        Ok(Self {
            basic_queue: MessageQueue::new(max_messages, max_message_size, owner)?,
            notification: None,
            notify_process: None,
        })
    }

    pub fn set_notification(&mut self, notification: MqNotification, process: ProcessId) {
        log::debug!(
            "Message queue notification set for process {}: {:?}",
            process,
            &notification
        );
        self.notification = Some(notification);
        self.notify_process = Some(process);
    }

    pub fn send(&self, data: &[u8], priority: u32) -> AgaveResult<()> {
        let was_empty = self.basic_queue.is_empty();
        let result = self.basic_queue.send(data, priority);

        // Send notification if queue was empty and now has messages
        if result.is_ok() && was_empty && self.notification.is_some() {
            self.send_notification();
        }

        result
    }

    pub fn receive(&self, buffer: &mut [u8]) -> AgaveResult<(usize, u32)> {
        self.basic_queue.receive(buffer)
    }

    fn send_notification(&self) {
        if let (Some(notification), Some(process)) = (&self.notification, self.notify_process) {
            match notification {
                MqNotification::Signal(sig) => {
                    log::debug!(
                        "Sending signal {} to process {} for message queue notification",
                        sig,
                        process
                    );
                    // TODO: Implement signal sending
                }
                MqNotification::Thread(func_ptr) => {
                    log::debug!(
                        "Creating notification thread for process {} with function 0x{:x}",
                        process,
                        func_ptr
                    );
                    // TODO: Implement thread creation
                }
                MqNotification::None => {}
            }
        }
    }

    // Delegate other methods to the basic queue
    pub fn peek(&self) -> AgaveResult<(usize, u32, u64)> {
        self.basic_queue.peek()
    }

    pub fn len(&self) -> usize {
        self.basic_queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.basic_queue.is_empty()
    }

    pub fn is_full(&self) -> bool {
        self.basic_queue.is_full()
    }

    pub fn get_stats(&self) -> MessageQueueStats {
        self.basic_queue.get_stats()
    }

    pub fn clear(&self) -> AgaveResult<()> {
        self.basic_queue.clear()
    }
}
