/// Security framework for Agave OS
/// Provides access control, sandboxing, and security monitoring
use crate::sys::{
    diagnostics::{add_diagnostic, DiagnosticCategory, DiagnosticLevel},
    error::{AgaveError, AgaveResult},
    process::ProcessId,
};
use alloc::{
    collections::{BTreeMap, BTreeSet},
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    hash::{Hash, Hasher},
    sync::atomic::{AtomicU64, Ordering},
};
use spin::Mutex;

/// Security context for processes and operations
#[derive(Debug, Clone, PartialEq)]
pub struct SecurityContext {
    pub user_id: UserId,
    pub group_id: GroupId,
    pub capabilities: BTreeSet<Capability>,
    pub security_level: SecurityLevel,
    pub sandbox_profile: SandboxProfile,
}

/// User identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserId(u32);

impl UserId {
    pub const ROOT: UserId = UserId(0);
    pub const SYSTEM: UserId = UserId(1);
    pub const GUEST: UserId = UserId(1000);

    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// Group identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GroupId(u32);

impl GroupId {
    pub const ROOT: GroupId = GroupId(0);
    pub const SYSTEM: GroupId = GroupId(1);
    pub const USERS: GroupId = GroupId(100);

    pub fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(&self) -> u32 {
        self.0
    }
}

/// System capabilities that can be granted to processes
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Capability {
    // File system capabilities
    FileRead,
    FileWrite,
    FileExecute,
    FileCreate,
    FileDelete,
    DirectoryCreate,
    DirectoryDelete,

    // Network capabilities
    NetworkConnect,
    NetworkListen,
    NetworkRaw,

    // System capabilities
    ProcessCreate,
    ProcessKill,
    ProcessDebug,
    SystemTime,
    SystemShutdown,
    SystemReboot,

    // Memory capabilities
    MemoryMap,
    MemoryProtect,
    MemoryAllocate,

    // Device capabilities
    DeviceAccess,
    DeviceControl,

    // IPC capabilities
    IpcSend,
    IpcReceive,
    IpcBroadcast,

    // Security capabilities
    SecurityAudit,
    SecurityConfig,
    UserManagement,
}

/// Security levels for different trust contexts
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum SecurityLevel {
    System,     // Kernel and system processes
    Trusted,    // Trusted applications
    Standard,   // Normal applications
    Restricted, // Limited applications
    Sandboxed,  // Heavily sandboxed applications
}

/// Sandbox profiles define restrictions for processes
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxProfile {
    pub name: String,
    pub allowed_paths: BTreeSet<String>,
    pub denied_paths: BTreeSet<String>,
    pub network_restrictions: NetworkRestrictions,
    pub resource_limits: SandboxResourceLimits,
    pub allowed_syscalls: BTreeSet<String>,
}

/// Network restrictions for sandboxed processes
#[derive(Debug, Clone, PartialEq)]
pub struct NetworkRestrictions {
    pub allow_loopback: bool,
    pub allow_outbound: bool,
    pub allow_inbound: bool,
    pub allowed_ports: BTreeSet<u16>,
    pub blocked_hosts: BTreeSet<String>,
}

/// Resource limits for sandboxed processes
#[derive(Debug, Clone, PartialEq)]
pub struct SandboxResourceLimits {
    pub max_memory: usize,
    pub max_cpu_percent: u8,
    pub max_file_handles: u32,
    pub max_network_connections: u32,
    pub max_execution_time_ms: u64,
}

impl Default for SandboxProfile {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            allowed_paths: ["/tmp".to_string(), "/home".to_string()]
                .into_iter()
                .collect(),
            denied_paths: ["/etc".to_string(), "/sys".to_string()]
                .into_iter()
                .collect(),
            network_restrictions: NetworkRestrictions {
                allow_loopback: true,
                allow_outbound: false,
                allow_inbound: false,
                allowed_ports: BTreeSet::new(),
                blocked_hosts: BTreeSet::new(),
            },
            resource_limits: SandboxResourceLimits {
                max_memory: 64 * 1024 * 1024, // 64MB
                max_cpu_percent: 50,
                max_file_handles: 20,
                max_network_connections: 5,
                max_execution_time_ms: 30000, // 30 seconds
            },
            allowed_syscalls: ["read", "write", "open", "close"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// Security event types for monitoring
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    AccessDenied {
        context: SecurityContext,
        resource: String,
        capability: Capability,
    },
    SandboxViolation {
        process_id: ProcessId,
        violation_type: String,
        details: String,
    },
    PrivilegeEscalation {
        process_id: ProcessId,
        from_level: SecurityLevel,
        to_level: SecurityLevel,
    },
    SuspiciousActivity {
        process_id: ProcessId,
        activity: String,
        risk_level: RiskLevel,
    },
    AuthenticationFailure {
        user_id: Option<UserId>,
        reason: String,
    },
    ResourceExhaustion {
        process_id: ProcessId,
        resource_type: String,
        limit: u64,
        requested: u64,
    },
}

/// Risk levels for security events
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// Security monitor tracks and responds to security events
pub struct SecurityMonitor {
    events: Vec<(u64, SecurityEvent)>, // (timestamp, event)
    max_events: usize,
    risk_counters: BTreeMap<ProcessId, u32>,
    blocked_processes: BTreeSet<ProcessId>,
    security_policies: Vec<SecurityPolicy>,
}

/// Security policy defines automated responses to security events
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    pub name: String,
    pub trigger: SecurityEventTrigger,
    pub action: SecurityAction,
    pub threshold: u32,
    pub time_window_ms: u64,
}

/// Event patterns that trigger security policies
#[derive(Debug, Clone)]
pub enum SecurityEventTrigger {
    AccessDeniedCount,
    SandboxViolationCount,
    HighRiskActivity,
    ResourceExhaustionCount,
}

/// Actions taken when security policies are triggered
#[derive(Debug, Clone)]
pub enum SecurityAction {
    LogOnly,
    TerminateProcess,
    BlockProcess,
    RestrictCapabilities(Vec<Capability>),
    NotifyAdmin,
    SystemAlert,
}

impl SecurityMonitor {
    fn new() -> Self {
        let mut monitor = Self {
            events: Vec::new(),
            max_events: 10000,
            risk_counters: BTreeMap::new(),
            blocked_processes: BTreeSet::new(),
            security_policies: Vec::new(),
        };

        // Add default security policies
        monitor.add_default_policies();
        monitor
    }

    fn add_default_policies(&mut self) {
        // Policy: Block processes with too many access denials
        self.security_policies.push(SecurityPolicy {
            name: "Access Denial Protection".to_string(),
            trigger: SecurityEventTrigger::AccessDeniedCount,
            action: SecurityAction::BlockProcess,
            threshold: 10,
            time_window_ms: 60000, // 1 minute
        });

        // Policy: Terminate processes with repeated sandbox violations
        self.security_policies.push(SecurityPolicy {
            name: "Sandbox Violation Protection".to_string(),
            trigger: SecurityEventTrigger::SandboxViolationCount,
            action: SecurityAction::TerminateProcess,
            threshold: 5,
            time_window_ms: 30000, // 30 seconds
        });

        // Policy: Alert on high-risk activities
        self.security_policies.push(SecurityPolicy {
            name: "High Risk Activity Alert".to_string(),
            trigger: SecurityEventTrigger::HighRiskActivity,
            action: SecurityAction::SystemAlert,
            threshold: 1,
            time_window_ms: 0, // Immediate
        });
    }

    /// Record a security event
    pub fn record_event(&mut self, event: SecurityEvent) {
        let timestamp = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);

        // Add to event log
        self.events.push((timestamp, event.clone()));

        // Trim old events
        if self.events.len() > self.max_events {
            self.events.remove(0);
        }

        // Update risk counters and check policies
        self.update_risk_assessment(&event);
        self.check_security_policies(timestamp, &event);

        // Log security event
        match &event {
            SecurityEvent::AccessDenied {
                context,
                resource,
                capability,
            } => {
                add_diagnostic(
                    DiagnosticLevel::Warning,
                    DiagnosticCategory::Security,
                    format!("Access denied: {:?} for {}", capability, resource),
                    Some(format!(
                        "User: {}, Security Level: {:?}",
                        context.user_id.as_u32(),
                        context.security_level
                    )),
                );
            }
            SecurityEvent::SandboxViolation {
                process_id,
                violation_type,
                details,
            } => {
                add_diagnostic(
                    DiagnosticLevel::Error,
                    DiagnosticCategory::Security,
                    format!(
                        "Sandbox violation: {} (PID: {})",
                        violation_type,
                        process_id.as_u64()
                    ),
                    Some(details.clone()),
                );
            }
            SecurityEvent::SuspiciousActivity {
                process_id,
                activity,
                risk_level,
            } => {
                let diag_level = match risk_level {
                    RiskLevel::Critical => DiagnosticLevel::Critical,
                    RiskLevel::High => DiagnosticLevel::Error,
                    RiskLevel::Medium => DiagnosticLevel::Warning,
                    RiskLevel::Low => DiagnosticLevel::Info,
                };
                add_diagnostic(
                    diag_level,
                    DiagnosticCategory::Security,
                    format!(
                        "Suspicious activity: {} (PID: {})",
                        activity,
                        process_id.as_u64()
                    ),
                    Some(format!("Risk Level: {:?}", risk_level)),
                );
            }
            _ => {}
        }
    }

    fn update_risk_assessment(&mut self, event: &SecurityEvent) {
        let process_id = match event {
            SecurityEvent::SandboxViolation { process_id, .. }
            | SecurityEvent::SuspiciousActivity { process_id, .. }
            | SecurityEvent::ResourceExhaustion { process_id, .. } => *process_id,
            SecurityEvent::PrivilegeEscalation { process_id, .. } => *process_id,
            _ => return,
        };

        let counter = self.risk_counters.entry(process_id).or_insert(0);
        *counter += 1;

        // Auto-block processes with high risk scores
        if *counter >= 20 {
            self.blocked_processes.insert(process_id);
            log::warn!(
                "Process {} auto-blocked due to high risk score",
                process_id.as_u64()
            );
        }
    }

    fn check_security_policies(&mut self, timestamp: u64, event: &SecurityEvent) {
        let policies: Vec<_> = self.security_policies.iter().cloned().collect();
        for policy in policies {
            if self.policy_matches(&policy, event) {
                let count =
                    self.count_matching_events(&policy.trigger, timestamp, policy.time_window_ms);

                if count >= policy.threshold {
                    self.execute_security_action(&policy.action, event);
                    log::warn!(
                        "Security policy '{}' triggered (count: {})",
                        policy.name,
                        count
                    );
                }
            }
        }
    }

    fn policy_matches(&self, policy: &SecurityPolicy, event: &SecurityEvent) -> bool {
        match (&policy.trigger, event) {
            (SecurityEventTrigger::AccessDeniedCount, SecurityEvent::AccessDenied { .. }) => true,
            (
                SecurityEventTrigger::SandboxViolationCount,
                SecurityEvent::SandboxViolation { .. },
            ) => true,
            (
                SecurityEventTrigger::HighRiskActivity,
                SecurityEvent::SuspiciousActivity { risk_level, .. },
            ) => {
                matches!(risk_level, RiskLevel::High | RiskLevel::Critical)
            }
            (
                SecurityEventTrigger::ResourceExhaustionCount,
                SecurityEvent::ResourceExhaustion { .. },
            ) => true,
            _ => false,
        }
    }

    fn count_matching_events(
        &self,
        trigger: &SecurityEventTrigger,
        timestamp: u64,
        window_ms: u64,
    ) -> u32 {
        let cutoff_time = timestamp.saturating_sub(window_ms);

        self.events
            .iter()
            .filter(|(event_time, event)| {
                *event_time >= cutoff_time && self.trigger_matches(trigger, event)
            })
            .count() as u32
    }

    fn trigger_matches(&self, trigger: &SecurityEventTrigger, event: &SecurityEvent) -> bool {
        match (trigger, event) {
            (SecurityEventTrigger::AccessDeniedCount, SecurityEvent::AccessDenied { .. }) => true,
            (
                SecurityEventTrigger::SandboxViolationCount,
                SecurityEvent::SandboxViolation { .. },
            ) => true,
            (
                SecurityEventTrigger::HighRiskActivity,
                SecurityEvent::SuspiciousActivity { risk_level, .. },
            ) => {
                matches!(risk_level, RiskLevel::High | RiskLevel::Critical)
            }
            (
                SecurityEventTrigger::ResourceExhaustionCount,
                SecurityEvent::ResourceExhaustion { .. },
            ) => true,
            _ => false,
        }
    }

    fn execute_security_action(&mut self, action: &SecurityAction, event: &SecurityEvent) {
        match action {
            SecurityAction::LogOnly => {
                log::info!("Security action: Log only for event: {:?}", event);
            }
            SecurityAction::TerminateProcess => {
                if let Some(process_id) = self.extract_process_id(event) {
                    log::warn!(
                        "Security action: Terminating process {}",
                        process_id.as_u64()
                    );
                    // TODO: Integrate with process manager to terminate
                }
            }
            SecurityAction::BlockProcess => {
                if let Some(process_id) = self.extract_process_id(event) {
                    self.blocked_processes.insert(process_id);
                    log::warn!("Security action: Blocking process {}", process_id.as_u64());
                }
            }
            SecurityAction::RestrictCapabilities(caps) => {
                log::warn!("Security action: Restricting capabilities: {:?}", caps);
                // TODO: Implement capability restriction
            }
            SecurityAction::NotifyAdmin => {
                log::error!("Security action: Admin notification for event: {:?}", event);
            }
            SecurityAction::SystemAlert => {
                log::error!("SECURITY ALERT: {:?}", event);
                add_diagnostic(
                    DiagnosticLevel::Critical,
                    DiagnosticCategory::Security,
                    "System security alert triggered".to_string(),
                    Some(format!("{:?}", event)),
                );
            }
        }
    }

    fn extract_process_id(&self, event: &SecurityEvent) -> Option<ProcessId> {
        match event {
            SecurityEvent::SandboxViolation { process_id, .. }
            | SecurityEvent::SuspiciousActivity { process_id, .. }
            | SecurityEvent::ResourceExhaustion { process_id, .. } => Some(*process_id),
            SecurityEvent::PrivilegeEscalation { process_id, .. } => Some(*process_id),
            _ => None,
        }
    }

    /// Check if a process is blocked
    pub fn is_process_blocked(&self, process_id: ProcessId) -> bool {
        self.blocked_processes.contains(&process_id)
    }

    /// Get recent security events
    pub fn get_recent_events(&self, limit: usize) -> Vec<(u64, &SecurityEvent)> {
        self.events
            .iter()
            .rev()
            .take(limit)
            .map(|(timestamp, event)| (*timestamp, event))
            .collect()
    }

    /// Get security statistics
    pub fn get_statistics(&self) -> SecurityStatistics {
        let mut event_counts = BTreeMap::new();
        let mut risk_distribution = BTreeMap::new();

        for (_, event) in &self.events {
            let event_type = match event {
                SecurityEvent::AccessDenied { .. } => "Access Denied",
                SecurityEvent::SandboxViolation { .. } => "Sandbox Violation",
                SecurityEvent::PrivilegeEscalation { .. } => "Privilege Escalation",
                SecurityEvent::SuspiciousActivity { .. } => "Suspicious Activity",
                SecurityEvent::AuthenticationFailure { .. } => "Authentication Failure",
                SecurityEvent::ResourceExhaustion { .. } => "Resource Exhaustion",
            };
            *event_counts.entry(event_type.to_string()).or_insert(0) += 1;

            if let SecurityEvent::SuspiciousActivity { risk_level, .. } = event {
                *risk_distribution
                    .entry(format!("{:?}", risk_level))
                    .or_insert(0) += 1;
            }
        }

        SecurityStatistics {
            total_events: self.events.len(),
            event_counts,
            risk_distribution,
            blocked_processes: self.blocked_processes.len(),
            active_policies: self.security_policies.len(),
        }
    }
}

/// Security statistics
#[derive(Debug, Clone)]
pub struct SecurityStatistics {
    pub total_events: usize,
    pub event_counts: BTreeMap<String, u32>,
    pub risk_distribution: BTreeMap<String, u32>,
    pub blocked_processes: usize,
    pub active_policies: usize,
}

/// Global security monitor
static SECURITY_MONITOR: Mutex<SecurityMonitor> = Mutex::new(SecurityMonitor {
    events: Vec::new(),
    max_events: 10000,
    risk_counters: BTreeMap::new(),
    blocked_processes: BTreeSet::new(),
    security_policies: Vec::new(),
});

/// Access control manager
pub struct AccessControlManager {
    contexts: BTreeMap<ProcessId, SecurityContext>,
    sandbox_profiles: BTreeMap<String, SandboxProfile>,
}

impl AccessControlManager {
    fn new() -> Self {
        let mut manager = Self {
            contexts: BTreeMap::new(),
            sandbox_profiles: BTreeMap::new(),
        };

        // Add default sandbox profiles
        manager.add_default_profiles();
        manager
    }

    fn add_default_profiles(&mut self) {
        // Restrictive profile for untrusted applications
        let restrictive = SandboxProfile {
            name: "restrictive".to_string(),
            allowed_paths: ["/tmp".to_string()].into_iter().collect(),
            denied_paths: ["/etc".to_string(), "/sys".to_string(), "/proc".to_string()]
                .into_iter()
                .collect(),
            network_restrictions: NetworkRestrictions {
                allow_loopback: true,
                allow_outbound: false,
                allow_inbound: false,
                allowed_ports: BTreeSet::new(),
                blocked_hosts: BTreeSet::new(),
            },
            resource_limits: SandboxResourceLimits {
                max_memory: 32 * 1024 * 1024, // 32MB
                max_cpu_percent: 25,
                max_file_handles: 10,
                max_network_connections: 0,
                max_execution_time_ms: 15000,
            },
            allowed_syscalls: ["read", "write"].iter().map(|s| s.to_string()).collect(),
        };

        // Standard profile for normal applications
        let standard = SandboxProfile::default();

        // Trusted profile for system applications
        let trusted = SandboxProfile {
            name: "trusted".to_string(),
            allowed_paths: ["/".to_string()].into_iter().collect(),
            denied_paths: BTreeSet::new(),
            network_restrictions: NetworkRestrictions {
                allow_loopback: true,
                allow_outbound: true,
                allow_inbound: true,
                allowed_ports: (1..65535).collect(),
                blocked_hosts: BTreeSet::new(),
            },
            resource_limits: SandboxResourceLimits {
                max_memory: 256 * 1024 * 1024, // 256MB
                max_cpu_percent: 90,
                max_file_handles: 1000,
                max_network_connections: 100,
                max_execution_time_ms: 0, // No limit
            },
            allowed_syscalls: BTreeSet::new(), // All syscalls allowed
        };

        self.sandbox_profiles
            .insert("restrictive".to_string(), restrictive);
        self.sandbox_profiles
            .insert("default".to_string(), standard);
        self.sandbox_profiles.insert("trusted".to_string(), trusted);
    }

    /// Set security context for a process
    pub fn set_context(&mut self, process_id: ProcessId, context: SecurityContext) {
        self.contexts.insert(process_id, context);
    }

    /// Get security context for a process
    pub fn get_context(&self, process_id: ProcessId) -> Option<&SecurityContext> {
        self.contexts.get(&process_id)
    }

    /// Check if a process has a specific capability
    pub fn check_capability(&self, process_id: ProcessId, capability: Capability) -> bool {
        if let Some(context) = self.contexts.get(&process_id) {
            context.capabilities.contains(&capability)
        } else {
            false
        }
    }

    /// Check access to a resource
    pub fn check_access(
        &self,
        process_id: ProcessId,
        resource: &str,
        capability: Capability,
    ) -> AgaveResult<()> {
        let context = self
            .contexts
            .get(&process_id)
            .ok_or(AgaveError::SecurityViolation)?;

        // Check if process is blocked
        {
            let monitor = SECURITY_MONITOR.lock();
            if monitor.is_process_blocked(process_id) {
                return Err(AgaveError::SecurityViolation);
            }
        }

        // Check capability
        if !context.capabilities.contains(&capability) {
            let mut monitor = SECURITY_MONITOR.lock();
            monitor.record_event(SecurityEvent::AccessDenied {
                context: context.clone(),
                resource: resource.to_string(),
                capability,
            });
            return Err(AgaveError::PermissionDenied);
        }

        // Check sandbox restrictions
        if let Some(profile) = self.sandbox_profiles.get(&context.sandbox_profile.name) {
            if !self.check_sandbox_access(profile, resource) {
                let mut monitor = SECURITY_MONITOR.lock();
                monitor.record_event(SecurityEvent::SandboxViolation {
                    process_id,
                    violation_type: "Path access denied".to_string(),
                    details: format!("Attempted to access: {}", resource),
                });
                return Err(AgaveError::SecurityViolation);
            }
        }

        Ok(())
    }

    fn check_sandbox_access(&self, profile: &SandboxProfile, resource: &str) -> bool {
        // Check if path is explicitly denied
        for denied_path in &profile.denied_paths {
            if resource.starts_with(denied_path) {
                return false;
            }
        }

        // Check if path is explicitly allowed
        for allowed_path in &profile.allowed_paths {
            if resource.starts_with(allowed_path) {
                return true;
            }
        }

        // Default deny if no explicit allow
        false
    }
}

static ACCESS_CONTROL: Mutex<AccessControlManager> = Mutex::new(AccessControlManager {
    contexts: BTreeMap::new(),
    sandbox_profiles: BTreeMap::new(),
});

/// Public API functions
pub fn set_process_security_context(process_id: ProcessId, context: SecurityContext) {
    let mut acm = ACCESS_CONTROL.lock();
    acm.set_context(process_id, context);
}

pub fn get_process_security_context(process_id: ProcessId) -> Option<SecurityContext> {
    let acm = ACCESS_CONTROL.lock();
    acm.get_context(process_id).cloned()
}

pub fn check_process_capability(process_id: ProcessId, capability: Capability) -> bool {
    let acm = ACCESS_CONTROL.lock();
    acm.check_capability(process_id, capability)
}

pub fn check_access(
    process_id: ProcessId,
    resource: &str,
    capability: Capability,
) -> AgaveResult<()> {
    let acm = ACCESS_CONTROL.lock();
    acm.check_access(process_id, resource, capability)
}

pub fn record_security_event(event: SecurityEvent) {
    let mut monitor = SECURITY_MONITOR.lock();
    monitor.record_event(event);
}

pub fn is_process_blocked(process_id: ProcessId) -> bool {
    let monitor = SECURITY_MONITOR.lock();
    monitor.is_process_blocked(process_id)
}

pub fn get_recent_security_events(limit: usize) -> Vec<(u64, SecurityEvent)> {
    let monitor = SECURITY_MONITOR.lock();
    monitor
        .get_recent_events(limit)
        .into_iter()
        .map(|(t, e)| (t, e.clone()))
        .collect()
}

pub fn get_security_statistics() -> SecurityStatistics {
    let monitor = SECURITY_MONITOR.lock();
    monitor.get_statistics()
}

/// Create a default security context for a process
pub fn create_default_context(user_id: UserId, security_level: SecurityLevel) -> SecurityContext {
    let capabilities = match security_level {
        SecurityLevel::System => {
            // System processes get all capabilities
            [
                Capability::FileRead,
                Capability::FileWrite,
                Capability::FileExecute,
                Capability::FileCreate,
                Capability::FileDelete,
                Capability::DirectoryCreate,
                Capability::DirectoryDelete,
                Capability::NetworkConnect,
                Capability::NetworkListen,
                Capability::ProcessCreate,
                Capability::ProcessKill,
                Capability::SystemTime,
                Capability::MemoryMap,
                Capability::MemoryProtect,
                Capability::DeviceAccess,
                Capability::IpcSend,
                Capability::IpcReceive,
                Capability::SecurityAudit,
            ]
            .into_iter()
            .collect()
        }
        SecurityLevel::Trusted => {
            // Trusted processes get most capabilities
            [
                Capability::FileRead,
                Capability::FileWrite,
                Capability::FileExecute,
                Capability::FileCreate,
                Capability::NetworkConnect,
                Capability::ProcessCreate,
                Capability::MemoryAllocate,
                Capability::IpcSend,
                Capability::IpcReceive,
            ]
            .into_iter()
            .collect()
        }
        SecurityLevel::Standard => {
            // Standard processes get basic capabilities
            [
                Capability::FileRead,
                Capability::FileWrite,
                Capability::NetworkConnect,
                Capability::MemoryAllocate,
                Capability::IpcSend,
                Capability::IpcReceive,
            ]
            .into_iter()
            .collect()
        }
        SecurityLevel::Restricted => {
            // Restricted processes get minimal capabilities
            [Capability::FileRead, Capability::MemoryAllocate]
                .into_iter()
                .collect()
        }
        SecurityLevel::Sandboxed => {
            // Sandboxed processes get very limited capabilities
            [Capability::MemoryAllocate].into_iter().collect()
        }
    };

    let sandbox_profile_name = match security_level {
        SecurityLevel::System | SecurityLevel::Trusted => "trusted".to_string(),
        SecurityLevel::Standard => "default".to_string(),
        SecurityLevel::Restricted | SecurityLevel::Sandboxed => "restrictive".to_string(),
    };

    SecurityContext {
        user_id,
        group_id: GroupId::USERS,
        capabilities,
        security_level,
        sandbox_profile: SandboxProfile {
            name: sandbox_profile_name,
            ..SandboxProfile::default()
        },
    }
}

/// Initialize security framework
pub fn init_security() {
    log::info!("Security framework initialized");

    // Initialize access control with default profiles
    {
        let mut acm = ACCESS_CONTROL.lock();
        acm.add_default_profiles();
    }

    // Initialize security monitor with default policies
    {
        let mut monitor = SECURITY_MONITOR.lock();
        monitor.add_default_policies();
    }

    add_diagnostic(
        DiagnosticLevel::Info,
        DiagnosticCategory::Security,
        "Security framework initialized".to_string(),
        Some("Access control, sandboxing, and monitoring enabled".to_string()),
    );
}
