/// System monitoring and diagnostics for Agave OS
use crate::sys::{
    allocator::{memory_pressure, memory_stats, MemoryPressure, MemoryStats},
    interrupts::{RANDTHING1, TIME_MS},
    task::executor::{TaskMetrics, TASK_METRICS},
};
use alloc::vec::Vec;
use core::sync::atomic::Ordering;
use spin::Mutex;

/// System performance metrics
#[derive(Debug, Clone)]
pub struct SystemMetrics {
    pub uptime_ms: u64,
    pub memory: MemoryStats,
    pub memory_pressure: MemoryPressure,
    pub tasks: TaskMetrics,
    pub interrupts_handled: u64,
    pub context_switches: u64,
    pub cpu_utilization_percent: f32,
}

/// Performance event types
#[derive(Debug, Clone)]
pub enum PerformanceEvent {
    MemoryPressureHigh,
    TaskQueueFull,
    ExcessiveContextSwitches,
    LongRunningTask { task_id: u64, duration_ms: u64 },
    MemoryLeak { growth_rate_kb_per_sec: f32 },
}

/// System health status
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Performance monitor
pub struct PerformanceMonitor {
    events: Vec<PerformanceEvent>,
    last_memory_check: u64,
    baseline_memory: usize,
    max_events: usize,
}

static PERFORMANCE_MONITOR: Mutex<PerformanceMonitor> = Mutex::new(PerformanceMonitor {
    events: Vec::new(),
    last_memory_check: 0,
    baseline_memory: 0,
    max_events: 100,
});

impl PerformanceMonitor {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            last_memory_check: 0,
            baseline_memory: 0,
            max_events: 100,
        }
    }

    pub fn record_event(&mut self, event: PerformanceEvent) {
        self.events.push(event);

        // Keep only the most recent events
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }
    }

    pub fn check_system_health(&mut self) -> HealthStatus {
        let current_time = TIME_MS.load(Ordering::Relaxed);

        // Check memory pressure
        match memory_pressure() {
            MemoryPressure::Critical => {
                self.record_event(PerformanceEvent::MemoryPressureHigh);
                return HealthStatus::Critical;
            }
            MemoryPressure::High => {
                self.record_event(PerformanceEvent::MemoryPressureHigh);
            }
            _ => {}
        }

        // Check for memory leaks
        if current_time - self.last_memory_check > 5000 {
            // Check every 5 seconds
            let current_memory = memory_stats().allocated;
            if self.baseline_memory == 0 {
                self.baseline_memory = current_memory;
            } else {
                let growth = current_memory.saturating_sub(self.baseline_memory);
                let time_diff_sec = (current_time - self.last_memory_check) as f32 / 1000.0;
                let growth_rate = growth as f32 / 1024.0 / time_diff_sec; // KB/sec

                if growth_rate > 100.0 {
                    // More than 100KB/sec growth
                    self.record_event(PerformanceEvent::MemoryLeak {
                        growth_rate_kb_per_sec: growth_rate,
                    });
                }

                self.baseline_memory = current_memory;
            }
            self.last_memory_check = current_time;
        }

        // Check recent events for warnings/critical issues
        let recent_critical_events = self
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    PerformanceEvent::MemoryPressureHigh | PerformanceEvent::TaskQueueFull
                )
            })
            .count();

        if recent_critical_events > 5 {
            HealthStatus::Critical
        } else if recent_critical_events > 2 {
            HealthStatus::Warning
        } else {
            HealthStatus::Healthy
        }
    }

    pub fn get_recent_events(&self) -> &[PerformanceEvent] {
        &self.events
    }
}

/// Get current system metrics
pub fn get_system_metrics() -> SystemMetrics {
    let memory = memory_stats();
    let tasks = {
        let metrics = TASK_METRICS.lock();
        TaskMetrics {
            total_tasks_spawned: metrics.total_tasks_spawned,
            tasks_completed: metrics.tasks_completed,
            total_execution_time_us: metrics.total_execution_time_us,
            context_switches: metrics.context_switches,
        }
    };

    // Estimate CPU utilization based on task execution
    let cpu_utilization = if tasks.total_execution_time_us > 0 {
        let uptime_us = TIME_MS.load(Ordering::Relaxed) * 1000;
        if uptime_us > 0 {
            ((tasks.total_execution_time_us as f32 / uptime_us as f32) * 100.0).min(100.0)
        } else {
            0.0
        }
    } else {
        0.0
    };

    SystemMetrics {
        uptime_ms: TIME_MS.load(Ordering::Relaxed),
        memory,
        memory_pressure: memory_pressure(),
        tasks: tasks.clone(),
        interrupts_handled: RANDTHING1.load(Ordering::Relaxed) as u64,
        context_switches: tasks.context_switches,
        cpu_utilization_percent: cpu_utilization,
    }
}

/// Get system health status
pub fn get_system_health() -> HealthStatus {
    let mut monitor = PERFORMANCE_MONITOR.lock();
    monitor.check_system_health()
}

/// Get recent performance events
pub fn get_recent_events() -> Vec<PerformanceEvent> {
    let monitor = PERFORMANCE_MONITOR.lock();
    monitor.get_recent_events().to_vec()
}

/// Log system status summary
pub fn log_system_status() {
    let metrics = get_system_metrics();
    let health = get_system_health();

    log::info!("=== System Status ===");
    log::info!("Health: {:?}", health);
    log::info!("Uptime: {}ms", metrics.uptime_ms);
    log::info!(
        "Memory: {}/{} bytes ({:.1}%)",
        metrics.memory.allocated,
        metrics.memory.heap_size,
        metrics.memory.utilization_percent()
    );
    log::info!("Memory Pressure: {:?}", metrics.memory_pressure);
    log::info!(
        "Tasks: {} spawned, {} completed",
        metrics.tasks.total_tasks_spawned,
        metrics.tasks.tasks_completed
    );
    log::info!("CPU Utilization: {:.1}%", metrics.cpu_utilization_percent);
    log::info!("Context Switches: {}", metrics.context_switches);

    // Log any critical events
    let events = get_recent_events();
    for event in events.iter().take(5) {
        log::warn!("Performance Event: {:?}", event);
    }
}

/// Simple built-in profiler for measuring function execution time
pub struct Profiler {
    name: &'static str,
    start_time: u64,
}

impl Profiler {
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            start_time: TIME_MS.load(Ordering::Relaxed),
        }
    }

    pub fn finish(self) -> u64 {
        let duration = TIME_MS.load(Ordering::Relaxed) - self.start_time;
        if duration > 10 {
            // Log functions taking more than 10ms
            log::debug!("Profile {}: {}ms", self.name, duration);
        }
        duration
    }
}

/// Macro for easy profiling
#[macro_export]
macro_rules! profile {
    ($name:expr, $code:block) => {{
        let _profiler = crate::sys::monitor::Profiler::new($name);
        let result = $code;
        _profiler.finish();
        result
    }};
}

/// Initialize monitoring system
pub fn init_monitoring() {
    log::info!("Performance monitoring initialized");

    // Set baseline memory usage
    let mut monitor = PERFORMANCE_MONITOR.lock();
    monitor.baseline_memory = memory_stats().allocated;
    monitor.last_memory_check = TIME_MS.load(Ordering::Relaxed);
}

/// Periodic monitoring task (should be called regularly)
pub fn periodic_monitor_check() {
    let health = get_system_health();

    match health {
        HealthStatus::Critical => {
            log::error!("System health critical!");
            log_system_status();
        }
        HealthStatus::Warning => {
            log::warn!("System health warning");
        }
        _ => {}
    }
}
