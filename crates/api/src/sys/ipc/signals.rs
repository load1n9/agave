/// Signal implementation for IPC
use crate::sys::{
    error::{AgaveError, AgaveResult},
    ipc::ProcessId,
};
use alloc::{collections::BTreeMap, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Standard Unix-like signals
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(i32)]
pub enum Signal {
    SIGHUP = 1,     // Hangup
    SIGINT = 2,     // Interrupt (Ctrl+C)
    SIGQUIT = 3,    // Quit (Ctrl+\)
    SIGILL = 4,     // Illegal instruction
    SIGTRAP = 5,    // Trace/breakpoint trap
    SIGABRT = 6,    // Abort
    SIGBUS = 7,     // Bus error
    SIGFPE = 8,     // Floating point exception
    SIGKILL = 9,    // Kill (cannot be caught or ignored)
    SIGUSR1 = 10,   // User-defined signal 1
    SIGSEGV = 11,   // Segmentation violation
    SIGUSR2 = 12,   // User-defined signal 2
    SIGPIPE = 13,   // Broken pipe
    SIGALRM = 14,   // Alarm clock
    SIGTERM = 15,   // Termination
    SIGCHLD = 17,   // Child status changed
    SIGCONT = 18,   // Continue
    SIGSTOP = 19,   // Stop (cannot be caught or ignored)
    SIGTSTP = 20,   // Terminal stop (Ctrl+Z)
    SIGTTIN = 21,   // Background read from terminal
    SIGTTOU = 22,   // Background write to terminal
    SIGURG = 23,    // Urgent condition on socket
    SIGXCPU = 24,   // CPU limit exceeded
    SIGXFSZ = 25,   // File size limit exceeded
    SIGVTALRM = 26, // Virtual alarm clock
    SIGPROF = 27,   // Profiling alarm clock
    SIGWINCH = 28,  // Window size change
    SIGIO = 29,     // I/O now possible
    SIGPWR = 30,    // Power failure
    SIGSYS = 31,    // Bad system call
}

impl Signal {
    /// Convert from signal number to Signal enum
    pub fn from_number(num: i32) -> Option<Self> {
        match num {
            1 => Some(Signal::SIGHUP),
            2 => Some(Signal::SIGINT),
            3 => Some(Signal::SIGQUIT),
            4 => Some(Signal::SIGILL),
            5 => Some(Signal::SIGTRAP),
            6 => Some(Signal::SIGABRT),
            7 => Some(Signal::SIGBUS),
            8 => Some(Signal::SIGFPE),
            9 => Some(Signal::SIGKILL),
            10 => Some(Signal::SIGUSR1),
            11 => Some(Signal::SIGSEGV),
            12 => Some(Signal::SIGUSR2),
            13 => Some(Signal::SIGPIPE),
            14 => Some(Signal::SIGALRM),
            15 => Some(Signal::SIGTERM),
            17 => Some(Signal::SIGCHLD),
            18 => Some(Signal::SIGCONT),
            19 => Some(Signal::SIGSTOP),
            20 => Some(Signal::SIGTSTP),
            21 => Some(Signal::SIGTTIN),
            22 => Some(Signal::SIGTTOU),
            23 => Some(Signal::SIGURG),
            24 => Some(Signal::SIGXCPU),
            25 => Some(Signal::SIGXFSZ),
            26 => Some(Signal::SIGVTALRM),
            27 => Some(Signal::SIGPROF),
            28 => Some(Signal::SIGWINCH),
            29 => Some(Signal::SIGIO),
            30 => Some(Signal::SIGPWR),
            31 => Some(Signal::SIGSYS),
            _ => None,
        }
    }

    /// Get signal number
    pub fn number(&self) -> i32 {
        *self as i32
    }

    /// Get signal name
    pub fn name(&self) -> &'static str {
        match self {
            Signal::SIGHUP => "SIGHUP",
            Signal::SIGINT => "SIGINT",
            Signal::SIGQUIT => "SIGQUIT",
            Signal::SIGILL => "SIGILL",
            Signal::SIGTRAP => "SIGTRAP",
            Signal::SIGABRT => "SIGABRT",
            Signal::SIGBUS => "SIGBUS",
            Signal::SIGFPE => "SIGFPE",
            Signal::SIGKILL => "SIGKILL",
            Signal::SIGUSR1 => "SIGUSR1",
            Signal::SIGSEGV => "SIGSEGV",
            Signal::SIGUSR2 => "SIGUSR2",
            Signal::SIGPIPE => "SIGPIPE",
            Signal::SIGALRM => "SIGALRM",
            Signal::SIGTERM => "SIGTERM",
            Signal::SIGCHLD => "SIGCHLD",
            Signal::SIGCONT => "SIGCONT",
            Signal::SIGSTOP => "SIGSTOP",
            Signal::SIGTSTP => "SIGTSTP",
            Signal::SIGTTIN => "SIGTTIN",
            Signal::SIGTTOU => "SIGTTOU",
            Signal::SIGURG => "SIGURG",
            Signal::SIGXCPU => "SIGXCPU",
            Signal::SIGXFSZ => "SIGXFSZ",
            Signal::SIGVTALRM => "SIGVTALRM",
            Signal::SIGPROF => "SIGPROF",
            Signal::SIGWINCH => "SIGWINCH",
            Signal::SIGIO => "SIGIO",
            Signal::SIGPWR => "SIGPWR",
            Signal::SIGSYS => "SIGSYS",
        }
    }

    /// Check if signal can be caught or ignored
    pub fn can_be_caught(&self) -> bool {
        !matches!(self, Signal::SIGKILL | Signal::SIGSTOP)
    }

    /// Check if signal terminates the process by default
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Signal::SIGQUIT
                | Signal::SIGILL
                | Signal::SIGTRAP
                | Signal::SIGABRT
                | Signal::SIGBUS
                | Signal::SIGFPE
                | Signal::SIGKILL
                | Signal::SIGSEGV
                | Signal::SIGPIPE
                | Signal::SIGTERM
                | Signal::SIGXCPU
                | Signal::SIGXFSZ
                | Signal::SIGSYS
        )
    }
}

/// Signal action types
#[derive(Debug, Clone)]
pub enum SignalAction {
    Default,                 // Use default action
    Ignore,                  // Ignore the signal
    Handle(SignalHandlerFn), // Custom handler function
    Stop,                    // Stop the process
    Continue,                // Continue the process
}

/// Signal handler function type
pub type SignalHandlerFn = fn(Signal, ProcessId);

/// Signal information
#[derive(Debug, Clone)]
pub struct SignalInfo {
    pub signal: Signal,
    pub sender: ProcessId,
    pub timestamp: u64,
    pub data: Option<u64>, // Additional signal data
}

/// Signal mask for blocking signals
#[derive(Debug, Clone, Default)]
pub struct SignalMask {
    mask: u64, // Bit mask for signals 1-64
}

impl SignalMask {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a signal to the mask
    pub fn add(&mut self, signal: Signal) {
        let bit = signal.number() as u64;
        if bit > 0 && bit <= 64 {
            self.mask |= 1 << (bit - 1);
        }
    }

    /// Remove a signal from the mask
    pub fn remove(&mut self, signal: Signal) {
        let bit = signal.number() as u64;
        if bit > 0 && bit <= 64 {
            self.mask &= !(1 << (bit - 1));
        }
    }

    /// Check if a signal is blocked
    pub fn is_blocked(&self, signal: Signal) -> bool {
        let bit = signal.number() as u64;
        if bit > 0 && bit <= 64 {
            (self.mask & (1 << (bit - 1))) != 0
        } else {
            false
        }
    }

    /// Clear all signals from the mask
    pub fn clear(&mut self) {
        self.mask = 0;
    }

    /// Block all signals
    pub fn block_all(&mut self) {
        self.mask = !0; // All bits set
    }

    /// Get the raw mask value
    pub fn raw_mask(&self) -> u64 {
        self.mask
    }

    /// Set the raw mask value
    pub fn set_raw_mask(&mut self, mask: u64) {
        self.mask = mask;
    }
}

/// Signal handler for a process
#[derive(Debug)]
pub struct SignalHandler {
    process: ProcessId,
    actions: BTreeMap<Signal, SignalAction>,
    pending_signals: Mutex<Vec<SignalInfo>>,
    blocked_signals: Mutex<SignalMask>,
    signal_stack: Option<SignalStack>,
    stats: Mutex<SignalStats>,
}

impl SignalHandler {
    /// Create a new signal handler for a process
    pub fn new(process: ProcessId) -> Self {
        let mut actions = BTreeMap::new();

        // Set default actions for all signals
        for signal_num in 1..=31 {
            if let Some(signal) = Signal::from_number(signal_num) {
                actions.insert(signal, SignalAction::Default);
            }
        }

        Self {
            process,
            actions,
            pending_signals: Mutex::new(Vec::new()),
            blocked_signals: Mutex::new(SignalMask::new()),
            signal_stack: None,
            stats: Mutex::new(SignalStats::default()),
        }
    }

    /// Set signal action
    pub fn set_action(
        &mut self,
        signal: Signal,
        action: SignalAction,
    ) -> AgaveResult<SignalAction> {
        if !signal.can_be_caught() {
            return Err(AgaveError::InvalidOperation);
        }

        let old_action = self
            .actions
            .get(&signal)
            .cloned()
            .unwrap_or(SignalAction::Default);
        self.actions.insert(signal, action);

        log::debug!(
            "Signal action set for process {}: {} -> {:?}",
            self.process,
            signal.name(),
            self.actions.get(&signal)
        );

        Ok(old_action)
    }

    /// Get signal action
    pub fn get_action(&self, signal: Signal) -> SignalAction {
        self.actions
            .get(&signal)
            .cloned()
            .unwrap_or(SignalAction::Default)
    }

    /// Send a signal to this process
    pub fn send_signal(
        &self,
        signal: Signal,
        sender: ProcessId,
        data: Option<u64>,
    ) -> AgaveResult<()> {
        let mut stats = self.stats.lock();
        stats.signals_received += 1;

        // Check if signal is blocked
        let blocked_signals = self.blocked_signals.lock();
        if blocked_signals.is_blocked(signal) && signal.can_be_caught() {
            drop(blocked_signals);

            // Add to pending signals
            let signal_info = SignalInfo {
                signal,
                sender,
                timestamp: crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed),
                data,
            };

            let mut pending = self.pending_signals.lock();
            pending.push(signal_info);

            stats.signals_pending += 1;
            log::debug!(
                "Signal {} blocked for process {}, added to pending",
                signal.name(),
                self.process
            );
            return Ok(());
        }
        drop(blocked_signals);

        // Deliver signal immediately
        self.deliver_signal(signal, sender, data)?;

        Ok(())
    }

    /// Deliver a signal (internal)
    fn deliver_signal(
        &self,
        signal: Signal,
        sender: ProcessId,
        data: Option<u64>,
    ) -> AgaveResult<()> {
        let action = self.get_action(signal);
        let mut stats = self.stats.lock();

        match action {
            SignalAction::Default => {
                stats.signals_handled_default += 1;
                self.handle_default_action(signal)?;
            }
            SignalAction::Ignore => {
                stats.signals_ignored += 1;
                log::debug!(
                    "Signal {} ignored by process {}",
                    signal.name(),
                    self.process
                );
            }
            SignalAction::Handle(handler) => {
                stats.signals_handled_custom += 1;
                log::debug!(
                    "Calling custom handler for signal {} in process {}",
                    signal.name(),
                    self.process
                );
                handler(signal, sender);
            }
            SignalAction::Stop => {
                stats.signals_stopped += 1;
                log::debug!(
                    "Process {} stopped by signal {}",
                    self.process,
                    signal.name()
                );
                // TODO: Actually stop the process
            }
            SignalAction::Continue => {
                stats.signals_continued += 1;
                log::debug!(
                    "Process {} continued by signal {}",
                    self.process,
                    signal.name()
                );
                // TODO: Actually continue the process
            }
        }

        Ok(())
    }

    /// Handle default signal action
    fn handle_default_action(&self, signal: Signal) -> AgaveResult<()> {
        match signal {
            Signal::SIGTERM | Signal::SIGINT | Signal::SIGQUIT => {
                log::info!(
                    "Process {} terminated by signal {}",
                    self.process,
                    signal.name()
                );
                // TODO: Terminate the process
            }
            Signal::SIGKILL => {
                log::info!(
                    "Process {} killed by signal {}",
                    self.process,
                    signal.name()
                );
                // TODO: Kill the process immediately
            }
            Signal::SIGSTOP | Signal::SIGTSTP => {
                log::info!(
                    "Process {} stopped by signal {}",
                    self.process,
                    signal.name()
                );
                // TODO: Stop the process
            }
            Signal::SIGCONT => {
                log::info!(
                    "Process {} continued by signal {}",
                    self.process,
                    signal.name()
                );
                // TODO: Continue the process
            }
            Signal::SIGCHLD => {
                log::debug!("Child status change signal for process {}", self.process);
                // Default is to ignore SIGCHLD
            }
            _ => {
                if signal.is_fatal() {
                    log::info!(
                        "Process {} terminated by fatal signal {}",
                        self.process,
                        signal.name()
                    );
                    // TODO: Terminate the process
                } else {
                    log::debug!(
                        "Signal {} delivered to process {} (default action: ignore)",
                        signal.name(),
                        self.process
                    );
                }
            }
        }
        Ok(())
    }

    /// Set signal mask (block/unblock signals)
    pub fn set_signal_mask(&self, mask: SignalMask) -> SignalMask {
        let mut blocked_signals = self.blocked_signals.lock();
        let old_mask = blocked_signals.clone();
        *blocked_signals = mask;

        // Deliver any unblocked pending signals
        self.deliver_pending_signals();

        old_mask
    }

    /// Get current signal mask
    pub fn get_signal_mask(&self) -> SignalMask {
        self.blocked_signals.lock().clone()
    }

    /// Deliver pending signals that are no longer blocked
    fn deliver_pending_signals(&self) {
        let blocked_signals = self.blocked_signals.lock();
        let blocked_mask = blocked_signals.clone();
        drop(blocked_signals);

        let mut pending = self.pending_signals.lock();
        let mut i = 0;

        while i < pending.len() {
            let signal_info = &pending[i];

            if !blocked_mask.is_blocked(signal_info.signal) {
                let signal_info = pending.remove(i);
                drop(pending);

                // Deliver the signal
                if let Err(e) =
                    self.deliver_signal(signal_info.signal, signal_info.sender, signal_info.data)
                {
                    log::error!(
                        "Failed to deliver pending signal {}: {:?}",
                        signal_info.signal.name(),
                        e
                    );
                }

                let mut stats = self.stats.lock();
                stats.signals_pending = stats.signals_pending.saturating_sub(1);
                drop(stats);

                pending = self.pending_signals.lock();
            } else {
                i += 1;
            }
        }
    }

    /// Wait for a signal (sigwait)
    pub fn wait_for_signal(&self, mask: &SignalMask) -> AgaveResult<Signal> {
        // Check pending signals first
        let mut pending = self.pending_signals.lock();

        for (i, signal_info) in pending.iter().enumerate() {
            if mask.is_blocked(signal_info.signal) {
                let signal = signal_info.signal;
                pending.remove(i);

                let mut stats = self.stats.lock();
                stats.signals_pending = stats.signals_pending.saturating_sub(1);
                stats.signals_waited += 1;

                return Ok(signal);
            }
        }

        drop(pending);

        // No matching pending signals, would need to block
        // In a real implementation, this would suspend the process
        log::debug!("Process {} waiting for signal (would block)", self.process);
        Err(AgaveError::WouldBlock)
    }

    /// Get signal statistics
    pub fn get_stats(&self) -> SignalStats {
        let stats = self.stats.lock();
        let mut result = stats.clone();

        result.signals_pending = self.pending_signals.lock().len();
        result
    }

    /// Clear all pending signals
    pub fn clear_pending(&self) {
        let mut pending = self.pending_signals.lock();
        let cleared_count = pending.len();
        pending.clear();

        let mut stats = self.stats.lock();
        stats.signals_pending = 0;
        stats.signals_discarded += cleared_count;

        log::debug!(
            "Cleared {} pending signals for process {}",
            cleared_count,
            self.process
        );
    }
}

/// Signal stack for alternate signal handling
#[derive(Debug)]
pub struct SignalStack {
    pub base: usize,
    pub size: usize,
    pub flags: u32,
}

/// Signal statistics
#[derive(Debug, Clone, Default)]
pub struct SignalStats {
    pub signals_received: usize,
    pub signals_sent: usize,
    pub signals_pending: usize,
    pub signals_handled_default: usize,
    pub signals_handled_custom: usize,
    pub signals_ignored: usize,
    pub signals_stopped: usize,
    pub signals_continued: usize,
    pub signals_waited: usize,
    pub signals_discarded: usize,
}

/// Default signal handlers
pub fn default_sigint_handler(signal: Signal, _sender: ProcessId) {
    log::info!("Default SIGINT handler called: {}", signal.name());
    // In a real system, this would terminate the process
}

pub fn default_sigterm_handler(signal: Signal, _sender: ProcessId) {
    log::info!("Default SIGTERM handler called: {}", signal.name());
    // In a real system, this would gracefully terminate the process
}

pub fn default_sigusr1_handler(signal: Signal, sender: ProcessId) {
    log::info!(
        "Default SIGUSR1 handler called: {} from process {}",
        signal.name(),
        sender
    );
    // User-defined behavior
}

pub fn default_sigusr2_handler(signal: Signal, sender: ProcessId) {
    log::info!(
        "Default SIGUSR2 handler called: {} from process {}",
        signal.name(),
        sender
    );
    // User-defined behavior
}
