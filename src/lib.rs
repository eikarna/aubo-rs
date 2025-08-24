//! # aubo-rs: Android uBlock Origin Rust
//! 
//! A high-performance, system-wide ad-blocker for Android built with Rust and ZygiskNext.
//! This module provides comprehensive network request filtering, analysis, and blocking
//! capabilities that operate at the system level for maximum effectiveness and minimal
//! performance impact.
//!
//! ## Features
//!
//! - **System-wide blocking**: Intercepts network requests at the ZygiskNext level
//! - **Multiple filter formats**: Supports EasyList, AdGuard, uBlock Origin filters
//! - **High performance**: Rust-powered filtering engine with minimal overhead
//! - **Real-time updates**: Dynamic filter list updates without restarts
//! - **Comprehensive stats**: Detailed blocking statistics and performance metrics
//! - **Configurable**: Extensive configuration options for advanced users
//!
//! ## Architecture
//!
//! The system is built on several core components:
//!
//! - [`hooks`]: Network interception and ZygiskNext integration
//! - [`filters`]: Filter list management and request analysis
//! - [`engine`]: Core blocking engine and decision logic
//! - [`config`]: Configuration management and persistence
//! - [`stats`]: Performance monitoring and statistics collection
//!
//! ## Safety
//!
//! This crate uses unsafe code for system-level interception and FFI bindings.
//! All unsafe blocks are carefully reviewed and documented.

#![warn(
    missing_docs,
    rust_2018_idioms,
    unreachable_pub,
    bad_style,
    dead_code,
    improper_ctypes,
    non_shorthand_field_patterns,
    no_mangle_generic_items,
    overflowing_literals,
    path_statements,
    patterns_in_fns_without_body,
    unconditional_recursion,
    unused,
    unused_allocation,
    unused_comparisons,
    unused_parens,
    while_true
)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod config;
pub mod engine;
pub mod error;
pub mod filters;
pub mod hooks;
pub mod stats;
pub mod utils;
pub mod zygisk;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::Result;
use log::{error, info, warn};
use once_cell::sync::Lazy;
use parking_lot::RwLock;

use crate::config::AuboConfig;
use crate::engine::FilterEngine;
use crate::hooks::NetworkHooks;
use crate::stats::StatsCollector;

/// Global instance of the aubo-rs system
pub static AUBO_INSTANCE: Lazy<Arc<RwLock<Option<AuboSystem>>>> = 
    Lazy::new(|| Arc::new(RwLock::new(None)));

/// Global flag indicating if the system is initialized
static INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Main aubo-rs system that coordinates all components
pub struct AuboSystem {
    /// Configuration manager
    config: Arc<AuboConfig>,
    /// Filter engine for request analysis
    filter_engine: Arc<FilterEngine>,
    /// Network hooks for interception
    network_hooks: Arc<NetworkHooks>,
    /// Statistics collector
    stats: Arc<StatsCollector>,
    /// Shutdown flag
    shutdown: AtomicBool,
}

impl AuboSystem {
    /// Initialize the aubo-rs system with the given configuration
    pub fn new(config: AuboConfig) -> Result<Self> {
        info!("Initializing aubo-rs system");
        
        let config = Arc::new(config);
        let stats = Arc::new(StatsCollector::new());
        let filter_engine = Arc::new(FilterEngine::new(Arc::clone(&config), Arc::clone(&stats))?);
        let network_hooks = Arc::new(NetworkHooks::new(
            Arc::clone(&config),
            Arc::clone(&filter_engine),
            Arc::clone(&stats),
        )?);

        Ok(Self {
            config,
            filter_engine,
            network_hooks,
            stats,
            shutdown: AtomicBool::new(false),
        })
    }

    /// Start the aubo-rs system
    pub fn start(&self) -> Result<()> {
        info!("Starting aubo-rs system");
        
        // Initialize network hooks
        self.network_hooks.install_hooks()?;
        
        // Start filter engine background tasks
        self.filter_engine.start_background_tasks()?;
        
        // Start statistics collection
        self.stats.start_collection()?;
        
        info!("aubo-rs system started successfully");
        Ok(())
    }

    /// Stop the aubo-rs system
    pub fn stop(&self) -> Result<()> {
        info!("Stopping aubo-rs system");
        
        self.shutdown.store(true, Ordering::SeqCst);
        
        // Stop components in reverse order
        self.stats.stop_collection()?;
        self.filter_engine.stop_background_tasks()?;
        self.network_hooks.uninstall_hooks()?;
        
        info!("aubo-rs system stopped successfully");
        Ok(())
    }

    /// Check if the system is shutting down
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown.load(Ordering::SeqCst)
    }

    /// Get a reference to the configuration
    pub fn config(&self) -> &Arc<AuboConfig> {
        &self.config
    }

    /// Get a reference to the filter engine
    pub fn filter_engine(&self) -> &Arc<FilterEngine> {
        &self.filter_engine
    }

    /// Get a reference to the network hooks
    pub fn network_hooks(&self) -> &Arc<NetworkHooks> {
        &self.network_hooks
    }

    /// Get a reference to the stats collector
    pub fn stats(&self) -> &Arc<StatsCollector> {
        &self.stats
    }
}

/// Initialize the global aubo-rs system
pub fn initialize(config: AuboConfig) -> Result<()> {
    if INITIALIZED.load(Ordering::SeqCst) {
        warn!("aubo-rs system already initialized");
        return Ok(());
    }

    let system = AuboSystem::new(config)?;
    system.start()?;

    {
        let mut instance = AUBO_INSTANCE.write();
        *instance = Some(system);
    }

    INITIALIZED.store(true, Ordering::SeqCst);
    info!("aubo-rs global system initialized");
    Ok(())
}

/// Shutdown the global aubo-rs system
pub fn shutdown() -> Result<()> {
    if !INITIALIZED.load(Ordering::SeqCst) {
        warn!("aubo-rs system not initialized");
        return Ok(());
    }

    let system = {
        let mut instance = AUBO_INSTANCE.write();
        instance.take()
    };

    if let Some(system) = system {
        system.stop()?;
    }

    INITIALIZED.store(false, Ordering::SeqCst);
    info!("aubo-rs global system shutdown");
    Ok(())
}

/// Get a reference to the global aubo-rs system
pub fn get_system() -> Option<Arc<RwLock<Option<AuboSystem>>>> {
    if INITIALIZED.load(Ordering::SeqCst) {
        Some(Arc::clone(&AUBO_INSTANCE))
    } else {
        None
    }
}

/// Check if a request should be blocked
/// 
/// This is the main entry point for request filtering
pub fn should_block_request(url: &str, request_type: &str, origin: &str) -> bool {
    if let Some(system_ref) = get_system() {
        if let Some(system) = system_ref.read().as_ref() {
            return system.filter_engine().should_block(url, request_type, origin);
        }
    }
    false
}

// C FFI exports for ZygiskNext integration
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};

/// Initialize aubo-rs from ZygiskNext context
/// This function is called by the ZygiskNext module during initialization
pub fn initialize_from_zygisk() -> Result<()> {
    // Set up logging first
    setup_logging();
    
    // Enhanced dmesg logging with priority markers
    log_to_dmesg("=== ZygiskNext module initialization started ===");
    log_to_dmesg(&format!("aubo-rs version: {}", env!("CARGO_PKG_VERSION")));
    log_to_dmesg(&format!("Process ID: {}", std::process::id()));
    
    // Log system information for debugging
    if let Ok(api_level) = std::env::var("ANDROID_API") {
        log_to_dmesg(&format!("Android API Level: {}", api_level));
    }
    
    // Ensure data directory exists with proper permissions
    let data_dir = "/data/adb/aubo-rs";
    if let Err(e) = std::fs::create_dir_all(data_dir) {
        log_to_dmesg(&format!("Failed to create data directory: {}", e));
    } else {
        log_to_dmesg("Data directory verified/created successfully");
    }
    
    // Set proper permissions using chmod
    let _ = std::process::Command::new("chmod")
        .arg("-R")
        .arg("755")
        .arg(data_dir)
        .output();
    
    // Load configuration with enhanced error handling
    let config_path = "/data/adb/aubo-rs/aubo-rs.toml";
    log_to_dmesg(&format!("Attempting to load configuration from: {}", config_path));
    
    let config = match AuboConfig::load_from_file(config_path) {
        Ok(config) => {
            log_to_dmesg(&format!("Configuration loaded successfully from {}", config_path));
            config
        },
        Err(e) => {
            warn!("Failed to load config from {}: {}", config_path, e);
            log_to_dmesg(&format!("Config load failed: {} - creating default configuration", e));
            
            match create_default_config(config_path) {
                Ok(config) => {
                    log_to_dmesg("Default configuration created successfully");
                    config
                },
                Err(e) => {
                    log_to_dmesg(&format!("Failed to create default config: {}", e));
                    update_status_file("error", &format!("Configuration creation failed: {}", e));
                    return Err(e);
                }
            }
        }
    };
    
    // Verify ZygiskNext environment
    log_to_dmesg("Verifying ZygiskNext environment...");
    if std::path::Path::new("/data/adb/modules/zygisksu").exists() {
        log_to_dmesg("ZygiskNext module detected");
    } else {
        log_to_dmesg("WARNING: ZygiskNext module directory not found");
    }
    
    // Initialize the main system
    log_to_dmesg("Initializing aubo-rs main system...");
    update_status_file("initializing", "Starting main system initialization");
    
    match initialize(config) {
        Ok(_) => {
            log_to_dmesg("=== System initialization completed successfully ===");
            log_to_dmesg("aubo-rs is now active and monitoring network requests");
            update_status_file("running", "System initialized and actively filtering requests");
            
            // Log successful hooks installation
            log_to_dmesg("Network hooks installed - request interception active");
            log_to_dmesg("Filter engine started - ad-blocking rules loaded");
            log_to_dmesg("Statistics collection enabled - monitoring performance");
            
            Ok(())
        }
        Err(e) => {
            error!("Failed to initialize aubo-rs: {}", e);
            log_to_dmesg(&format!("=== INITIALIZATION FAILED: {} ===", e));
            log_to_dmesg("aubo-rs module is not active - no ad-blocking will occur");
            update_status_file("error", &format!("Initialization failed: {}", e));
            
            // Provide debugging hints
            log_to_dmesg("Debugging hints:");
            log_to_dmesg("1. Check if ZygiskNext is properly installed and enabled");
            log_to_dmesg("2. Verify /data/adb/aubo-rs directory permissions");
            log_to_dmesg("3. Check logcat output: logcat -s aubo-rs");
            log_to_dmesg("4. Run health check: sh /data/adb/aubo-rs/health_check.sh");
            
            Err(e)
        }
    }
}

/// Handle companion process connection for ZygiskNext
/// This function manages communication with Zygisk companion processes
pub fn handle_companion_connection(fd: i32) -> Result<()> {
    info!("Handling companion connection on fd: {}", fd);
    log_to_dmesg(&format!("aubo-rs: Companion connection established on fd: {}", fd));
    // For now, just acknowledge the connection
    // In a full implementation, this would handle companion process communication
    Ok(())
}

/// Set up logging for the module
fn setup_logging() {
    env_logger::Builder::from_default_env()
        .target(env_logger::Target::Stderr)
        .init();
}

/// Log message to dmesg for debugging
fn log_to_dmesg(message: &str) {
    use std::process::Command;
    
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    let full_message = format!("aubo-rs: {}", message);
    
    // Method 1: Direct write to /dev/kmsg (most reliable for dmesg)
    if let Err(_) = std::fs::write("/dev/kmsg", format!("<6>{}", full_message)) {
        // Method 2: Try using log command as fallback
        let _ = Command::new("log")
            .arg("-p")
            .arg("i")
            .arg("-t")
            .arg("aubo-rs")
            .arg(message)
            .output();
    }
    
    // Method 3: Also log to logcat for runtime debugging
    let _ = Command::new("logcat")
        .arg("-d")
        .arg("-s")
        .arg("aubo-rs")
        .output();
    
    // Append to debug log file with proper formatting
    let log_entry = format!("{}: {}\n", timestamp, message);
    if let Ok(existing) = std::fs::read_to_string("/data/adb/aubo-rs/logs/debug.log") {
        let _ = std::fs::write("/data/adb/aubo-rs/logs/debug.log", format!("{}{}", existing, log_entry));
    } else {
        let _ = std::fs::write("/data/adb/aubo-rs/logs/debug.log", log_entry);
    }
    
    // Ensure file permissions are correct
    let _ = Command::new("chmod")
        .arg("644")
        .arg("/data/adb/aubo-rs/logs/debug.log")
        .output();
}

/// Update module status file for debugging
fn update_status_file(status: &str, message: &str) {
    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
    
    // Get additional system information
    let hook_count = get_active_hook_count();
    let filter_count = get_loaded_filter_count();
    let uptime = std::fs::read_to_string("/proc/uptime")
        .unwrap_or_default()
        .split_whitespace()
        .next()
        .unwrap_or("unknown")
        .to_string();
    
    let status_content = format!(
        "status={}\ntime={}\nmessage={}\nversion={}\nuptime={}\nhooks_active={}\nfilters_loaded={}\nlast_update={}\nlib_path=/data/adb/modules/aubo_rs/lib/aubo_rs.so\nconfig_path=/data/adb/aubo-rs/aubo-rs.toml\ndebug_log=/data/adb/aubo-rs/debug.log\nprocess_id={}\n",
        status,
        timestamp,
        message,
        env!("CARGO_PKG_VERSION"),
        uptime,
        hook_count,
        filter_count,
        timestamp,
        std::process::id()
    );
    
    let _ = std::fs::write("/data/adb/aubo-rs/status.txt", status_content);
    
    // Also update module.prop if possible
    update_module_prop_status(status, message);
}

/// Get count of active hooks (placeholder - will be implemented with actual hook tracking)
fn get_active_hook_count() -> u32 {
    // This would return actual hook count from NetworkHooks
    if INITIALIZED.load(Ordering::SeqCst) {
        // TODO: Implement actual hook counting
        1  // Placeholder
    } else {
        0
    }
}

/// Get count of loaded filters (placeholder - will be implemented with actual filter tracking)
fn get_loaded_filter_count() -> u32 {
    // This would return actual filter count from FilterEngine
    if INITIALIZED.load(Ordering::SeqCst) {
        // TODO: Implement actual filter counting
        1  // Placeholder
    } else {
        0
    }
}

/// Update module.prop with current status and dynamic description
fn update_module_prop_status(status: &str, message: &str) {
    let module_prop_path = "/data/adb/modules/aubo_rs/module.prop";
    if let Ok(content) = std::fs::read_to_string(module_prop_path) {
        // Get current system information for dynamic description
        let hooks_status = if status == "running" { "✅" } else { "❌" };
        let filter_status = if status == "running" { "✅" } else { "❌" };
        let zygisk_status = if std::path::Path::new("/data/adb/modules/zygisksu").exists() { "✅" } else { "❌" };
        let library_status = if std::path::Path::new("/data/adb/modules/aubo_rs/lib/aubo_rs.so").exists() { "✅" } else { "❌" };
        
        // Get blocked count from stats if available
        let blocked_count = if let Some(system_ref) = get_system() {
            if let Some(system) = system_ref.read().as_ref() {
                let stats = system.stats().get_stats();
                stats.blocked_requests.to_string()
            } else {
                "0".to_string()
            }
        } else {
            "0".to_string()
        };
        
        // Detect root method
        let root_method = if std::path::Path::new("/data/adb/modules/magisk_busybox").exists() || 
                             std::path::Path::new("/system/xbin/magisk").exists() {
            "Magisk"
        } else if std::path::Path::new("/data/adb/modules/kernelsu").exists() || 
                  std::path::Path::new("/system/bin/ksu").exists() {
            "KernelSU"
        } else if std::path::Path::new("/data/adb/modules/apatch").exists() || 
                  std::path::Path::new("/system/bin/apd").exists() {
            "APatch"
        } else {
            "Unknown"
        };
        
        // Create dynamic description similar to ZygiskNext format
        let dynamic_desc = format!(
            "[{}Network Hooks {}Ad Filters {}ZygiskNext {}Library. Root: {}, {} blocked] System-wide ad-blocker using Rust and ZygiskNext",
            hooks_status, filter_status, zygisk_status, library_status, root_method, blocked_count
        );
        
        // Update description line
        let lines: Vec<&str> = content.lines().collect();
        let mut new_content = String::new();
        let mut in_runtime_section = false;
        
        for line in lines {
            if line.starts_with("# Runtime Status") {
                in_runtime_section = true;
                break;
            }
            if line.starts_with("description=") {
                new_content.push_str(&format!("description={}\n", dynamic_desc));
            } else if !in_runtime_section {
                new_content.push_str(line);
                new_content.push('\n');
            }
        }
        
        // Add current runtime status
        new_content.push_str(&format!(
            "\n# Runtime Status\nruntimeStatus={}\nruntimeMessage={}\nruntimeUpdate={}\nruntimeActive={}\nhooksActive={}\nfiltersActive={}\nzygiskActive={}\nlibraryActive={}\nblockedTotal={}\nrootMethod={}\n",
            status,
            message.replace('\n', " "),
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            if status == "running" { "true" } else { "false" },
            hooks_status,
            filter_status,
            zygisk_status,
            library_status,
            blocked_count,
            root_method
        ));
        
        let _ = std::fs::write(module_prop_path, new_content);
    }
}

/// Create default configuration with fallback values
fn create_default_config(config_path: &str) -> Result<AuboConfig> {
    let config = AuboConfig::default();
    
    // Ensure directory exists
    if let Some(parent) = std::path::Path::new(config_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    
    // Save default config
    if let Err(e) = config.save_to_file(config_path) {
        warn!("Failed to save default config: {}", e);
    } else {
        info!("Created default configuration at {}", config_path);
    }
    
    Ok(config)
}

/// C-compatible initialization function
#[no_mangle]
#[export_name = "aubo_initialize"]
pub unsafe extern "C" fn aubo_initialize(config_path: *const c_char) -> c_int {
    let config_path = {
        if config_path.is_null() {
            return -1;
        }
        match unsafe { CStr::from_ptr(config_path) }.to_str() {
            Ok(path) => path,
            Err(_) => return -1,
        }
    };

    match AuboConfig::load_from_file(config_path) {
        Ok(config) => match initialize(config) {
            Ok(_) => 0,
            Err(e) => {
                error!("Failed to initialize aubo-rs: {}", e);
                -1
            }
        },
        Err(e) => {
            error!("Failed to load config from {}: {}", config_path, e);
            -1
        }
    }
}

/// C-compatible shutdown function
#[no_mangle]
#[export_name = "aubo_shutdown"]
pub unsafe extern "C" fn aubo_shutdown() -> c_int {
    match shutdown() {
        Ok(_) => 0,
        Err(e) => {
            error!("Failed to shutdown aubo-rs: {}", e);
            -1
        }
    }
}

/// C-compatible request blocking check
#[no_mangle]
#[export_name = "aubo_should_block_request"]
pub unsafe extern "C" fn aubo_should_block_request(
    url: *const c_char,
    request_type: *const c_char,
    origin: *const c_char,
) -> c_int {
    let (url, request_type, origin) = {
        if url.is_null() || request_type.is_null() || origin.is_null() {
            return 0;
        }
        
        let url = match unsafe { CStr::from_ptr(url) }.to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        
        let request_type = match unsafe { CStr::from_ptr(request_type) }.to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        
        let origin = match unsafe { CStr::from_ptr(origin) }.to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        
        (url, request_type, origin)
    };

    if should_block_request(url, request_type, origin) {
        1
    } else {
        0
    }
}
