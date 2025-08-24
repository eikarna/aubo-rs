use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use crate::error::{AuboError, StatsError};

/// Performance metrics for the ad blocker
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub avg_processing_time_us: u64,
    pub memory_usage_bytes: u64,
    pub cpu_usage_percent: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            avg_processing_time_us: 0,
            memory_usage_bytes: 0,
            cpu_usage_percent: 0.0,
        }
    }
}

/// Statistics about blocked and allowed requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub total_requests: u64,
    pub blocked_requests: u64,
    pub allowed_requests: u64,
    pub domains_blocked: HashMap<String, u64>,
    pub request_types: HashMap<String, u64>,
    pub performance_metrics: PerformanceMetrics,
    pub start_time: u64,
    pub last_updated: u64,
}

impl Default for Stats {
    fn default() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        
        Self {
            total_requests: 0,
            blocked_requests: 0,
            allowed_requests: 0,
            domains_blocked: HashMap::new(),
            request_types: HashMap::new(),
            performance_metrics: PerformanceMetrics::default(),
            start_time: now,
            last_updated: now,
        }
    }
}

/// Thread-safe statistics collector for aubo-rs
#[derive(Debug)]
pub struct StatsCollector {
    stats: Arc<RwLock<Stats>>,
    collecting: Arc<RwLock<bool>>,
}

impl StatsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            stats: Arc::new(RwLock::new(Stats::default())),
            collecting: Arc::new(RwLock::new(false)),
        }
    }

    /// Start collecting statistics
    pub fn start_collection(&self) -> Result<(), AuboError> {
        let mut collecting = self.collecting.write();
        *collecting = true;
        Ok(())
    }

    /// Stop collecting statistics
    pub fn stop_collection(&self) -> Result<(), AuboError> {
        let mut collecting = self.collecting.write();
        *collecting = false;
        Ok(())
    }

    /// Record a blocked request
    pub fn record_blocked_request(&self, domain: &str, request_type: &str) {
        if !*self.collecting.read() {
            return;
        }

        let mut stats = self.stats.write();
        stats.total_requests += 1;
        stats.blocked_requests += 1;
        
        // Update domain count
        *stats.domains_blocked.entry(domain.to_string()).or_insert(0) += 1;
        
        // Update request type count
        *stats.request_types.entry(request_type.to_string()).or_insert(0) += 1;
        
        // Update timestamp
        stats.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Record an allowed request
    pub fn record_allowed_request(&self, _domain: &str, request_type: &str) {
        if !*self.collecting.read() {
            return;
        }

        let mut stats = self.stats.write();
        stats.total_requests += 1;
        stats.allowed_requests += 1;
        
        // Update request type count
        *stats.request_types.entry(request_type.to_string()).or_insert(0) += 1;
        
        // Update timestamp
        stats.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Get a snapshot of current statistics
    pub fn get_stats(&self) -> Stats {
        self.stats.read().clone()
    }

    /// Update performance metrics
    pub fn update_performance_metrics(
        &self,
        avg_processing_time_us: u64,
        memory_usage_bytes: u64,
        cpu_usage_percent: f64,
    ) {
        let mut stats = self.stats.write();
        stats.performance_metrics.avg_processing_time_us = avg_processing_time_us;
        stats.performance_metrics.memory_usage_bytes = memory_usage_bytes;
        stats.performance_metrics.cpu_usage_percent = cpu_usage_percent;
        
        stats.last_updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }

    /// Reset all statistics
    pub fn reset(&self) {
        let mut stats = self.stats.write();
        *stats = Stats::default();
    }

    /// Get statistics as JSON string
    pub fn to_json(&self) -> Result<String, AuboError> {
        let stats = self.get_stats();
        serde_json::to_string_pretty(&stats)
            .map_err(|e| AuboError::Stats(StatsError::SerializationError { 
                message: e.to_string() 
            }))
    }

    /// Save statistics to file
    pub fn save_to_file(&self, path: &str) -> Result<(), AuboError> {
        let json = self.to_json()?;
        std::fs::write(path, json)
            .map_err(|e| AuboError::Stats(StatsError::IoError { 
                message: format!("Failed to write stats to {}: {}", path, e) 
            }))
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for StatsCollector {
    fn clone(&self) -> Self {
        Self {
            stats: Arc::clone(&self.stats),
            collecting: Arc::clone(&self.collecting),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use std::thread;

    #[test]
    fn test_stats_collector_creation() {
        let collector = StatsCollector::new();
        let stats = collector.get_stats();
        
        assert_eq!(stats.total_requests, 0);
        assert_eq!(stats.blocked_requests, 0);
        assert_eq!(stats.allowed_requests, 0);
        assert!(stats.domains_blocked.is_empty());
        assert!(stats.request_types.is_empty());
    }

    #[test]
    fn test_recording_blocked_requests() {
        let collector = StatsCollector::new();
        
        collector.record_blocked_request("example.com", "http");
        collector.record_blocked_request("ads.google.com", "http");
        collector.record_blocked_request("example.com", "https");
        
        let stats = collector.get_stats();
        
        assert_eq!(stats.total_requests, 3);
        assert_eq!(stats.blocked_requests, 3);
        assert_eq!(stats.allowed_requests, 0);
        assert_eq!(stats.domains_blocked.get("example.com"), Some(&2));
        assert_eq!(stats.domains_blocked.get("ads.google.com"), Some(&1));
        assert_eq!(stats.request_types.get("http"), Some(&2));
        assert_eq!(stats.request_types.get("https"), Some(&1));
    }

    #[test]
    fn test_recording_allowed_requests() {
        let collector = StatsCollector::new();
        
        collector.record_allowed_request("github.com", "https");
        collector.record_allowed_request("stackoverflow.com", "https");
        
        let stats = collector.get_stats();
        
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.blocked_requests, 0);
        assert_eq!(stats.allowed_requests, 2);
        assert!(stats.domains_blocked.is_empty());
        assert_eq!(stats.request_types.get("https"), Some(&2));
    }

    #[test]
    fn test_mixed_requests() {
        let collector = StatsCollector::new();
        
        collector.record_blocked_request("ads.example.com", "http");
        collector.record_allowed_request("api.example.com", "https");
        collector.record_blocked_request("tracking.example.com", "http");
        collector.record_allowed_request("cdn.example.com", "https");
        
        let stats = collector.get_stats();
        
        assert_eq!(stats.total_requests, 4);
        assert_eq!(stats.blocked_requests, 2);
        assert_eq!(stats.allowed_requests, 2);
        assert_eq!(stats.domains_blocked.len(), 2);
        assert_eq!(stats.request_types.get("http"), Some(&2));
        assert_eq!(stats.request_types.get("https"), Some(&2));
    }

    #[test]
    fn test_concurrent_stats_recording() {
        let collector = Arc::new(StatsCollector::new());
        let mut handles = Vec::new();
        
        // Spawn multiple threads to record stats concurrently
        for thread_id in 0..10 {
            let collector_clone = Arc::clone(&collector);
            let handle = thread::spawn(move || {
                for i in 0..100 {
                    let domain = format!("domain{}.com", thread_id);
                    if i % 2 == 0 {
                        collector_clone.record_blocked_request(&domain, "http");
                    } else {
                        collector_clone.record_allowed_request(&domain, "https");
                    }
                }
            });
            handles.push(handle);
        }
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        let stats = collector.get_stats();
        
        assert_eq!(stats.total_requests, 1000);
        assert_eq!(stats.blocked_requests, 500);
        assert_eq!(stats.allowed_requests, 500);
        assert_eq!(stats.domains_blocked.len(), 10);
        assert_eq!(stats.request_types.get("http"), Some(&500));
        assert_eq!(stats.request_types.get("https"), Some(&500));
    }

    #[test]
    fn test_performance_metrics_initialization() {
        let collector = StatsCollector::new();
        let stats = collector.get_stats();
        
        assert_eq!(stats.performance_metrics.avg_processing_time_us, 0);
        assert_eq!(stats.performance_metrics.memory_usage_bytes, 0);
        assert_eq!(stats.performance_metrics.cpu_usage_percent, 0.0);
    }

    #[test]
    fn test_stats_start_stop_collection() {
        let collector = StatsCollector::new();
        
        // Should not panic
        assert!(collector.start_collection().is_ok());
        assert!(collector.stop_collection().is_ok());
    }

    #[test]
    fn test_domain_counting_accuracy() {
        let collector = StatsCollector::new();
        let domain = "example.com";
        
        // Record multiple requests for same domain
        for _ in 0..5 {
            collector.record_blocked_request(domain, "http");
        }
        
        let stats = collector.get_stats();
        assert_eq!(stats.domains_blocked.get(domain), Some(&5));
        assert_eq!(stats.total_requests, 5);
        assert_eq!(stats.blocked_requests, 5);
    }

    #[test]
    fn test_request_type_counting() {
        let collector = StatsCollector::new();
        
        collector.record_blocked_request("example.com", "http");
        collector.record_blocked_request("example.com", "https");
        collector.record_blocked_request("example.com", "websocket");
        collector.record_allowed_request("github.com", "http");
        collector.record_allowed_request("github.com", "https");
        
        let stats = collector.get_stats();
        
        assert_eq!(stats.request_types.get("http"), Some(&2));
        assert_eq!(stats.request_types.get("https"), Some(&2));
        assert_eq!(stats.request_types.get("websocket"), Some(&1));
    }

    #[test]
    fn test_stats_serialization() {
        let collector = StatsCollector::new();
        
        collector.record_blocked_request("example.com", "http");
        collector.record_allowed_request("github.com", "https");
        
        let stats = collector.get_stats();
        
        // Test JSON serialization
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("total_requests"));
        assert!(json.contains("blocked_requests"));
        assert!(json.contains("allowed_requests"));
        
        // Test deserialization
        let deserialized: Stats = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_requests, stats.total_requests);
        assert_eq!(deserialized.blocked_requests, stats.blocked_requests);
    }
}