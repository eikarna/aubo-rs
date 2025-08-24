//! Filter engine for aubo-rs ad-blocking

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use aho_corasick::AhoCorasick;
use log::info;
use parking_lot::RwLock;
use regex::Regex;

use crate::config::AuboConfig;
use crate::error::Result;
use crate::stats::StatsCollector;

/// Filter rule types
#[derive(Debug, Clone)]
pub enum FilterRule {
    /// Block rule
    Block { pattern: String, regex: Option<Regex> },
    /// Allow rule (whitelist)
    Allow { pattern: String, regex: Option<Regex> },
    /// Host-based block
    HostBlock { domain: String },
}

/// Filter engine for processing requests
pub struct FilterEngine {
    config: Arc<AuboConfig>,
    stats: Arc<StatsCollector>,
    rules: RwLock<Vec<FilterRule>>,
    domain_blocklist: RwLock<HashSet<String>>,
    domain_allowlist: RwLock<HashSet<String>>,
    pattern_matcher: RwLock<Option<AhoCorasick>>,
    last_update: RwLock<Instant>,
}

impl FilterEngine {
    /// Create a new filter engine
    pub fn new(config: Arc<AuboConfig>, stats: Arc<StatsCollector>) -> Result<Self> {
        let engine = Self {
            config,
            stats,
            rules: RwLock::new(Vec::new()),
            domain_blocklist: RwLock::new(HashSet::new()),
            domain_allowlist: RwLock::new(HashSet::new()),
            pattern_matcher: RwLock::new(None),
            last_update: RwLock::new(Instant::now()),
        };

        engine.load_default_filters()?;
        Ok(engine)
    }

    /// Check if a request should be blocked
    pub fn should_block(&self, url: &str, request_type: &str, origin: &str) -> bool {
        // Check allowlist first (whitelist takes priority)
        if self.is_whitelisted(url) {
            return false;
        }

        // Check domain blocklist
        if let Some(domain) = extract_domain(url) {
            if self.domain_blocklist.read().contains(&domain) {
                return true;
            }
        }

        // Check pattern-based rules
        self.check_pattern_rules(url, request_type, origin)
    }

    /// Load default filter lists
    fn load_default_filters(&self) -> Result<()> {
        info!("Loading default filter lists");
        
        // Load built-in blocklist
        let mut blocklist = self.domain_blocklist.write();
        blocklist.extend([
            "googleadservices.com".to_string(),
            "doubleclick.net".to_string(),
            "googlesyndication.com".to_string(),
            "facebook.com".to_string(),
            "analytics.google.com".to_string(),
        ]);

        // Load allowlist
        let mut allowlist = self.domain_allowlist.write();
        allowlist.extend([
            "github.com".to_string(),
            "stackoverflow.com".to_string(),
        ]);

        info!("Loaded {} blocked domains, {} allowed domains", 
              blocklist.len(), allowlist.len());
        Ok(())
    }

    /// Check if URL is whitelisted
    fn is_whitelisted(&self, url: &str) -> bool {
        if let Some(domain) = extract_domain(url) {
            self.domain_allowlist.read().contains(&domain)
        } else {
            false
        }
    }

    /// Check pattern-based rules
    fn check_pattern_rules(&self, url: &str, _request_type: &str, _origin: &str) -> bool {
        // Simple pattern matching for now
        let patterns = ["ads", "analytics", "tracking", "adnxs", "adsystem"];
        patterns.iter().any(|pattern| url.contains(pattern))
    }

    /// Start background tasks
    pub fn start_background_tasks(&self) -> Result<()> {
        info!("Starting filter engine background tasks");
        Ok(())
    }

    /// Stop background tasks
    pub fn stop_background_tasks(&self) -> Result<()> {
        info!("Stopping filter engine background tasks");
        Ok(())
    }
}

/// Extract domain from URL
fn extract_domain(url: &str) -> Option<String> {
    if let Ok(parsed) = url::Url::parse(url) {
        parsed.host_str().map(|h| h.to_string())
    } else {
        // Handle domain-only strings
        if url.contains("://") {
            None
        } else {
            Some(url.split('/').next()?.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AuboConfig;
    use crate::stats::StatsCollector;
    use std::sync::Arc;

    fn create_test_engine() -> FilterEngine {
        let config = Arc::new(AuboConfig::default());
        let stats = Arc::new(StatsCollector::new());
        FilterEngine::new(config, stats).unwrap()
    }

    #[test]
    fn test_filter_engine_creation() {
        let engine = create_test_engine();
        assert!(!engine.domain_blocklist.read().is_empty());
        assert!(!engine.domain_allowlist.read().is_empty());
    }

    #[test]
    fn test_domain_blocking() {
        let engine = create_test_engine();
        
        // Test blocked domains
        assert!(engine.should_block("https://googleadservices.com/ads", "http", "test"));
        assert!(engine.should_block("https://doubleclick.net/track", "http", "test"));
        
        // Test allowed domains
        assert!(!engine.should_block("https://github.com/user/repo", "http", "test"));
        assert!(!engine.should_block("https://stackoverflow.com/questions", "http", "test"));
    }

    #[test]
    fn test_pattern_blocking() {
        let engine = create_test_engine();
        
        // Test pattern-based blocking
        assert!(engine.should_block("https://example.com/ads/banner.js", "http", "test"));
        assert!(engine.should_block("https://example.com/analytics.js", "http", "test"));
        assert!(engine.should_block("https://tracking.example.com", "http", "test"));
        
        // Test clean URLs
        assert!(!engine.should_block("https://example.com/content.js", "http", "test"));
        assert!(!engine.should_block("https://example.com/api/data", "http", "test"));
    }

    #[test]
    fn test_whitelist_priority() {
        let engine = create_test_engine();
        
        // Whitelist should override blocklist
        assert!(!engine.should_block("https://github.com/ads/something", "http", "test"));
    }

    #[test]
    fn test_extract_domain() {
        assert_eq!(extract_domain("https://example.com/path"), Some("example.com".to_string()));
        assert_eq!(extract_domain("http://sub.example.com"), Some("sub.example.com".to_string()));
        assert_eq!(extract_domain("example.com"), Some("example.com".to_string()));
        assert_eq!(extract_domain("invalid://"), None);
    }

    #[test]
    fn test_performance_blocking() {
        let engine = create_test_engine();
        let start = Instant::now();
        
        // Test performance with 1000 requests
        for i in 0..1000 {
            let url = format!("https://example{}.com/path", i);
            engine.should_block(&url, "http", "test");
        }
        
        let duration = start.elapsed();
        println!("1000 requests processed in {:?}", duration);
        
        // Should process requests very quickly
        assert!(duration.as_millis() < 100, "Performance test failed: took {:?}", duration);
    }
}