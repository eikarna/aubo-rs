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
    // Load default configuration or from standard path
    let config_path = "/data/adb/aubo-rs/aubo-rs.toml";
    let config = match AuboConfig::load_from_file(config_path) {
        Ok(config) => config,
        Err(_) => {
            warn!("Failed to load config from {}, using defaults", config_path);
            AuboConfig::default()
        }
    };
    
    initialize(config)
}

/// Handle companion process connection for ZygiskNext
/// This function manages communication with Zygisk companion processes
pub fn handle_companion_connection(fd: i32) -> Result<()> {
    info!("Handling companion connection on fd: {}", fd);
    // For now, just acknowledge the connection
    // In a full implementation, this would handle companion process communication
    Ok(())
}

/// C-compatible initialization function
#[no_mangle]
pub extern "C" fn aubo_initialize(config_path: *const c_char) -> c_int {
    let config_path = unsafe {
        if config_path.is_null() {
            return -1;
        }
        match CStr::from_ptr(config_path).to_str() {
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
pub extern "C" fn aubo_shutdown() -> c_int {
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
pub extern "C" fn aubo_should_block_request(
    url: *const c_char,
    request_type: *const c_char,
    origin: *const c_char,
) -> c_int {
    let (url, request_type, origin) = unsafe {
        if url.is_null() || request_type.is_null() || origin.is_null() {
            return 0;
        }
        
        let url = match CStr::from_ptr(url).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        
        let request_type = match CStr::from_ptr(request_type).to_str() {
            Ok(s) => s,
            Err(_) => return 0,
        };
        
        let origin = match CStr::from_ptr(origin).to_str() {
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