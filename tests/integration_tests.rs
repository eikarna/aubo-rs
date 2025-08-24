//! Integration tests for aubo-rs
//! 
//! These tests verify that all components work together correctly
//! and that the system meets performance requirements.

use std::sync::Arc;
use std::time::{Duration, Instant};
use tempfile::TempDir;

use aubo_rs::config::AuboConfig;
use aubo_rs::{initialize, shutdown, should_block_request};

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Create a test configuration with temporary directories
    fn create_test_config() -> (AuboConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let mut config = AuboConfig::default();
        
        config.general.data_dir = temp_dir.path().to_path_buf();
        config.filters.filters_dir = temp_dir.path().join("filters");
        config.stats.stats_file = temp_dir.path().join("stats.json");
        
        // Disable some features for testing
        config.filters.update_interval = Duration::from_secs(3600); // 1 hour
        config.performance.worker_threads = 1;
        config.logging.console = false;
        
        (config, temp_dir)
    }

    #[test]
    fn test_system_initialization_and_shutdown() {
        let (config, _temp_dir) = create_test_config();
        
        // Test initialization
        let result = initialize(config);
        assert!(result.is_ok(), "Failed to initialize aubo-rs: {:?}", result);
        
        // Test shutdown
        let result = shutdown();
        assert!(result.is_ok(), "Failed to shutdown aubo-rs: {:?}", result);
    }

    #[test]
    fn test_basic_blocking_functionality() {
        let (config, _temp_dir) = create_test_config();
        
        // Initialize system
        initialize(config).unwrap();
        
        // Test blocking known ad domains
        assert!(should_block_request(
            "https://googleadservices.com/ads/test",
            "http",
            "com.example.app"
        ));
        
        assert!(should_block_request(
            "https://doubleclick.net/track",
            "http", 
            "com.example.app"
        ));
        
        // Test allowing clean domains
        assert!(!should_block_request(
            "https://github.com/user/repo",
            "http",
            "com.example.app"
        ));
        
        assert!(!should_block_request(
            "https://stackoverflow.com/questions",
            "http",
            "com.example.app"
        ));
        
        // Cleanup
        shutdown().unwrap();
    }

    #[test]
    fn test_pattern_based_blocking() {
        let (config, _temp_dir) = create_test_config();
        
        initialize(config).unwrap();
        
        // Test pattern-based blocking
        assert!(should_block_request(
            "https://example.com/ads/banner.js",
            "http",
            "com.example.app"
        ));
        
        assert!(should_block_request(
            "https://analytics.example.com",
            "http",
            "com.example.app"
        ));
        
        assert!(should_block_request(
            "https://tracking.service.com/pixel",
            "http",
            "com.example.app"
        ));
        
        // Test clean requests
        assert!(!should_block_request(
            "https://example.com/api/content",
            "http",
            "com.example.app"
        ));
        
        shutdown().unwrap();
    }

    #[test]
    fn test_performance_requirements() {
        let (config, _temp_dir) = create_test_config();
        
        initialize(config).unwrap();
        
        // Test processing speed - should handle requests in <1ms average
        let test_urls = vec![
            "https://example.com/content",
            "https://googleadservices.com/ads",
            "https://github.com/user/repo", 
            "https://doubleclick.net/track",
            "https://analytics.example.com",
            "https://stackoverflow.com/questions",
            "https://tracking.service.com",
            "https://clean.example.com/api",
        ];
        
        let iterations = 1000;
        let start = Instant::now();
        
        for i in 0..iterations {
            let url = &test_urls[i % test_urls.len()];
            should_block_request(url, "http", "com.example.app");
        }
        
        let duration = start.elapsed();
        let avg_per_request = duration / iterations;
        
        println!("Average processing time per request: {:?}", avg_per_request);
        
        // Should process each request in less than 1ms on average
        assert!(
            avg_per_request < Duration::from_millis(1),
            "Performance requirement not met: {:?} per request",
            avg_per_request
        );
        
        shutdown().unwrap();
    }

    #[test]
    fn test_concurrent_requests() {
        let (config, _temp_dir) = create_test_config();
        
        initialize(config).unwrap();
        
        use std::thread;
        use std::sync::Arc;
        use std::sync::atomic::{AtomicUsize, Ordering};
        
        let request_count = Arc::new(AtomicUsize::new(0));
        let blocked_count = Arc::new(AtomicUsize::new(0));
        
        let handles: Vec<_> = (0..10).map(|thread_id| {
            let request_count = Arc::clone(&request_count);
            let blocked_count = Arc::clone(&blocked_count);
            
            thread::spawn(move || {
                for i in 0..100 {
                    let url = format!("https://test{}.com/path{}", thread_id, i);
                    let blocked = should_block_request(&url, "http", "com.example.app");
                    
                    request_count.fetch_add(1, Ordering::SeqCst);
                    if blocked {
                        blocked_count.fetch_add(1, Ordering::SeqCst);
                    }
                }
            })
        }).collect();
        
        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }
        
        let total_requests = request_count.load(Ordering::SeqCst);
        let total_blocked = blocked_count.load(Ordering::SeqCst);
        
        println!("Processed {} concurrent requests, blocked {}", total_requests, total_blocked);
        
        assert_eq!(total_requests, 1000);
        // Some requests should be blocked due to patterns
        assert!(total_blocked > 0);
        
        shutdown().unwrap();
    }

    #[test]
    fn test_memory_usage() {
        let (config, _temp_dir) = create_test_config();
        
        initialize(config).unwrap();
        
        // Process many requests to test memory usage
        for i in 0..10000 {
            let url = format!("https://test{}.com/path", i % 100);
            should_block_request(&url, "http", "com.example.app");
        }
        
        // Memory usage should remain stable
        // (In a real test, you would measure actual memory usage here)
        
        shutdown().unwrap();
    }

    #[test]
    fn test_configuration_validation() {
        let (mut config, _temp_dir) = create_test_config();
        
        // Test invalid configuration
        config.general.max_memory_mb = 0;
        let result = config.validate();
        assert!(result.is_err());
        
        // Test valid configuration
        config.general.max_memory_mb = 64;
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_filter_engine_edge_cases() {
        let (config, _temp_dir) = create_test_config();
        
        initialize(config).unwrap();
        
        // Test edge cases
        assert!(!should_block_request("", "http", "com.example.app"));
        assert!(!should_block_request("invalid-url", "http", "com.example.app"));
        assert!(!should_block_request("://invalid", "http", "com.example.app"));
        
        // Test very long URLs
        let long_url = format!("https://example.com/{}", "a".repeat(1000));
        should_block_request(&long_url, "http", "com.example.app"); // Should not crash
        
        shutdown().unwrap();
    }
}

// Benchmark tests (require --features=bench to run)
#[cfg(feature = "bench")]
mod benchmarks {
    use super::*;
    use criterion::{black_box, criterion_group, criterion_main, Criterion};

    fn benchmark_blocking_decision(c: &mut Criterion) {
        let (config, _temp_dir) = create_test_config();
        initialize(config).unwrap();
        
        c.bench_function("blocking_decision", |b| {
            b.iter(|| {
                should_block_request(
                    black_box("https://googleadservices.com/ads/test"),
                    black_box("http"),
                    black_box("com.example.app")
                )
            })
        });
        
        shutdown().unwrap();
    }

    fn benchmark_clean_request(c: &mut Criterion) {
        let (config, _temp_dir) = create_test_config();
        initialize(config).unwrap();
        
        c.bench_function("clean_request", |b| {
            b.iter(|| {
                should_block_request(
                    black_box("https://github.com/user/repo"),
                    black_box("http"),
                    black_box("com.example.app")
                )
            })
        });
        
        shutdown().unwrap();
    }

    criterion_group!(benches, benchmark_blocking_decision, benchmark_clean_request);
    criterion_main!(benches);
}