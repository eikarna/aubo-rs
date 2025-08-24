use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::Arc;
use std::time::Duration;

use aubo_rs::config::AuboConfig;
use aubo_rs::engine::FilterEngine;
use aubo_rs::stats::StatsCollector;
use aubo_rs::{initialize, should_block_request, shutdown};

fn setup_test_system() {
    let mut config = AuboConfig::default();
    config.logging.console = false;
    config.performance.worker_threads = 1;
    initialize(config).unwrap();
}

fn teardown_test_system() {
    shutdown().unwrap();
}

fn benchmark_filter_engine(c: &mut Criterion) {
    let config = Arc::new(AuboConfig::default());
    let stats = Arc::new(StatsCollector::new());
    let engine = FilterEngine::new(config, stats).unwrap();

    let test_urls = vec![
        "https://googleadservices.com/ads/test",
        "https://doubleclick.net/track",
        "https://github.com/user/repo",
        "https://stackoverflow.com/questions",
        "https://example.com/ads/banner.js",
        "https://analytics.example.com",
        "https://clean.example.com/api",
        "https://tracking.service.com/pixel",
    ];

    c.bench_function("filter_engine_should_block", |b| {
        let mut i = 0;
        b.iter(|| {
            let url = &test_urls[i % test_urls.len()];
            i += 1;
            black_box(engine.should_block(url, "http", "com.example.app"))
        })
    });
}

fn benchmark_blocking_decisions(c: &mut Criterion) {
    setup_test_system();

    let test_cases = vec![
        ("ad_domain", "https://googleadservices.com/ads/test"),
        ("tracking_domain", "https://doubleclick.net/track"),
        ("clean_domain", "https://github.com/user/repo"),
        ("pattern_match", "https://example.com/ads/banner.js"),
        ("analytics", "https://analytics.example.com"),
        ("clean_api", "https://clean.example.com/api/data"),
    ];

    let mut group = c.benchmark_group("blocking_decisions");
    
    for (name, url) in test_cases {
        group.bench_with_input(BenchmarkId::new("should_block", name), url, |b, url| {
            b.iter(|| {
                black_box(should_block_request(
                    black_box(url),
                    black_box("http"),
                    black_box("com.example.app"),
                ))
            })
        });
    }
    
    group.finish();
    teardown_test_system();
}

fn benchmark_concurrent_requests(c: &mut Criterion) {
    setup_test_system();
    
    let thread_counts = vec![1, 2, 4, 8];
    let requests_per_thread = 100;
    
    let mut group = c.benchmark_group("concurrent_requests");
    
    for thread_count in thread_counts {
        group.bench_with_input(
            BenchmarkId::new("threads", thread_count),
            &thread_count,
            |b, &thread_count| {
                b.iter(|| {
                    let handles: Vec<_> = (0..thread_count)
                        .map(|thread_id| {
                            std::thread::spawn(move || {
                                for i in 0..requests_per_thread {
                                    let url = format!("https://test{}.com/path{}", thread_id, i);
                                    should_block_request(&url, "http", "com.example.app");
                                }
                            })
                        })
                        .collect();
                    
                    for handle in handles {
                        handle.join().unwrap();
                    }
                })
            },
        );
    }
    
    group.finish();
    teardown_test_system();
}

fn benchmark_url_parsing(c: &mut Criterion) {
    let test_urls = vec![
        "https://example.com/path",
        "http://sub.example.com/long/path/to/resource",
        "https://very-long-domain-name.example.com/api/v1/data",
        "https://googleadservices.com/ads/test?param=value",
        "https://tracking.analytics.example.com/pixel.gif",
    ];

    c.bench_function("url_parsing", |b| {
        let mut i = 0;
        b.iter(|| {
            let url = &test_urls[i % test_urls.len()];
            i += 1;
            
            // Test URL parsing performance
            if let Ok(parsed) = url::Url::parse(url) {
                black_box(parsed.host_str());
                black_box(parsed.path());
            }
        })
    });
}

fn benchmark_pattern_matching(c: &mut Criterion) {
    let patterns = ["ads", "analytics", "tracking", "adnxs", "adsystem"];
    let test_strings = vec![
        "https://example.com/ads/banner.js",
        "https://analytics.service.com/track",
        "https://clean.example.com/api/data",
        "https://tracking.example.com/pixel",
        "https://normal.website.com/content",
    ];

    c.bench_function("pattern_matching", |b| {
        let mut i = 0;
        b.iter(|| {
            let test_string = &test_strings[i % test_strings.len()];
            i += 1;
            
            black_box(patterns.iter().any(|pattern| test_string.contains(pattern)))
        })
    });
}

fn benchmark_memory_allocation(c: &mut Criterion) {
    c.bench_function("string_operations", |b| {
        let mut counter = 0;
        b.iter(|| {
            counter += 1;
            let url = format!("https://test{}.com/path", counter);
            let domain = url.split('/').nth(2).unwrap_or("");
            black_box(domain.to_string());
        })
    });
}

fn benchmark_large_scale_processing(c: &mut Criterion) {
    setup_test_system();
    
    let request_counts = vec![100, 1000, 10000];
    
    let mut group = c.benchmark_group("large_scale");
    group.measurement_time(Duration::from_secs(10));
    
    for request_count in request_counts {
        group.bench_with_input(
            BenchmarkId::new("requests", request_count),
            &request_count,
            |b, &request_count| {
                b.iter(|| {
                    for i in 0..request_count {
                        let url = format!("https://test{}.com/path", i % 100);
                        should_block_request(&url, "http", "com.example.app");
                    }
                })
            },
        );
    }
    
    group.finish();
    teardown_test_system();
}

criterion_group!(
    benches,
    benchmark_filter_engine,
    benchmark_blocking_decisions,
    benchmark_concurrent_requests,
    benchmark_url_parsing,
    benchmark_pattern_matching,
    benchmark_memory_allocation,
    benchmark_large_scale_processing
);

criterion_main!(benches);