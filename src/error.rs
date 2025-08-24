//! Error handling for aubo-rs
//!
//! This module provides comprehensive error types for all aubo-rs operations,
//! ensuring proper error handling and debugging capabilities.

use std::fmt;
use thiserror::Error;

/// Main error type for aubo-rs operations
#[derive(Error, Debug)]
pub enum AuboError {
    /// Configuration-related errors
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),

    /// Filter engine errors
    #[error("Filter engine error: {0}")]
    Filter(#[from] FilterError),

    /// Network hooking errors
    #[error("Network hook error: {0}")]
    Hook(#[from] HookError),

    /// Statistics collection errors
    #[error("Statistics error: {0}")]
    Stats(#[from] StatsError),

    /// ZygiskNext integration errors
    #[error("Zygisk error: {0}")]
    Zygisk(#[from] ZygiskError),

    /// I/O errors
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML parsing errors
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// URL parsing errors
    #[error("URL error: {0}")]
    Url(#[from] url::ParseError),

    /// Regex compilation errors
    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),

    /// Generic errors with context
    #[error("Generic error: {message}")]
    Generic { message: String },

    /// System initialization errors
    #[error("Initialization error: {0}")]
    Initialization(String),

    /// System shutdown errors
    #[error("Shutdown error: {0}")]
    Shutdown(String),
}

/// Configuration-specific errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// Invalid configuration value
    #[error("Invalid configuration value for '{key}': {value}")]
    InvalidValue { key: String, value: String },

    /// Missing required configuration
    #[error("Missing required configuration: {key}")]
    MissingRequired { key: String },

    /// Configuration file not found
    #[error("Configuration file not found: {path}")]
    FileNotFound { path: String },

    /// Configuration file permission denied
    #[error("Permission denied accessing configuration file: {path}")]
    PermissionDenied { path: String },

    /// Invalid configuration file format
    #[error("Invalid configuration file format: {details}")]
    InvalidFormat { details: String },
}

/// Filter engine specific errors
#[derive(Error, Debug)]
pub enum FilterError {
    /// Filter list download failed
    #[error("Failed to download filter list '{name}' from '{url}': {reason}")]
    DownloadFailed {
        name: String,
        url: String,
        reason: String,
    },

    /// Filter list parsing error
    #[error("Failed to parse filter list '{name}': {reason}")]
    ParseError { name: String, reason: String },

    /// Invalid filter rule
    #[error("Invalid filter rule: {rule} - {reason}")]
    InvalidRule { rule: String, reason: String },

    /// Filter compilation error
    #[error("Failed to compile filter: {reason}")]
    CompilationFailed { reason: String },

    /// Filter list not found
    #[error("Filter list not found: {name}")]
    ListNotFound { name: String },

    /// Filter update failed
    #[error("Failed to update filters: {reason}")]
    UpdateFailed { reason: String },
}

/// Network hooking specific errors
#[derive(Error, Debug)]
pub enum HookError {
    /// Failed to find symbol for hooking
    #[error("Symbol not found: {symbol} in {library}")]
    SymbolNotFound { symbol: String, library: String },

    /// Hook installation failed
    #[error("Failed to install hook for {function}: {reason}")]
    InstallationFailed { function: String, reason: String },

    /// Hook removal failed
    #[error("Failed to remove hook for {function}: {reason}")]
    RemovalFailed { function: String, reason: String },

    /// Memory protection error
    #[error("Memory protection error: {reason}")]
    MemoryProtection { reason: String },

    /// Function signature mismatch
    #[error("Function signature mismatch for {function}: expected {expected}, got {actual}")]
    SignatureMismatch {
        function: String,
        expected: String,
        actual: String,
    },

    /// Hook already installed
    #[error("Hook already installed for function: {function}")]
    AlreadyInstalled { function: String },
}

/// Statistics collection specific errors
#[derive(Error, Debug)]
pub enum StatsError {
    /// Failed to initialize statistics collection
    #[error("Failed to initialize statistics collection: {reason}")]
    InitializationFailed { reason: String },

    /// Failed to write statistics
    #[error("Failed to write statistics to {path}: {reason}")]
    WriteFailed { path: String, reason: String },

    /// Failed to read statistics
    #[error("Failed to read statistics from {path}: {reason}")]
    ReadFailed { path: String, reason: String },

    /// Statistics corruption detected
    #[error("Statistics file corruption detected: {details}")]
    Corruption { details: String },
}

/// ZygiskNext integration specific errors
#[derive(Error, Debug)]
pub enum ZygiskError {
    /// ZygiskNext not available
    #[error("ZygiskNext is not available on this system")]
    NotAvailable,

    /// API version mismatch
    #[error("ZygiskNext API version mismatch: expected {expected}, got {actual}")]
    VersionMismatch { expected: u32, actual: u32 },

    /// Module loading failed
    #[error("Failed to load ZygiskNext module: {reason}")]
    ModuleLoadFailed { reason: String },

    /// Companion connection failed
    #[error("Failed to connect to companion module: {reason}")]
    CompanionConnectionFailed { reason: String },

    /// IPC communication error
    #[error("IPC communication error: {reason}")]
    IpcError { reason: String },

    /// Process injection failed
    #[error("Process injection failed for {process}: {reason}")]
    InjectionFailed { process: String, reason: String },
}

/// Result type alias for aubo-rs operations
pub type Result<T> = std::result::Result<T, AuboError>;

/// Helper trait for adding context to errors
pub trait ErrorContext<T> {
    /// Add context to an error
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;

    /// Add static context to an error
    fn context(self, msg: &'static str) -> Result<T>;
}

impl<T, E> ErrorContext<T> for std::result::Result<T, E>
where
    E: Into<AuboError>,
{
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| {
            let original_error = e.into();
            AuboError::Generic {
                message: format!("{}: {}", f(), original_error),
            }
        })
    }

    fn context(self, msg: &'static str) -> Result<T> {
        self.with_context(|| msg.to_string())
    }
}

/// Helper function to create a generic error
pub fn generic_error(message: impl Into<String>) -> AuboError {
    AuboError::Generic {
        message: message.into(),
    }
}

/// Helper function to create an initialization error
pub fn init_error(message: impl Into<String>) -> AuboError {
    AuboError::Initialization(message.into())
}

/// Helper function to create a shutdown error
pub fn shutdown_error(message: impl Into<String>) -> AuboError {
    AuboError::Shutdown(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let config_error = ConfigError::InvalidValue {
            key: "max_connections".to_string(),
            value: "invalid".to_string(),
        };
        let aubo_error = AuboError::Config(config_error);
        
        assert!(aubo_error.to_string().contains("Configuration error"));
        assert!(aubo_error.to_string().contains("max_connections"));
    }

    #[test]
    fn test_error_context() {
        let result: std::result::Result<(), std::io::Error> = 
            Err(std::io::Error::new(std::io::ErrorKind::NotFound, "test"));
        
        let with_context = result.context("Failed to read file");
        assert!(with_context.is_err());
        assert!(with_context.unwrap_err().to_string().contains("Failed to read file"));
    }
}