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