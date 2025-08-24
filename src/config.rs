//! Configuration management for aubo-rs
//!
//! This module handles all configuration aspects of the aubo-rs system,
//! including loading, validation, and runtime configuration updates.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use url::Url;

use crate::error::{AuboError, ConfigError, Result};

/// Default configuration file name
pub const DEFAULT_CONFIG_FILE: &str = "aubo-rs.toml";

/// Default data directory path on Android
pub const DEFAULT_DATA_DIR: &str = "/data/adb/aubo-rs";

/// Default filter lists directory
pub const DEFAULT_FILTERS_DIR: &str = "/data/adb/aubo-rs/filters";

/// Default statistics file path
pub const DEFAULT_STATS_FILE: &str = "/data/adb/aubo-rs/stats.json";

/// Main configuration structure for aubo-rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuboConfig {
    /// General system configuration
    pub general: GeneralConfig,
    
    /// Filter engine configuration
    pub filters: FilterConfig,
    
    /// Network hooking configuration
    pub hooks: HookConfig,
    
    /// Statistics collection configuration
    pub stats: StatsConfig,
    
    /// Performance tuning configuration
    pub performance: PerformanceConfig,
    
    /// Logging configuration
    pub logging: LoggingConfig,
}

/// General system configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    /// Enable/disable the entire system
    pub enabled: bool,
    
    /// Data directory path
    pub data_dir: PathBuf,
    
    /// Configuration file path
    pub config_file: PathBuf,
    
    /// Debug mode enable/disable
    pub debug_mode: bool,
    
    /// System update check interval
    pub update_check_interval: Duration,
    
    /// Maximum memory usage (in MB)
    pub max_memory_mb: u64,
    
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f32,
}

/// Filter engine configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterConfig {
    /// Enable/disable filter engine
    pub enabled: bool,
    
    /// Filter lists directory
    pub filters_dir: PathBuf,
    
    /// Default filter lists to load
    pub default_lists: Vec<FilterListConfig>,
    
    /// Custom filter rules
    pub custom_rules: Vec<String>,
    
    /// Filter update interval
    pub update_interval: Duration,
    
    /// Maximum filter rules in memory
    pub max_rules: usize,
    
    /// Enable filter rule compilation for better performance
    pub compile_rules: bool,
    
    /// Cache compiled filters
    pub cache_compiled: bool,
    
    /// Whitelist domains (never block)
    pub whitelist_domains: Vec<String>,
    
    /// Blacklist domains (always block)
    pub blacklist_domains: Vec<String>,
}

/// Filter list configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterListConfig {
    /// Filter list name
    pub name: String,
    
    /// Filter list URL for updates
    pub url: Url,
    
    /// Filter list type (easylist, adguard, ublock, custom)
    pub list_type: FilterListType,
    
    /// Enable/disable this filter list
    pub enabled: bool,
    
    /// Update interval override for this list
    pub update_interval: Option<Duration>,
    
    /// Priority (higher priority lists are checked first)
    pub priority: u32,
}

/// Filter list types supported by aubo-rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FilterListType {
    /// EasyList format
    EasyList,
    /// AdGuard format
    AdGuard,
    /// uBlock Origin format
    UBlockOrigin,
    /// Custom format
    Custom,
    /// Hosts file format
    Hosts,
}

/// Network hooking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookConfig {
    /// Enable/disable network hooks
    pub enabled: bool,
    
    /// Target processes to hook (empty means hook all)
    pub target_processes: Vec<String>,
    
    /// Exclude processes from hooking
    pub exclude_processes: Vec<String>,
    
    /// Network functions to hook
    pub hook_functions: Vec<HookFunction>,
    
    /// Enable deep packet inspection
    pub deep_inspection: bool,
    
    /// Maximum request size to analyze (in bytes)
    pub max_request_size: usize,
    
    /// Request analysis timeout
    pub analysis_timeout: Duration,
}

/// Network function hooking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookFunction {
    /// Function name to hook
    pub name: String,
    
    /// Library containing the function
    pub library: String,
    
    /// Enable/disable this hook
    pub enabled: bool,
    
    /// Hook priority
    pub priority: u32,
}

/// Statistics collection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsConfig {
    /// Enable/disable statistics collection
    pub enabled: bool,
    
    /// Statistics file path
    pub stats_file: PathBuf,
    
    /// Statistics collection interval
    pub collection_interval: Duration,
    
    /// Statistics retention period
    pub retention_period: Duration,
    
    /// Enable detailed request logging
    pub detailed_logging: bool,
    
    /// Maximum log entries to keep
    pub max_log_entries: usize,
    
    /// Enable performance metrics
    pub performance_metrics: bool,
}

/// Performance tuning configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Worker thread pool size
    pub worker_threads: usize,
    
    /// Request queue size
    pub request_queue_size: usize,
    
    /// Filter cache size
    pub filter_cache_size: usize,
    
    /// DNS cache size
    pub dns_cache_size: usize,
    
    /// DNS cache TTL
    pub dns_cache_ttl: Duration,
    
    /// Enable aggressive caching
    pub aggressive_caching: bool,
    
    /// Memory pressure threshold
    pub memory_pressure_threshold: f32,
    
    /// CPU pressure threshold
    pub cpu_pressure_threshold: f32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (error, warn, info, debug, trace)
    pub level: String,
    
    /// Log file path
    pub log_file: Option<PathBuf>,
    
    /// Maximum log file size (in MB)
    pub max_file_size: u64,
    
    /// Number of log files to keep
    pub max_files: u32,
    
    /// Enable console logging
    pub console: bool,
    
    /// Enable structured logging (JSON)
    pub structured: bool,
}

impl Default for AuboConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            filters: FilterConfig::default(),
            hooks: HookConfig::default(),
            stats: StatsConfig::default(),
            performance: PerformanceConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            data_dir: PathBuf::from(DEFAULT_DATA_DIR),
            config_file: PathBuf::from(DEFAULT_DATA_DIR).join(DEFAULT_CONFIG_FILE),
            debug_mode: false,
            update_check_interval: Duration::from_secs(24 * 60 * 60), // 24 hours
            max_memory_mb: 64,
            max_cpu_percent: 5.0,
        }
    }
}

impl Default for FilterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            filters_dir: PathBuf::from(DEFAULT_FILTERS_DIR),
            default_lists: vec![
                FilterListConfig {
                    name: "EasyList".to_string(),
                    url: Url::parse("https://easylist.to/easylist/easylist.txt").unwrap(),
                    list_type: FilterListType::EasyList,
                    enabled: true,
                    update_interval: None,
                    priority: 100,
                },
                FilterListConfig {
                    name: "EasyPrivacy".to_string(),
                    url: Url::parse("https://easylist.to/easylist/easyprivacy.txt").unwrap(),
                    list_type: FilterListType::EasyList,
                    enabled: true,
                    update_interval: None,
                    priority: 90,
                },
            ],
            custom_rules: Vec::new(),
            update_interval: Duration::from_secs(6 * 60 * 60), // 6 hours
            max_rules: 100000,
            compile_rules: true,
            cache_compiled: true,
            whitelist_domains: Vec::new(),
            blacklist_domains: Vec::new(),
        }
    }
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            target_processes: Vec::new(), // Hook all processes
            exclude_processes: vec![
                "kernel".to_string(),
                "init".to_string(),
                "kthreadd".to_string(),
            ],
            hook_functions: vec![
                HookFunction {
                    name: "getaddrinfo".to_string(),
                    library: "libc.so".to_string(),
                    enabled: true,
                    priority: 100,
                },
                HookFunction {
                    name: "gethostbyname".to_string(),
                    library: "libc.so".to_string(),
                    enabled: true,
                    priority: 90,
                },
                HookFunction {
                    name: "connect".to_string(),
                    library: "libc.so".to_string(),
                    enabled: true,
                    priority: 80,
                },
            ],
            deep_inspection: true,
            max_request_size: 1024 * 1024, // 1MB
            analysis_timeout: Duration::from_millis(100),
        }
    }
}

impl Default for StatsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            stats_file: PathBuf::from(DEFAULT_STATS_FILE),
            collection_interval: Duration::from_secs(60), // 1 minute
            retention_period: Duration::from_secs(7 * 24 * 60 * 60), // 7 days
            detailed_logging: false,
            max_log_entries: 10000,
            performance_metrics: true,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            worker_threads: num_cpus::get().min(4),
            request_queue_size: 1000,
            filter_cache_size: 10000,
            dns_cache_size: 1000,
            dns_cache_ttl: Duration::from_secs(300), // 5 minutes
            aggressive_caching: false,
            memory_pressure_threshold: 0.8,
            cpu_pressure_threshold: 0.7,
        }
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: "info".to_string(),
            log_file: Some(PathBuf::from(DEFAULT_DATA_DIR).join("aubo-rs.log")),
            max_file_size: 10, // 10MB
            max_files: 5,
            console: false, // Don't log to console by default on Android
            structured: false,
        }
    }
}

impl AuboConfig {
    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        
        if !path.exists() {
            return Err(AuboError::Config(ConfigError::FileNotFound {
                path: path.to_string_lossy().to_string(),
            }));
        }

        let content = fs::read_to_string(path)
            .map_err(|e| AuboError::Config(ConfigError::PermissionDenied {
                path: path.to_string_lossy().to_string(),
            }))?;

        let config: AuboConfig = toml::from_str(&content)
            .map_err(|e| AuboError::Config(ConfigError::InvalidFormat {
                details: e.to_string(),
            }))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let path = path.as_ref();
        
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)?;
        fs::write(path, content)?;
        
        Ok(())
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate general config
        if self.general.max_memory_mb == 0 {
            return Err(AuboError::Config(ConfigError::InvalidValue {
                key: "general.max_memory_mb".to_string(),
                value: "0".to_string(),
            }));
        }

        if self.general.max_cpu_percent < 0.0 || self.general.max_cpu_percent > 100.0 {
            return Err(AuboError::Config(ConfigError::InvalidValue {
                key: "general.max_cpu_percent".to_string(),
                value: self.general.max_cpu_percent.to_string(),
            }));
        }

        // Validate filter config
        if self.filters.max_rules == 0 {
            return Err(AuboError::Config(ConfigError::InvalidValue {
                key: "filters.max_rules".to_string(),
                value: "0".to_string(),
            }));
        }

        // Validate performance config
        if self.performance.worker_threads == 0 {
            return Err(AuboError::Config(ConfigError::InvalidValue {
                key: "performance.worker_threads".to_string(),
                value: "0".to_string(),
            }));
        }

        // Validate logging level
        match self.logging.level.as_str() {
            "error" | "warn" | "info" | "debug" | "trace" => {},
            _ => {
                return Err(AuboError::Config(ConfigError::InvalidValue {
                    key: "logging.level".to_string(),
                    value: self.logging.level.clone(),
                }));
            }
        }

        Ok(())
    }

    /// Create default configuration file
    pub fn create_default_config<P: AsRef<Path>>(path: P) -> Result<()> {
        let config = Self::default();
        config.save_to_file(path)
    }

    /// Get the effective data directory (create if it doesn't exist)
    pub fn ensure_data_dir(&self) -> Result<&Path> {
        if !self.general.data_dir.exists() {
            fs::create_dir_all(&self.general.data_dir)?;
        }
        Ok(&self.general.data_dir)
    }

    /// Get the effective filters directory (create if it doesn't exist)
    pub fn ensure_filters_dir(&self) -> Result<&Path> {
        if !self.filters.filters_dir.exists() {
            fs::create_dir_all(&self.filters.filters_dir)?;
        }
        Ok(&self.filters.filters_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_creation() {
        let config = AuboConfig::default();
        
        assert!(config.general.enabled);
        assert_eq!(config.general.data_dir, PathBuf::from(DEFAULT_DATA_DIR));
        assert!(config.filters.enabled);
        assert!(!config.filters.default_lists.is_empty());
        assert!(config.hooks.enabled);
        assert!(config.stats.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AuboConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid memory setting
        config.general.max_memory_mb = 0;
        assert!(config.validate().is_err());
        
        // Invalid CPU setting
        config.general.max_memory_mb = 64;
        config.general.max_cpu_percent = 150.0;
        assert!(config.validate().is_err());
        
        // Invalid worker threads
        config.general.max_cpu_percent = 5.0;
        config.performance.worker_threads = 0;
        assert!(config.validate().is_err());
        
        // Invalid log level
        config.performance.worker_threads = 2;
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        // Create and save config
        let original_config = AuboConfig::default();
        original_config.save_to_file(&config_path).unwrap();
        
        // Load config
        let loaded_config = AuboConfig::load_from_file(&config_path).unwrap();
        
        // Compare key values
        assert_eq!(original_config.general.enabled, loaded_config.general.enabled);
        assert_eq!(original_config.filters.enabled, loaded_config.filters.enabled);
        assert_eq!(original_config.hooks.enabled, loaded_config.hooks.enabled);
    }

    #[test]
    fn test_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AuboConfig::default();
        config.general.data_dir = temp_dir.path().join("aubo-rs");
        config.filters.filters_dir = temp_dir.path().join("aubo-rs/filters");
        
        // Directories should not exist initially
        assert!(!config.general.data_dir.exists());
        assert!(!config.filters.filters_dir.exists());
        
        // ensure_data_dir should create the directory
        let data_dir = config.ensure_data_dir().unwrap();
        assert!(data_dir.exists());
        
        // ensure_filters_dir should create the directory
        let filters_dir = config.ensure_filters_dir().unwrap();
        assert!(filters_dir.exists());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_default_config_creation() {
        let config = AuboConfig::default();
        
        assert!(config.general.enabled);
        assert_eq!(config.general.data_dir, PathBuf::from(DEFAULT_DATA_DIR));
        assert!(config.filters.enabled);
        assert!(!config.filters.default_lists.is_empty());
        assert!(config.hooks.enabled);
        assert!(config.stats.enabled);
    }

    #[test]
    fn test_config_validation() {
        let mut config = AuboConfig::default();
        
        // Valid config should pass
        assert!(config.validate().is_ok());
        
        // Invalid memory setting
        config.general.max_memory_mb = 0;
        assert!(config.validate().is_err());
        
        // Invalid CPU setting
        config.general.max_memory_mb = 64;
        config.general.max_cpu_percent = 150.0;
        assert!(config.validate().is_err());
        
        // Invalid worker threads
        config.general.max_cpu_percent = 5.0;
        config.performance.worker_threads = 0;
        assert!(config.validate().is_err());
        
        // Invalid log level
        config.performance.worker_threads = 2;
        config.logging.level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_file_operations() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");
        
        // Create and save config
        let original_config = AuboConfig::default();
        original_config.save_to_file(&config_path).unwrap();
        
        // Load config
        let loaded_config = AuboConfig::load_from_file(&config_path).unwrap();
        
        // Compare key values
        assert_eq!(original_config.general.enabled, loaded_config.general.enabled);
        assert_eq!(original_config.filters.enabled, loaded_config.filters.enabled);
        assert_eq!(original_config.hooks.enabled, loaded_config.hooks.enabled);
    }

    #[test]
    fn test_config_file_not_found() {
        let result = AuboConfig::load_from_file("/nonexistent/path/config.toml");
        assert!(result.is_err());
        
        if let Err(AuboError::Config(ConfigError::FileNotFound { path })) = result {
            assert!(path.contains("nonexistent"));
        } else {
            panic!("Expected FileNotFound error");
        }
    }

    #[test]
    fn test_invalid_config_format() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("invalid_config.toml");
        
        // Write invalid TOML
        fs::write(&config_path, "invalid toml content [[[").unwrap();
        
        let result = AuboConfig::load_from_file(&config_path);
        assert!(result.is_err());
        
        if let Err(AuboError::Config(ConfigError::InvalidFormat { .. })) = result {
            // Expected error type
        } else {
            panic!("Expected InvalidFormat error");
        }
    }

    #[test]
    fn test_filter_list_config() {
        let config = FilterListConfig {
            name: "Test List".to_string(),
            url: Url::parse("https://example.com/filters.txt").unwrap(),
            list_type: FilterListType::EasyList,
            enabled: true,
            update_interval: Some(Duration::from_secs(3600)),
            priority: 100,
        };
        
        assert_eq!(config.name, "Test List");
        assert_eq!(config.url.host_str(), Some("example.com"));
        assert_eq!(config.priority, 100);
    }

    #[test]
    fn test_directory_creation() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AuboConfig::default();
        config.general.data_dir = temp_dir.path().join("aubo-rs");
        config.filters.filters_dir = temp_dir.path().join("aubo-rs/filters");
        
        // Directories should not exist initially
        assert!(!config.general.data_dir.exists());
        assert!(!config.filters.filters_dir.exists());
        
        // ensure_data_dir should create the directory
        let data_dir = config.ensure_data_dir().unwrap();
        assert!(data_dir.exists());
        
        // ensure_filters_dir should create the directory
        let filters_dir = config.ensure_filters_dir().unwrap();
        assert!(filters_dir.exists());
    }

    #[test]
    fn test_performance_config_defaults() {
        let config = PerformanceConfig::default();
        
        assert!(config.worker_threads > 0);
        assert!(config.worker_threads <= 4);
        assert!(config.request_queue_size > 0);
        assert!(config.filter_cache_size > 0);
        assert!(config.memory_pressure_threshold > 0.0);
        assert!(config.memory_pressure_threshold <= 1.0);
    }

    #[test]
    fn test_hook_function_config() {
        let hook = HookFunction {
            name: "getaddrinfo".to_string(),
            library: "libc.so".to_string(),
            enabled: true,
            priority: 100,
        };
        
        assert_eq!(hook.name, "getaddrinfo");
        assert_eq!(hook.library, "libc.so");
        assert!(hook.enabled);
        assert_eq!(hook.priority, 100);
    }

    #[test]
    fn test_logging_config_validation() {
        let mut config = LoggingConfig::default();
        
        // Valid log levels
        for level in &["error", "warn", "info", "debug", "trace"] {
            config.level = level.to_string();
            let full_config = AuboConfig {
                logging: config.clone(),
                ..Default::default()
            };
            assert!(full_config.validate().is_ok());
        }
        
        // Invalid log level
        config.level = "invalid".to_string();
        let full_config = AuboConfig {
            logging: config,
            ..Default::default()
        };
        assert!(full_config.validate().is_err());
    }
}
