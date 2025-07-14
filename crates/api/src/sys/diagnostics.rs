/// Real-time system diagnostics and health monitoring
use crate::sys::{
    allocator::{memory_stats, memory_pressure, MemoryPressure},
    interrupts::TIME_MS,
    task::executor::TASK_METRICS,
    monitor::{get_system_metrics, SystemMetrics},
};
use alloc::{vec::Vec, string::{String, ToString}, format, collections::BTreeMap};
use core::{sync::atomic::Ordering, fmt::Write};
use spin::Mutex;

/// System diagnostic levels
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticLevel {
    Info,
    Warning,
    Error,
    Critical,
    Fatal,
}

/// Diagnostic category
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum DiagnosticCategory {
    Memory,
    Tasks,
    Network,
    Filesystem,
    Hardware,
    Security,
    Performance,
    System,
}

/// Diagnostic entry
#[derive(Debug, Clone)]
pub struct DiagnosticEntry {
    pub timestamp: u64,
    pub level: DiagnosticLevel,
    pub category: DiagnosticCategory,
    pub message: String,
    pub details: Option<String>,
    pub count: u32, // How many times this issue occurred
}

/// System health checker
pub struct HealthChecker {
    diagnostics: Vec<DiagnosticEntry>,
    max_entries: usize,
    thresholds: DiagnosticThresholds,
    last_checks: BTreeMap<DiagnosticCategory, u64>,
}

/// Configurable thresholds for health checking
#[derive(Debug, Clone)]
pub struct DiagnosticThresholds {
    pub memory_warning_percent: f32,
    pub memory_critical_percent: f32,
    pub task_queue_warning_size: u32,
    pub task_queue_critical_size: u32,
    pub context_switch_warning_rate: f32, // per second
    pub uptime_check_interval_ms: u64,
}

impl Default for DiagnosticThresholds {
    fn default() -> Self {
        Self {
            memory_warning_percent: 75.0,
            memory_critical_percent: 90.0,
            task_queue_warning_size: 100,
            task_queue_critical_size: 500,
            context_switch_warning_rate: 1000.0,
            uptime_check_interval_ms: 30000, // 30 seconds
        }
    }
}

static HEALTH_CHECKER: Mutex<HealthChecker> = Mutex::new(HealthChecker {
    diagnostics: Vec::new(),
    max_entries: 1000,
    thresholds: DiagnosticThresholds {
        memory_warning_percent: 75.0,
        memory_critical_percent: 90.0,
        task_queue_warning_size: 100,
        task_queue_critical_size: 500,
        context_switch_warning_rate: 1000.0,
        uptime_check_interval_ms: 30000,
    },
    last_checks: BTreeMap::new(),
});

impl HealthChecker {
    /// Add a diagnostic entry
    pub fn add_diagnostic(&mut self, level: DiagnosticLevel, category: DiagnosticCategory, message: String, details: Option<String>) {
        let now = TIME_MS.load(Ordering::Relaxed);
        
        // Check if we already have this diagnostic (merge similar entries)
        if let Some(existing) = self.diagnostics.iter_mut()
            .find(|d| d.category == category && d.message == message && now - d.timestamp < 60000) {
            existing.count += 1;
            existing.timestamp = now;
            return;
        }
        
        let entry = DiagnosticEntry {
            timestamp: now,
            level,
            category,
            message,
            details,
            count: 1,
        };
        
        self.diagnostics.push(entry);
        
        // Keep only recent entries
        if self.diagnostics.len() > self.max_entries {
            self.diagnostics.remove(0);
        }
    }
    
    /// Perform comprehensive system health check
    pub fn perform_health_check(&mut self) -> SystemHealthReport {
        let now = TIME_MS.load(Ordering::Relaxed);
        let metrics = get_system_metrics();
        
        let mut issues = Vec::new();
        let mut overall_status = SystemHealthStatus::Healthy;
        
        // Memory health check
        if self.should_check(DiagnosticCategory::Memory, now) {
            let memory_issues = self.check_memory_health(&metrics);
            issues.extend(memory_issues);
        }
        
        // Task system health check
        if self.should_check(DiagnosticCategory::Tasks, now) {
            let task_issues = self.check_task_health(&metrics);
            issues.extend(task_issues);
        }
        
        // Performance health check
        if self.should_check(DiagnosticCategory::Performance, now) {
            let perf_issues = self.check_performance_health(&metrics);
            issues.extend(perf_issues);
        }
        
        // System stability check
        if self.should_check(DiagnosticCategory::System, now) {
            let system_issues = self.check_system_stability(&metrics);
            issues.extend(system_issues);
        }
        
        // Determine overall status based on highest severity issue
        for issue in &issues {
            match issue.level {
                DiagnosticLevel::Fatal | DiagnosticLevel::Critical => {
                    overall_status = SystemHealthStatus::Critical;
                    break;
                }
                DiagnosticLevel::Error => {
                    if overall_status != SystemHealthStatus::Critical {
                        overall_status = SystemHealthStatus::Degraded;
                    }
                }
                DiagnosticLevel::Warning => {
                    if overall_status == SystemHealthStatus::Healthy {
                        overall_status = SystemHealthStatus::Warning;
                    }
                }
                _ => {}
            }
        }
        
        SystemHealthReport {
            status: overall_status,
            timestamp: now,
            metrics: metrics.clone(),
            issues,
            uptime_ms: metrics.uptime_ms,
        }
    }
    
    fn should_check(&mut self, category: DiagnosticCategory, now: u64) -> bool {
        let last_check = self.last_checks.get(&category).copied().unwrap_or(0);
        if now - last_check >= self.thresholds.uptime_check_interval_ms {
            self.last_checks.insert(category, now);
            true
        } else {
            false
        }
    }
    
    fn check_memory_health(&mut self, metrics: &SystemMetrics) -> Vec<DiagnosticEntry> {
        let mut issues = Vec::new();
        let utilization = metrics.memory.utilization_percent();
        
        if utilization >= self.thresholds.memory_critical_percent {
            issues.push(DiagnosticEntry {
                timestamp: metrics.uptime_ms,
                level: DiagnosticLevel::Critical,
                category: DiagnosticCategory::Memory,
                message: format!("Critical memory usage: {:.1}%", utilization),
                details: Some(format!(
                    "Allocated: {} bytes, Total: {} bytes, Available: {} bytes",
                    metrics.memory.allocated,
                    metrics.memory.heap_size,
                    metrics.memory.heap_size - metrics.memory.allocated
                )),
                count: 1,
            });
        } else if utilization >= self.thresholds.memory_warning_percent {
            issues.push(DiagnosticEntry {
                timestamp: metrics.uptime_ms,
                level: DiagnosticLevel::Warning,
                category: DiagnosticCategory::Memory,
                message: format!("High memory usage: {:.1}%", utilization),
                details: Some(format!("Consider freeing unused resources")),
                count: 1,
            });
        }
        
        // Check memory pressure
        match metrics.memory_pressure {
            MemoryPressure::Critical => {
                issues.push(DiagnosticEntry {
                    timestamp: metrics.uptime_ms,
                    level: DiagnosticLevel::Critical,
                    category: DiagnosticCategory::Memory,
                    message: "Critical memory pressure detected".to_string(),
                    details: Some("System may become unresponsive".to_string()),
                    count: 1,
                });
            }
            MemoryPressure::High => {
                issues.push(DiagnosticEntry {
                    timestamp: metrics.uptime_ms,
                    level: DiagnosticLevel::Warning,
                    category: DiagnosticCategory::Memory,
                    message: "High memory pressure".to_string(),
                    details: Some("Performance may be affected".to_string()),
                    count: 1,
                });
            }
            _ => {}
        }
        
        issues
    }
    
    fn check_task_health(&mut self, metrics: &SystemMetrics) -> Vec<DiagnosticEntry> {
        let mut issues = Vec::new();
        
        // Check task completion rate
        let tasks = &metrics.tasks;
        if tasks.total_tasks_spawned > 0 {
            let completion_rate = tasks.tasks_completed as f32 / tasks.total_tasks_spawned as f32;
            if completion_rate < 0.5 {
                issues.push(DiagnosticEntry {
                    timestamp: metrics.uptime_ms,
                    level: DiagnosticLevel::Warning,
                    category: DiagnosticCategory::Tasks,
                    message: format!("Low task completion rate: {:.1}%", completion_rate * 100.0),
                    details: Some(format!(
                        "Spawned: {}, Completed: {}",
                        tasks.total_tasks_spawned,
                        tasks.tasks_completed
                    )),
                    count: 1,
                });
            }
        }
        
        // Check context switch rate
        let uptime_sec = metrics.uptime_ms as f32 / 1000.0;
        if uptime_sec > 0.0 {
            let switch_rate = metrics.context_switches as f32 / uptime_sec;
            if switch_rate > self.thresholds.context_switch_warning_rate {
                issues.push(DiagnosticEntry {
                    timestamp: metrics.uptime_ms,
                    level: DiagnosticLevel::Warning,
                    category: DiagnosticCategory::Performance,
                    message: format!("High context switch rate: {:.1}/sec", switch_rate),
                    details: Some("May indicate task thrashing".to_string()),
                    count: 1,
                });
            }
        }
        
        issues
    }
    
    fn check_performance_health(&mut self, metrics: &SystemMetrics) -> Vec<DiagnosticEntry> {
        let mut issues = Vec::new();
        
        // Check CPU utilization
        if metrics.cpu_utilization_percent > 95.0 {
            issues.push(DiagnosticEntry {
                timestamp: metrics.uptime_ms,
                level: DiagnosticLevel::Warning,
                category: DiagnosticCategory::Performance,
                message: format!("High CPU utilization: {:.1}%", metrics.cpu_utilization_percent),
                details: Some("System may be overloaded".to_string()),
                count: 1,
            });
        }
        
        // Check interrupt rate
        let uptime_sec = metrics.uptime_ms as f32 / 1000.0;
        if uptime_sec > 0.0 {
            let interrupt_rate = metrics.interrupts_handled as f32 / uptime_sec;
            if interrupt_rate > 10000.0 {
                issues.push(DiagnosticEntry {
                    timestamp: metrics.uptime_ms,
                    level: DiagnosticLevel::Info,
                    category: DiagnosticCategory::Performance,
                    message: format!("High interrupt rate: {:.1}/sec", interrupt_rate),
                    details: Some("Hardware may be very active".to_string()),
                    count: 1,
                });
            }
        }
        
        issues
    }
    
    fn check_system_stability(&mut self, metrics: &SystemMetrics) -> Vec<DiagnosticEntry> {
        let mut issues = Vec::new();
        
        // Check system uptime milestones
        let uptime_hours = metrics.uptime_ms as f32 / (1000.0 * 60.0 * 60.0);
        if uptime_hours >= 24.0 && (uptime_hours as u32 % 24) == 0 {
            issues.push(DiagnosticEntry {
                timestamp: metrics.uptime_ms,
                level: DiagnosticLevel::Info,
                category: DiagnosticCategory::System,
                message: format!("System uptime milestone: {:.0} hours", uptime_hours),
                details: Some("System stability confirmed".to_string()),
                count: 1,
            });
        }
        
        issues
    }
    
    /// Get recent diagnostics
    pub fn get_recent_diagnostics(&self, limit: usize) -> Vec<DiagnosticEntry> {
        self.diagnostics.iter()
            .rev()
            .take(limit)
            .cloned()
            .collect()
    }
    
    /// Get diagnostics by category
    pub fn get_diagnostics_by_category(&self, category: DiagnosticCategory) -> Vec<DiagnosticEntry> {
        self.diagnostics.iter()
            .filter(|d| d.category == category)
            .cloned()
            .collect()
    }
    
    /// Clear old diagnostics
    pub fn clear_old_diagnostics(&mut self, max_age_ms: u64) {
        let now = TIME_MS.load(Ordering::Relaxed);
        self.diagnostics.retain(|d| now - d.timestamp <= max_age_ms);
    }
}

/// System health status
#[derive(Debug, Clone, PartialEq)]
pub enum SystemHealthStatus {
    Healthy,
    Warning,
    Degraded,
    Critical,
    Unknown,
}

/// Comprehensive system health report
#[derive(Debug, Clone)]
pub struct SystemHealthReport {
    pub status: SystemHealthStatus,
    pub timestamp: u64,
    pub metrics: SystemMetrics,
    pub issues: Vec<DiagnosticEntry>,
    pub uptime_ms: u64,
}

impl SystemHealthReport {
    /// Generate a human-readable summary
    pub fn summary(&self) -> String {
        let mut summary = String::new();
        
        writeln!(&mut summary, "=== System Health Report ===").ok();
        writeln!(&mut summary, "Status: {:?}", self.status).ok();
        writeln!(&mut summary, "Uptime: {:.2} hours", self.uptime_ms as f32 / (1000.0 * 60.0 * 60.0)).ok();
        writeln!(&mut summary, "Memory Usage: {:.1}%", self.metrics.memory.utilization_percent()).ok();
        writeln!(&mut summary, "CPU Usage: {:.1}%", self.metrics.cpu_utilization_percent).ok();
        writeln!(&mut summary, "Active Tasks: {}", self.metrics.tasks.total_tasks_spawned - self.metrics.tasks.tasks_completed).ok();
        
        if !self.issues.is_empty() {
            writeln!(&mut summary, "\nIssues Found:").ok();
            for issue in &self.issues {
                writeln!(&mut summary, "  [{:?}] {}", issue.level, issue.message).ok();
                if issue.count > 1 {
                    writeln!(&mut summary, "    (occurred {} times)", issue.count).ok();
                }
            }
        } else {
            writeln!(&mut summary, "\nNo issues detected.").ok();
        }
        
        summary
    }
}

/// Public API functions
pub fn add_diagnostic(level: DiagnosticLevel, category: DiagnosticCategory, message: String, details: Option<String>) {
    let mut checker = HEALTH_CHECKER.lock();
    checker.add_diagnostic(level, category, message, details);
}

pub fn perform_health_check() -> SystemHealthReport {
    let mut checker = HEALTH_CHECKER.lock();
    checker.perform_health_check()
}

pub fn get_recent_diagnostics(limit: usize) -> Vec<DiagnosticEntry> {
    let checker = HEALTH_CHECKER.lock();
    checker.get_recent_diagnostics(limit)
}

pub fn get_diagnostics_by_category(category: DiagnosticCategory) -> Vec<DiagnosticEntry> {
    let checker = HEALTH_CHECKER.lock();
    checker.get_diagnostics_by_category(category)
}

pub fn clear_old_diagnostics(max_age_ms: u64) {
    let mut checker = HEALTH_CHECKER.lock();
    checker.clear_old_diagnostics(max_age_ms);
}

/// Initialize diagnostics system
pub fn init_diagnostics() {
    log::info!("Initializing system diagnostics...");
    add_diagnostic(
        DiagnosticLevel::Info,
        DiagnosticCategory::System,
        "Diagnostics system initialized".to_string(),
        Some("Real-time health monitoring enabled".to_string())
    );
}

/// Periodic diagnostic check (should be called regularly)
pub fn periodic_diagnostic_check() {
    let report = perform_health_check();
    
    match report.status {
        SystemHealthStatus::Critical => {
            log::error!("CRITICAL: System health is critical!");
            log::error!("{}", report.summary());
        }
        SystemHealthStatus::Degraded => {
            log::warn!("WARNING: System health is degraded");
            log::warn!("{}", report.summary());
        }
        SystemHealthStatus::Warning => {
            log::info!("System health warning detected");
        }
        _ => {
            log::trace!("System health check completed - status: {:?}", report.status);
        }
    }
    
    // Clean up old diagnostics (keep last 24 hours)
    clear_old_diagnostics(24 * 60 * 60 * 1000);
}
