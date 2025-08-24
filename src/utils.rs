//! Utility functions for aubo-rs
//!
//! This module provides common utility functions used throughout the aubo-rs system,
//! including string processing, URL parsing, performance helpers, and more.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use url::Url;
use regex::Regex;
use once_cell::sync::Lazy;

use crate::error::{Result, AuboError};

/// Fast hash function for strings
pub fn fast_hash(data: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

/// Extract domain from URL
pub fn extract_domain(url: &str) -> Result<String> {
    let parsed = Url::parse(url)?;
    parsed
        .host_str()
        .map(|host| host.to_lowercase())
        .ok_or_else(|| AuboError::Generic {
            message: format!("No host found in URL: {}", url),
        })
}

/// Extract top-level domain from a domain name
pub fn extract_tld(domain: &str) -> Option<&str> {
    domain.split('.').last()
}

/// Check if a domain is a subdomain of another domain
pub fn is_subdomain_of(subdomain: &str, parent_domain: &str) -> bool {
    if subdomain == parent_domain {
        return true;
    }
    
    subdomain.ends_with(&format!(".{}", parent_domain))
}

/// Normalize URL for consistent processing
pub fn normalize_url(url: &str) -> Result<String> {
    let mut parsed = Url::parse(url)?;
    
    // Remove fragment
    parsed.set_fragment(None);
    
    // Sort query parameters for consistency
    if let Some(query) = parsed.query() {
        let mut params: Vec<_> = query.split('&').collect();
        params.sort_unstable();
        parsed.set_query(Some(&params.join("&")));
    }
    
    // Convert to lowercase
    let normalized = parsed.as_str().to_lowercase();
    
    Ok(normalized)
}

/// Check if URL matches a wildcard pattern
pub fn matches_wildcard_pattern(url: &str, pattern: &str) -> bool {
    // Convert wildcard pattern to regex
    let regex_pattern = pattern
        .replace(".", r"\.")
        .replace("*", ".*")
        .replace("?", ".");
    
    if let Ok(regex) = Regex::new(&format!("^{}$", regex_pattern)) {
        regex.is_match(url)
    } else {
        false
    }
}

/// Extract request type from URL and headers
pub fn determine_request_type(url: &str, content_type: Option<&str>) -> &'static str {
    // Check content type first
    if let Some(ct) = content_type {
        let ct = ct.to_lowercase();
        if ct.contains("image/") {
            return "image";
        } else if ct.contains("video/") {
            return "media";
        } else if ct.contains("audio/") {
            return "media";
        } else if ct.contains("text/css") {
            return "stylesheet";
        } else if ct.contains("javascript") || ct.contains("application/js") {
            return "script";
        } else if ct.contains("font/") || ct.contains("application/font") {
            return "font";
        }
    }
    
    // Fallback to URL extension analysis
    if let Ok(parsed_url) = Url::parse(url) {
        if let Some(path) = parsed_url.path_segments() {
            if let Some(last_segment) = path.last() {
                if let Some(extension) = last_segment.split('.').last() {
                    match extension.to_lowercase().as_str() {
                        "css" => return "stylesheet",
                        "js" | "mjs" => return "script",
                        "jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "ico" => return "image",
                        "mp4" | "webm" | "avi" | "mov" | "mp3" | "wav" | "ogg" => return "media",
                        "woff" | "woff2" | "ttf" | "otf" | "eot" => return "font",
                        "xml" => return "xmlhttprequest",
                        _ => {}
                    }
                }
            }
        }
    }
    
    "other"
}

/// Time-based utilities
pub struct TimeUtils;

impl TimeUtils {
    /// Get current timestamp in milliseconds
    pub fn now_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    /// Get current timestamp in seconds
    pub fn now_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Convert duration to human-readable string
    pub fn duration_to_string(duration: Duration) -> String {
        let total_seconds = duration.as_secs();
        let days = total_seconds / 86400;
        let hours = (total_seconds % 86400) / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let seconds = total_seconds % 60;
        
        if days > 0 {
            format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
        } else if hours > 0 {
            format!("{}h {}m {}s", hours, minutes, seconds)
        } else if minutes > 0 {
            format!("{}m {}s", minutes, seconds)
        } else {
            format!("{}s", seconds)
        }
    }
}

/// Performance measurement utilities
pub struct PerfTimer {
    start: Instant,
    name: String,
}

impl PerfTimer {
    /// Start a new performance timer
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
        }
    }
    
    /// Get elapsed time without stopping the timer
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    
    /// Stop the timer and return elapsed time
    pub fn stop(self) -> Duration {
        let elapsed = self.start.elapsed();
        log::debug!("Timer '{}' elapsed: {:?}", self.name, elapsed);
        elapsed
    }
}

/// Memory usage utilities
pub struct MemoryUtils;

impl MemoryUtils {
    /// Get current process memory usage in bytes
    pub fn get_memory_usage() -> Result<u64> {
        #[cfg(target_os = "android")]
        {
            use std::fs;
            let statm = fs::read_to_string("/proc/self/statm")?;
            let fields: Vec<&str> = statm.trim().split_whitespace().collect();
            if fields.len() >= 2 {
                let rss_pages: u64 = fields[1].parse().unwrap_or(0);
                let page_size = 4096; // 4KB pages on most systems
                Ok(rss_pages * page_size)
            } else {
                Ok(0)
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            // Fallback for non-Android platforms
            Ok(0)
        }
    }
    
    /// Format bytes to human-readable string
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = bytes as f64;
        let mut unit_index = 0;
        
        while size >= 1024.0 && unit_index < UNITS.len() - 1 {
            size /= 1024.0;
            unit_index += 1;
        }
        
        if unit_index == 0 {
            format!("{} {}", bytes, UNITS[unit_index])
        } else {
            format!("{:.2} {}", size, UNITS[unit_index])
        }
    }
}

/// String utilities
pub struct StringUtils;

impl StringUtils {
    /// Check if string contains any of the given patterns (case-insensitive)
    pub fn contains_any(text: &str, patterns: &[&str]) -> bool {
        let text_lower = text.to_lowercase();
        patterns.iter().any(|pattern| text_lower.contains(&pattern.to_lowercase()))
    }
    
    /// Remove common URL tracking parameters
    pub fn clean_tracking_params(url: &str) -> Result<String> {
        static TRACKING_PARAMS: Lazy<Vec<&str>> = Lazy::new(|| {
            vec![
                "utm_source", "utm_medium", "utm_campaign", "utm_term", "utm_content",
                "gclid", "fbclid", "msclkid", "_ga", "mc_eid", "mc_cid",
                "ref", "referrer", "source", "campaign", "medium",
                "igshid", "feature", "gws_rd"
            ]
        });
        
        let mut parsed = Url::parse(url)?;
        
        if let Some(query) = parsed.query() {
            let params: Vec<_> = query
                .split('&')
                .filter(|param| {
                    if let Some(key) = param.split('=').next() {
                        !TRACKING_PARAMS.contains(&key)
                    } else {
                        true
                    }
                })
                .collect();
            
            if params.is_empty() {
                parsed.set_query(None);
            } else {
                parsed.set_query(Some(&params.join("&")));
            }
        }
        
        Ok(parsed.to_string())
    }
    
    /// Truncate string to specified length with ellipsis
    pub fn truncate(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else {
            format!("{}...", &text[..max_len.saturating_sub(3)])
        }
    }
}

/// Network utilities
pub struct NetworkUtils;

impl NetworkUtils {
    /// Check if an IP address is private/local
    pub fn is_private_ip(ip: &str) -> bool {
        // Simple check for common private IP ranges
        ip.starts_with("192.168.") ||
        ip.starts_with("10.") ||
        ip.starts_with("172.") ||
        ip.starts_with("127.") ||
        ip == "localhost"
    }
    
    /// Extract port from URL
    pub fn extract_port(url: &str) -> Option<u16> {
        if let Ok(parsed) = Url::parse(url) {
            parsed.port()
        } else {
            None
        }
    }
    
    /// Check if URL uses HTTPS
    pub fn is_https(url: &str) -> bool {
        url.starts_with("https://")
    }
}

/// Configuration validation utilities
pub struct ValidationUtils;

impl ValidationUtils {
    /// Validate URL format
    pub fn is_valid_url(url: &str) -> bool {
        Url::parse(url).is_ok()
    }
    
    /// Validate domain name format
    pub fn is_valid_domain(domain: &str) -> bool {
        // Basic domain validation
        static DOMAIN_REGEX: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?(\.[a-zA-Z0-9]([a-zA-Z0-9\-]{0,61}[a-zA-Z0-9])?)*$")
                .unwrap()
        });
        
        !domain.is_empty() && 
        domain.len() <= 253 && 
        DOMAIN_REGEX.is_match(domain)
    }
    
    /// Validate filter rule format
    pub fn is_valid_filter_rule(rule: &str) -> bool {
        // Basic filter rule validation
        !rule.trim().is_empty() && 
        !rule.starts_with('#') && // Not a comment
        rule.len() <= 1000 // Reasonable length limit
    }
}

/// System information utilities
pub struct SystemInfo;

impl SystemInfo {
    /// Get Android API level
    pub fn get_android_api_level() -> u32 {
        #[cfg(target_os = "android")]
        {
            use std::fs;
            if let Ok(content) = fs::read_to_string("/system/build.prop") {
                for line in content.lines() {
                    if line.starts_with("ro.build.version.sdk=") {
                        if let Some(value) = line.split('=').nth(1) {
                            return value.parse().unwrap_or(21);
                        }
                    }
                }
            }
            21 // Default to API 21 if can't determine
        }
        #[cfg(not(target_os = "android"))]
        {
            21 // Default for non-Android platforms
        }
    }
    
    /// Check if running on rooted device
    pub fn is_rooted() -> bool {
        #[cfg(target_os = "android")]
        {
            use std::path::Path;
            Path::exists(Path::new("/data/adb")) || 
            Path::exists(Path::new("/sbin/su")) ||
            Path::exists(Path::new("/system/bin/su")) ||
            Path::exists(Path::new("/system/xbin/su"))
        }
        #[cfg(not(target_os = "android"))]
        {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path").unwrap(), "example.com");
        assert_eq!(extract_domain("http://sub.example.com").unwrap(), "sub.example.com");
    }

    #[test]
    fn test_is_subdomain_of() {
        assert!(is_subdomain_of("sub.example.com", "example.com"));
        assert!(is_subdomain_of("example.com", "example.com"));
        assert!(!is_subdomain_of("example.com", "sub.example.com"));
    }

    #[test]
    fn test_determine_request_type() {
        assert_eq!(determine_request_type("https://example.com/style.css", None), "stylesheet");
        assert_eq!(determine_request_type("https://example.com/script.js", None), "script");
        assert_eq!(determine_request_type("https://example.com/image.png", None), "image");
    }

    #[test]
    fn test_memory_utils() {
        assert_eq!(MemoryUtils::format_bytes(1024), "1.00 KB");
        assert_eq!(MemoryUtils::format_bytes(1048576), "1.00 MB");
    }

    #[test]
    fn test_validation_utils() {
        assert!(ValidationUtils::is_valid_url("https://example.com"));
        assert!(ValidationUtils::is_valid_domain("example.com"));
        assert!(ValidationUtils::is_valid_filter_rule("||example.com^"));
        assert!(!ValidationUtils::is_valid_filter_rule("# comment"));
    }
}