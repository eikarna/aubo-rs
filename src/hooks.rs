//! Network interception hooks for aubo-rs

use std::collections::HashMap;

use std::net::IpAddr;
use std::os::raw::c_void;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

use log::{debug, error, info};
use parking_lot::RwLock;

use crate::config::AuboConfig;
use crate::engine::FilterEngine;
use crate::error::{HookError, Result};
use crate::stats::StatsCollector;
use crate::zygisk::{get_zygisk_api, ZygiskApi};

/// Network function hook information
#[derive(Debug)]
pub struct HookInfo {
    pub name: String,
    pub library: String,
    pub original_fn: *mut c_void,
    pub installed: AtomicBool,
}

impl Clone for HookInfo {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            library: self.library.clone(),
            original_fn: self.original_fn,
            installed: AtomicBool::new(self.installed.load(Ordering::SeqCst)),
        }
    }
}

unsafe impl Send for HookInfo {}
unsafe impl Sync for HookInfo {}

/// Network request context
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub url: String,
    pub domain: String,
    pub request_type: String,
    pub origin_process: String,
    pub timestamp: Instant,
    pub ip_address: Option<IpAddr>,
}

/// Network hooks manager
pub struct NetworkHooks {
    config: Arc<AuboConfig>,
    filter_engine: Arc<FilterEngine>,
    stats: Arc<StatsCollector>,
    hooks: RwLock<HashMap<String, HookInfo>>,
    zygisk_api: Option<&'static ZygiskApi>,
    request_counter: AtomicUsize,
    blocked_counter: AtomicUsize,
}

impl NetworkHooks {
    /// Create a new NetworkHooks instance
    pub fn new(
        config: Arc<AuboConfig>,
        filter_engine: Arc<FilterEngine>,
        stats: Arc<StatsCollector>,
    ) -> Result<Self> {
        Ok(Self {
            config,
            filter_engine,
            stats,
            hooks: RwLock::new(HashMap::new()),
            zygisk_api: get_zygisk_api(),
            request_counter: AtomicUsize::new(0),
            blocked_counter: AtomicUsize::new(0),
        })
    }

    /// Install all configured network hooks
    pub fn install_hooks(&self) -> Result<()> {
        if !self.config.hooks.enabled {
            info!("Network hooks disabled in configuration");
            return Ok(());
        }

        let api = self.zygisk_api.as_ref().ok_or_else(|| {
            HookError::InstallationFailed {
                function: "all".to_string(),
                reason: "ZygiskNext API not available".to_string(),
            }
        })?;

        info!("Installing network hooks");

        for hook_config in &self.config.hooks.hook_functions {
            if !hook_config.enabled {
                continue;
            }

            match self.install_hook(api, hook_config) {
                Ok(_) => info!("Installed hook for: {}", hook_config.name),
                Err(e) => error!("Failed to install hook for {}: {}", hook_config.name, e),
            }
        }

        Ok(())
    }

    /// Install a specific network hook
    fn install_hook(
        &self,
        api: &ZygiskApi,
        hook_config: &crate::config::HookFunction,
    ) -> Result<()> {
        let resolver = api.new_symbol_resolver(&hook_config.library)?;
        let (symbol_addr, _) = resolver.lookup_symbol(&hook_config.name)?
            .ok_or_else(|| HookError::SymbolNotFound {
                symbol: hook_config.name.clone(),
                library: hook_config.library.clone(),
            })?;

        // For now, just create placeholder hooks
        let hook_fn = std::ptr::null_mut::<c_void>();
        let original_fn = api.inline_hook(symbol_addr, hook_fn)?;

        let hook_info = HookInfo {
            name: hook_config.name.clone(),
            library: hook_config.library.clone(),
            original_fn,
            installed: AtomicBool::new(true),
        };

        self.hooks
            .write()
            .insert(hook_config.name.clone(), hook_info);

        Ok(())
    }

    /// Uninstall all network hooks
    pub fn uninstall_hooks(&self) -> Result<()> {
        let api = self.zygisk_api.as_ref().ok_or_else(|| {
            HookError::RemovalFailed {
                function: "all".to_string(),
                reason: "ZygiskNext API not available".to_string(),
            }
        })?;

        info!("Uninstalling network hooks");

        let hooks = self.hooks.read();
        for (name, hook_info) in hooks.iter() {
            if hook_info.installed.load(Ordering::SeqCst) {
                match api.inline_unhook(hook_info.original_fn) {
                    Ok(_) => {
                        hook_info.installed.store(false, Ordering::SeqCst);
                        info!("Uninstalled hook for: {}", name);
                    }
                    Err(e) => error!("Failed to uninstall hook for {}: {}", name, e),
                }
            }
        }

        Ok(())
    }

    /// Analyze a network request and determine if it should be blocked
    pub fn analyze_request(&self, context: &RequestContext) -> bool {
        self.request_counter.fetch_add(1, Ordering::SeqCst);

        let should_block = self.filter_engine.should_block(
            &context.url,
            &context.request_type,
            &context.origin_process,
        );

        if should_block {
            self.blocked_counter.fetch_add(1, Ordering::SeqCst);
            info!("Blocked request: {} from {}", context.domain, context.origin_process);
            self.stats.record_blocked_request(&context.domain, &context.request_type);
        } else {
            debug!("Allowed request: {} from {}", context.domain, context.origin_process);
            self.stats.record_allowed_request(&context.domain, &context.request_type);
        }

        should_block
    }

    /// Get request statistics
    pub fn get_stats(&self) -> (usize, usize) {
        (
            self.request_counter.load(Ordering::SeqCst),
            self.blocked_counter.load(Ordering::SeqCst),
        )
    }
}