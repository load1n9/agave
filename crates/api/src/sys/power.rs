/// Power management system for Agave OS
/// Provides CPU frequency scaling, sleep states, and energy optimization
use crate::sys::{
    error::{AgaveError, AgaveResult},
    diagnostics::{add_diagnostic, DiagnosticLevel, DiagnosticCategory},
    monitor::get_system_metrics,
};
use alloc::{vec, vec::Vec, string::{String, ToString}, collections::BTreeMap, format};
use core::{
    sync::atomic::{AtomicU32, AtomicU64, AtomicBool, Ordering},
    cmp::{min, max},
};
use spin::Mutex;

/// CPU power states
#[derive(Debug, Clone, PartialEq)]
pub enum PowerState {
    Active,      // Full performance
    Reduced,     // Reduced frequency
    Sleep,       // CPU sleep mode
    DeepSleep,   // Deep sleep mode
    Hibernate,   // Save state to disk
}

/// Power management policy
#[derive(Debug, Clone, PartialEq)]
pub enum PowerPolicy {
    Performance,    // Maximum performance, ignore power usage
    Balanced,       // Balance performance and power
    PowerSaver,     // Minimize power usage
    Adaptive,       // Automatically adjust based on workload
    Custom(PowerConfig), // User-defined configuration
}

/// Custom power configuration
#[derive(Debug, Clone, PartialEq)]
pub struct PowerConfig {
    pub cpu_min_freq_mhz: u32,
    pub cpu_max_freq_mhz: u32,
    pub sleep_timeout_ms: u64,
    pub deep_sleep_timeout_ms: u64,
    pub cpu_utilization_threshold: u8,
    pub aggressive_scaling: bool,
}

impl Default for PowerConfig {
    fn default() -> Self {
        Self {
            cpu_min_freq_mhz: 800,
            cpu_max_freq_mhz: 2400,
            sleep_timeout_ms: 5000,
            deep_sleep_timeout_ms: 30000,
            cpu_utilization_threshold: 50,
            aggressive_scaling: false,
        }
    }
}

/// CPU frequency scaling
#[derive(Debug, Clone)]
pub struct CpuFrequencyScaler {
    current_freq_mhz: u32,
    min_freq_mhz: u32,
    max_freq_mhz: u32,
    available_frequencies: Vec<u32>,
    scaling_governor: ScalingGovernor,
}

/// CPU scaling governors
#[derive(Debug, Clone, PartialEq)]
pub enum ScalingGovernor {
    Performance,    // Always max frequency
    Powersave,      // Always min frequency  
    Ondemand,       // Scale based on load
    Conservative,   // Gradual scaling
    Userspace,      // User-controlled
}

impl CpuFrequencyScaler {
    fn new() -> Self {
        // Simulate common CPU frequencies
        let available_frequencies = vec![800, 1000, 1200, 1600, 2000, 2400];
        
        Self {
            current_freq_mhz: 2400,
            min_freq_mhz: 800,
            max_freq_mhz: 2400,
            available_frequencies,
            scaling_governor: ScalingGovernor::Ondemand,
        }
    }
    
    /// Scale CPU frequency based on utilization
    pub fn scale_frequency(&mut self, cpu_utilization: f32, policy: &PowerPolicy) -> AgaveResult<()> {
        let target_freq = match policy {
            PowerPolicy::Performance => self.max_freq_mhz,
            PowerPolicy::PowerSaver => self.min_freq_mhz,
            PowerPolicy::Balanced => self.calculate_balanced_frequency(cpu_utilization),
            PowerPolicy::Adaptive => self.calculate_adaptive_frequency(cpu_utilization),
            PowerPolicy::Custom(config) => self.calculate_custom_frequency(cpu_utilization, config),
        };
        
        self.set_frequency(target_freq)
    }
    
    fn calculate_balanced_frequency(&self, cpu_utilization: f32) -> u32 {
        let util_ratio = (cpu_utilization / 100.0).clamp(0.0, 1.0);
        let freq_range = self.max_freq_mhz - self.min_freq_mhz;
        let target = self.min_freq_mhz + (freq_range as f32 * util_ratio) as u32;
        
        // Find nearest available frequency
        self.find_nearest_frequency(target)
    }
    
    fn calculate_adaptive_frequency(&self, cpu_utilization: f32) -> u32 {
        // More aggressive scaling for adaptive mode
        if cpu_utilization < 10.0 {
            self.min_freq_mhz
        } else if cpu_utilization < 30.0 {
            self.min_freq_mhz + (self.max_freq_mhz - self.min_freq_mhz) / 4
        } else if cpu_utilization < 60.0 {
            self.min_freq_mhz + (self.max_freq_mhz - self.min_freq_mhz) / 2
        } else if cpu_utilization < 80.0 {
            self.min_freq_mhz + 3 * (self.max_freq_mhz - self.min_freq_mhz) / 4
        } else {
            self.max_freq_mhz
        }
    }
    
    fn calculate_custom_frequency(&self, cpu_utilization: f32, config: &PowerConfig) -> u32 {
        let threshold = config.cpu_utilization_threshold as f32;
        
        if config.aggressive_scaling {
            // More responsive to utilization changes
            let util_ratio = (cpu_utilization / 100.0).clamp(0.0, 1.0);
            let freq_range = config.cpu_max_freq_mhz - config.cpu_min_freq_mhz;
            config.cpu_min_freq_mhz + (freq_range as f32 * util_ratio * util_ratio) as u32
        } else {
            // Conservative scaling
            if cpu_utilization < threshold {
                config.cpu_min_freq_mhz
            } else {
                config.cpu_max_freq_mhz
            }
        }
    }
    
    fn find_nearest_frequency(&self, target: u32) -> u32 {
        self.available_frequencies
            .iter()
            .min_by_key(|&&freq| (freq as i32 - target as i32).abs())
            .copied()
            .unwrap_or(target)
    }
    
    fn set_frequency(&mut self, freq_mhz: u32) -> AgaveResult<()> {
        let clamped_freq = freq_mhz.clamp(self.min_freq_mhz, self.max_freq_mhz);
        let actual_freq = self.find_nearest_frequency(clamped_freq);
        
        if actual_freq != self.current_freq_mhz {
            log::debug!("CPU frequency scaling: {} MHz -> {} MHz", self.current_freq_mhz, actual_freq);
            self.current_freq_mhz = actual_freq;
            
            // TODO: Actually set CPU frequency via hardware interface
            // This would typically involve writing to MSRs or ACPI
        }
        
        Ok(())
    }
    
    pub fn get_current_frequency(&self) -> u32 {
        self.current_freq_mhz
    }
    
    pub fn get_available_frequencies(&self) -> &[u32] {
        &self.available_frequencies
    }
}

/// Power management statistics
#[derive(Debug, Clone, Default)]
pub struct PowerStatistics {
    pub time_in_states: BTreeMap<String, u64>, // Time spent in each power state (ms)
    pub frequency_changes: u64,
    pub sleep_events: u64,
    pub wake_events: u64,
    pub power_consumption_estimates: PowerConsumption,
    pub thermal_events: u64,
}

/// Estimated power consumption
#[derive(Debug, Clone, Default)]
pub struct PowerConsumption {
    pub cpu_watts: f32,
    pub total_watts: f32,
    pub battery_life_estimate_minutes: Option<u32>,
}

/// Thermal management
#[derive(Debug, Clone)]
pub struct ThermalManager {
    cpu_temperature_celsius: f32,
    thermal_zones: BTreeMap<String, ThermalZone>,
    cooling_policies: Vec<CoolingPolicy>,
    thermal_throttling_active: bool,
}

/// Thermal zone information
#[derive(Debug, Clone)]
pub struct ThermalZone {
    pub name: String,
    pub temperature_celsius: f32,
    pub critical_temp: f32,
    pub warning_temp: f32,
    pub cooling_devices: Vec<String>,
}

/// Cooling policy
#[derive(Debug, Clone)]
pub struct CoolingPolicy {
    pub name: String,
    pub trigger_temp: f32,
    pub target_temp: f32,
    pub actions: Vec<CoolingAction>,
}

/// Cooling actions
#[derive(Debug, Clone)]
pub enum CoolingAction {
    ReduceCpuFrequency(u32), // Reduce to specific MHz
    EnableFanControl,
    ThrottleProcesses,
    EmergencyShutdown,
}

impl ThermalManager {
    fn new() -> Self {
        let mut manager = Self {
            cpu_temperature_celsius: 45.0, // Simulated temperature
            thermal_zones: BTreeMap::new(),
            cooling_policies: Vec::new(),
            thermal_throttling_active: false,
        };
        
        // Add default thermal zones
        manager.thermal_zones.insert("cpu".to_string(), ThermalZone {
            name: "CPU".to_string(),
            temperature_celsius: 45.0,
            critical_temp: 85.0,
            warning_temp: 70.0,
            cooling_devices: vec!["cpu_fan".to_string(), "freq_scaling".to_string()],
        });
        
        // Add default cooling policies
        manager.cooling_policies.push(CoolingPolicy {
            name: "CPU Warning".to_string(),
            trigger_temp: 70.0,
            target_temp: 65.0,
            actions: vec![CoolingAction::ReduceCpuFrequency(1600)],
        });
        
        manager.cooling_policies.push(CoolingPolicy {
            name: "CPU Critical".to_string(),
            trigger_temp: 85.0,
            target_temp: 75.0,
            actions: vec![
                CoolingAction::ReduceCpuFrequency(800),
                CoolingAction::ThrottleProcesses,
            ],
        });
        
        manager.cooling_policies.push(CoolingPolicy {
            name: "Emergency Shutdown".to_string(),
            trigger_temp: 95.0,
            target_temp: 0.0, // Irrelevant for shutdown
            actions: vec![CoolingAction::EmergencyShutdown],
        });
        
        manager
    }
    
    /// Update thermal state and apply cooling if needed
    pub fn update_thermal_state(&mut self, freq_scaler: &mut CpuFrequencyScaler) -> AgaveResult<()> {
        // Simulate temperature based on CPU frequency and load
        let base_temp = 35.0;
        let freq_factor = freq_scaler.current_freq_mhz as f32 / 2400.0;
        
        // Get CPU utilization from system metrics
        let metrics = get_system_metrics();
        let load_factor = metrics.cpu_utilization_percent / 100.0;
        
        self.cpu_temperature_celsius = base_temp + (freq_factor * 30.0) + (load_factor * 20.0);
        
        // Update thermal zone
        if let Some(cpu_zone) = self.thermal_zones.get_mut("cpu") {
            cpu_zone.temperature_celsius = self.cpu_temperature_celsius;
        }
        
        // Check cooling policies
        let policies: Vec<_> = self.cooling_policies.iter().cloned().collect();
        for policy in policies {
            if self.cpu_temperature_celsius >= policy.trigger_temp {
                self.apply_cooling_policy(&policy, freq_scaler)?;
            }
        }
        
        Ok(())
    }
    
    fn apply_cooling_policy(&mut self, policy: &CoolingPolicy, freq_scaler: &mut CpuFrequencyScaler) -> AgaveResult<()> {
        log::warn!("Thermal policy '{}' activated - CPU temp: {:.1}°C", 
                   policy.name, self.cpu_temperature_celsius);
        
        for action in &policy.actions {
            match action {
                CoolingAction::ReduceCpuFrequency(target_mhz) => {
                    if freq_scaler.current_freq_mhz > *target_mhz {
                        freq_scaler.set_frequency(*target_mhz)?;
                        self.thermal_throttling_active = true;
                        
                        add_diagnostic(
                            DiagnosticLevel::Warning,
                            DiagnosticCategory::Hardware,
                            format!("Thermal throttling: CPU frequency reduced to {} MHz", target_mhz),
                            Some(format!("CPU temperature: {:.1}°C", self.cpu_temperature_celsius))
                        );
                    }
                }
                CoolingAction::EnableFanControl => {
                    log::info!("Fan control enabled due to thermal policy");
                    // TODO: Implement actual fan control
                }
                CoolingAction::ThrottleProcesses => {
                    log::warn!("Process throttling enabled due to high temperature");
                    // TODO: Implement process throttling
                }
                CoolingAction::EmergencyShutdown => {
                    add_diagnostic(
                        DiagnosticLevel::Critical,
                        DiagnosticCategory::Hardware,
                        "Emergency thermal shutdown triggered".to_string(),
                        Some(format!("CPU temperature: {:.1}°C", self.cpu_temperature_celsius))
                    );
                    log::error!("EMERGENCY: Thermal shutdown triggered at {:.1}°C", self.cpu_temperature_celsius);
                    // TODO: Implement emergency shutdown
                    return Err(AgaveError::HardwareError(crate::sys::error::HwError::DeviceNotResponding));
                }
            }
        }
        
        Ok(())
    }
    
    pub fn get_temperature(&self, zone: &str) -> Option<f32> {
        self.thermal_zones.get(zone).map(|zone| zone.temperature_celsius)
    }
    
    pub fn is_thermal_throttling(&self) -> bool {
        self.thermal_throttling_active
    }
}

/// Main power manager
pub struct PowerManager {
    current_state: PowerState,
    current_policy: PowerPolicy,
    freq_scaler: CpuFrequencyScaler,
    thermal_manager: ThermalManager,
    statistics: PowerStatistics,
    last_activity_time: AtomicU64,
    sleep_disabled: AtomicBool,
    state_change_time: u64,
}

impl PowerManager {
    fn new() -> Self {
        Self {
            current_state: PowerState::Active,
            current_policy: PowerPolicy::Balanced,
            freq_scaler: CpuFrequencyScaler::new(),
            thermal_manager: ThermalManager::new(),
            statistics: PowerStatistics::default(),
            last_activity_time: AtomicU64::new(0),
            sleep_disabled: AtomicBool::new(false),
            state_change_time: 0,
        }
    }
    
    /// Update power management state
    pub fn update(&mut self) -> AgaveResult<()> {
        let now = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
        let metrics = get_system_metrics();
        
        // Update thermal management
        self.thermal_manager.update_thermal_state(&mut self.freq_scaler)?;
        
        // Scale CPU frequency based on policy and thermal state
        if !self.thermal_manager.is_thermal_throttling() {
            self.freq_scaler.scale_frequency(metrics.cpu_utilization_percent, &self.current_policy)?;
        }
        
        // Check for power state transitions
        self.check_power_state_transitions(now, &metrics)?;
        
        // Update statistics
        self.update_statistics(now);
        
        // Periodic logging
        if now % 60000 == 0 { // Every minute
            self.log_power_status();
        }
        
        Ok(())
    }
    
    fn check_power_state_transitions(&mut self, now: u64, metrics: &crate::sys::monitor::SystemMetrics) -> AgaveResult<()> {
        let last_activity = self.last_activity_time.load(Ordering::Relaxed);
        let time_since_activity = now.saturating_sub(last_activity);
        
        let new_state = match (&self.current_policy, &self.current_state) {
            // Never sleep in performance mode
            (PowerPolicy::Performance, _) => PowerState::Active,
            
            // Check sleep conditions for other policies
            (_, PowerState::Active) => {
                if !self.sleep_disabled.load(Ordering::Relaxed) && 
                   metrics.cpu_utilization_percent < 5.0 &&
                   time_since_activity > 5000 { // 5 seconds of inactivity
                    PowerState::Sleep
                } else if metrics.cpu_utilization_percent < 30.0 {
                    PowerState::Reduced
                } else {
                    PowerState::Active
                }
            }
            
            (_, PowerState::Reduced) => {
                if metrics.cpu_utilization_percent > 50.0 {
                    PowerState::Active
                } else if !self.sleep_disabled.load(Ordering::Relaxed) && 
                         metrics.cpu_utilization_percent < 5.0 &&
                         time_since_activity > 5000 {
                    PowerState::Sleep
                } else {
                    PowerState::Reduced
                }
            }
            
            (_, PowerState::Sleep) => {
                if metrics.cpu_utilization_percent > 10.0 || 
                   time_since_activity < 1000 { // Recent activity
                    PowerState::Active
                } else if time_since_activity > 30000 { // 30 seconds in sleep
                    PowerState::DeepSleep
                } else {
                    PowerState::Sleep
                }
            }
            
            (_, PowerState::DeepSleep) => {
                if metrics.cpu_utilization_percent > 5.0 || 
                   time_since_activity < 1000 {
                    PowerState::Active
                } else {
                    PowerState::DeepSleep
                }
            }
            
            _ => self.current_state.clone(),
        };
        
        if new_state != self.current_state {
            self.transition_power_state(new_state, now)?;
        }
        
        Ok(())
    }
    
    fn transition_power_state(&mut self, new_state: PowerState, now: u64) -> AgaveResult<()> {
        let old_state = self.current_state.clone();
        
        log::info!("Power state transition: {:?} -> {:?}", old_state, new_state);
        
        // Update statistics for time in previous state
        let time_in_state = now.saturating_sub(self.state_change_time);
        let state_name = format!("{:?}", old_state);
        *self.statistics.time_in_states.entry(state_name).or_insert(0) += time_in_state;
        
        // Perform state-specific actions
        match &new_state {
            PowerState::Active => {
                // Restore full performance
                self.freq_scaler.set_frequency(self.freq_scaler.max_freq_mhz)?;
                self.statistics.wake_events += 1;
            }
            PowerState::Reduced => {
                // Reduce frequency for power saving
                let reduced_freq = self.freq_scaler.min_freq_mhz + 
                    (self.freq_scaler.max_freq_mhz - self.freq_scaler.min_freq_mhz) / 2;
                self.freq_scaler.set_frequency(reduced_freq)?;
            }
            PowerState::Sleep => {
                // Enter CPU sleep mode
                self.freq_scaler.set_frequency(self.freq_scaler.min_freq_mhz)?;
                self.statistics.sleep_events += 1;
                // TODO: Actually implement CPU sleep via HLT or similar
            }
            PowerState::DeepSleep => {
                // Enter deeper sleep mode
                self.freq_scaler.set_frequency(self.freq_scaler.min_freq_mhz)?;
                // TODO: Implement deeper sleep states
            }
            PowerState::Hibernate => {
                // Save system state and power down
                log::warn!("Hibernation not yet implemented");
                return Err(AgaveError::NotImplemented);
            }
        }
        
        self.current_state = new_state;
        self.state_change_time = now;
        
        add_diagnostic(
            DiagnosticLevel::Info,
            DiagnosticCategory::System,
            format!("Power state changed to {:?}", self.current_state),
            Some(format!("CPU frequency: {} MHz", self.freq_scaler.current_freq_mhz))
        );
        
        Ok(())
    }
    
    fn update_statistics(&mut self, now: u64) {
        // Update power consumption estimates
        let freq_ratio = self.freq_scaler.current_freq_mhz as f32 / self.freq_scaler.max_freq_mhz as f32;
        let base_power = 15.0; // Baseline power consumption in watts
        
        self.statistics.power_consumption_estimates.cpu_watts = match self.current_state {
            PowerState::Active => base_power * freq_ratio * freq_ratio, // Quadratic scaling
            PowerState::Reduced => base_power * freq_ratio * 0.7,
            PowerState::Sleep => base_power * 0.3,
            PowerState::DeepSleep => base_power * 0.1,
            PowerState::Hibernate => 0.5,
        };
        
        self.statistics.power_consumption_estimates.total_watts = 
            self.statistics.power_consumption_estimates.cpu_watts + 10.0; // Add system overhead
    }
    
    fn log_power_status(&self) {
        log::info!("Power Status - State: {:?}, CPU: {} MHz, Temp: {:.1}°C, Power: {:.1}W",
                   self.current_state,
                   self.freq_scaler.current_freq_mhz,
                   self.thermal_manager.cpu_temperature_celsius,
                   self.statistics.power_consumption_estimates.total_watts);
    }
    
    /// Set power management policy
    pub fn set_policy(&mut self, policy: PowerPolicy) -> AgaveResult<()> {
        log::info!("Power policy changed to: {:?}", policy);
        self.current_policy = policy;
        
        add_diagnostic(
            DiagnosticLevel::Info,
            DiagnosticCategory::System,
            format!("Power policy changed to {:?}", self.current_policy),
            None
        );
        
        Ok(())
    }
    
    /// Disable/enable sleep modes
    pub fn set_sleep_disabled(&self, disabled: bool) {
        self.sleep_disabled.store(disabled, Ordering::Relaxed);
        
        if disabled {
            log::info!("Sleep modes disabled");
        } else {
            log::info!("Sleep modes enabled");
        }
    }
    
    /// Record system activity to prevent sleep
    pub fn record_activity(&self) {
        let now = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
        self.last_activity_time.store(now, Ordering::Relaxed);
    }
    
    /// Get current power state
    pub fn get_state(&self) -> &PowerState {
        &self.current_state
    }
    
    /// Get current power policy
    pub fn get_policy(&self) -> &PowerPolicy {
        &self.current_policy
    }
    
    /// Get power statistics
    pub fn get_statistics(&self) -> &PowerStatistics {
        &self.statistics
    }
    
    /// Get CPU frequency information
    pub fn get_cpu_frequency_info(&self) -> (u32, u32, u32) {
        (
            self.freq_scaler.current_freq_mhz,
            self.freq_scaler.min_freq_mhz,
            self.freq_scaler.max_freq_mhz
        )
    }
    
    /// Get thermal information
    pub fn get_thermal_info(&self) -> (f32, bool) {
        (
            self.thermal_manager.cpu_temperature_celsius,
            self.thermal_manager.thermal_throttling_active
        )
    }
    
    /// Force CPU frequency (for debugging/testing)
    pub fn set_cpu_frequency(&mut self, freq_mhz: u32) -> AgaveResult<()> {
        self.freq_scaler.set_frequency(freq_mhz)
    }
}

/// Global power manager
static POWER_MANAGER: Mutex<PowerManager> = Mutex::new(PowerManager {
    current_state: PowerState::Active,
    current_policy: PowerPolicy::Balanced,
    freq_scaler: CpuFrequencyScaler {
        current_freq_mhz: 2400,
        min_freq_mhz: 800,
        max_freq_mhz: 2400,
        available_frequencies: Vec::new(),
        scaling_governor: ScalingGovernor::Ondemand,
    },
    thermal_manager: ThermalManager {
        cpu_temperature_celsius: 45.0,
        thermal_zones: BTreeMap::new(),
        cooling_policies: Vec::new(),
        thermal_throttling_active: false,
    },
    statistics: PowerStatistics {
        time_in_states: BTreeMap::new(),
        frequency_changes: 0,
        sleep_events: 0,
        wake_events: 0,
        power_consumption_estimates: PowerConsumption {
            cpu_watts: 0.0,
            total_watts: 0.0,
            battery_life_estimate_minutes: None,
        },
        thermal_events: 0,
    },
    last_activity_time: AtomicU64::new(0),
    sleep_disabled: AtomicBool::new(false),
    state_change_time: 0,
});

/// Public API functions
pub fn update_power_management() -> AgaveResult<()> {
    let mut pm = POWER_MANAGER.lock();
    pm.update()
}

pub fn set_power_policy(policy: PowerPolicy) -> AgaveResult<()> {
    let mut pm = POWER_MANAGER.lock();
    pm.set_policy(policy)
}

pub fn set_sleep_disabled(disabled: bool) {
    let pm = POWER_MANAGER.lock();
    pm.set_sleep_disabled(disabled);
}

pub fn record_system_activity() {
    let pm = POWER_MANAGER.lock();
    pm.record_activity();
}

pub fn get_power_state() -> PowerState {
    let pm = POWER_MANAGER.lock();
    pm.get_state().clone()
}

pub fn get_power_policy() -> PowerPolicy {
    let pm = POWER_MANAGER.lock();
    pm.get_policy().clone()
}

pub fn get_power_statistics() -> PowerStatistics {
    let pm = POWER_MANAGER.lock();
    pm.get_statistics().clone()
}

pub fn get_cpu_frequency_info() -> (u32, u32, u32) {
    let pm = POWER_MANAGER.lock();
    pm.get_cpu_frequency_info()
}

pub fn get_thermal_info() -> (f32, bool) {
    let pm = POWER_MANAGER.lock();
    pm.get_thermal_info()
}

pub fn set_cpu_frequency(freq_mhz: u32) -> AgaveResult<()> {
    let mut pm = POWER_MANAGER.lock();
    pm.set_cpu_frequency(freq_mhz)
}

/// Initialize power management system
pub fn init_power_management() {
    log::info!("Power management system initialized");
    
    // Initialize with current time
    let now = crate::sys::interrupts::TIME_MS.load(Ordering::Relaxed);
    {
        let pm = POWER_MANAGER.lock();
        pm.last_activity_time.store(now, Ordering::Relaxed);
    }
    
    add_diagnostic(
        DiagnosticLevel::Info,
        DiagnosticCategory::System,
        "Power management initialized".to_string(),
        Some("CPU frequency scaling and thermal management enabled".to_string())
    );
}
