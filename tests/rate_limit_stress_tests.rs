use axum::{
    extract::connect_info::MockConnectInfo, http::StatusCode, middleware::from_fn_with_state,
    response::IntoResponse, routing::get, Router,
};
use axum_test::TestServer;
use basic_axum_rate_limit::{
    rate_limit_middleware, security_context_middleware, NoOpOnBlocked, RateLimitConfig, RateLimiter,
};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::task::JoinSet;

// Test handlers
async fn handler_ok() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn handler_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}

async fn handler_cache(req: axum::http::Request<axum::body::Body>) -> impl IntoResponse {
    if req
        .headers()
        .get(axum::http::header::IF_NONE_MATCH)
        .is_some()
    {
        (StatusCode::NOT_MODIFIED, "")
    } else {
        (StatusCode::OK, "Content")
    }
}

// Helper to create test server
fn create_test_server(config: RateLimitConfig) -> TestServer {
    let rate_limiter = RateLimiter::new(config, NoOpOnBlocked);
    let socket_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

    let app = Router::new()
        .route("/", get(handler_ok))
        .route("/notfound", get(handler_not_found))
        .route("/cache", get(handler_cache))
        .layer(from_fn_with_state(rate_limiter, rate_limit_middleware))
        .layer(axum::middleware::from_fn(security_context_middleware))
        .layer(MockConnectInfo(socket_addr));

    TestServer::new(app).unwrap()
}

struct StressMetrics {
    total_requests: AtomicU64,
    success_2xx: AtomicU64,
    rate_limited_429: AtomicU64,
    errors: AtomicU64,
    min_duration_us: AtomicU64,
    max_duration_us: AtomicU64,
    total_duration_us: AtomicU64,
}

impl StressMetrics {
    fn new() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            success_2xx: AtomicU64::new(0),
            rate_limited_429: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            min_duration_us: AtomicU64::new(u64::MAX),
            max_duration_us: AtomicU64::new(0),
            total_duration_us: AtomicU64::new(0),
        }
    }

    fn record(&self, status: u16, duration_us: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.total_duration_us
            .fetch_add(duration_us, Ordering::Relaxed);

        // Update min/max with atomic operations
        let mut current_min = self.min_duration_us.load(Ordering::Relaxed);
        while duration_us < current_min {
            match self.min_duration_us.compare_exchange_weak(
                current_min,
                duration_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_min = x,
            }
        }

        let mut current_max = self.max_duration_us.load(Ordering::Relaxed);
        while duration_us > current_max {
            match self.max_duration_us.compare_exchange_weak(
                current_max,
                duration_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(x) => current_max = x,
            }
        }

        match status {
            200..=299 => {
                self.success_2xx.fetch_add(1, Ordering::Relaxed);
            }
            429 => {
                self.rate_limited_429.fetch_add(1, Ordering::Relaxed);
            }
            _ => {
                self.errors.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    fn print_report(&self, test_duration: Duration) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let success = self.success_2xx.load(Ordering::Relaxed);
        let rate_limited = self.rate_limited_429.load(Ordering::Relaxed);
        let errors = self.errors.load(Ordering::Relaxed);
        let min_us = self.min_duration_us.load(Ordering::Relaxed);
        let max_us = self.max_duration_us.load(Ordering::Relaxed);
        let total_us = self.total_duration_us.load(Ordering::Relaxed);

        let rps = total as f64 / test_duration.as_secs_f64();
        let avg_us = if total > 0 { total_us / total } else { 0 };

        println!("\n╔══════════════════════════════════════════╗");
        println!("║         STRESS TEST RESULTS              ║");
        println!("╠══════════════════════════════════════════╣");
        println!("║ Duration:        {:.2}s", test_duration.as_secs_f64());
        println!("║ Total Requests:  {}", total);
        println!("║ Throughput:      {:.2} req/s", rps);
        println!("╠══════════════════════════════════════════╣");
        println!("║ Status Codes:                            ║");
        println!(
            "║   2xx Success:   {} ({:.1}%)",
            success,
            (success as f64 / total as f64) * 100.0
        );
        println!(
            "║   429 Limited:   {} ({:.1}%)",
            rate_limited,
            (rate_limited as f64 / total as f64) * 100.0
        );
        println!(
            "║   Errors:        {} ({:.1}%)",
            errors,
            (errors as f64 / total as f64) * 100.0
        );
        println!("╠══════════════════════════════════════════╣");
        println!("║ Latency:                                 ║");
        println!(
            "║   Min:           {}µs ({:.2}ms)",
            min_us,
            min_us as f64 / 1000.0
        );
        println!(
            "║   Max:           {}µs ({:.2}ms)",
            max_us,
            max_us as f64 / 1000.0
        );
        println!(
            "║   Avg:           {}µs ({:.2}ms)",
            avg_us,
            avg_us as f64 / 1000.0
        );
        println!("╚══════════════════════════════════════════╝");
    }
}

#[tokio::test]
#[ignore] // Run with: cargo test --test rate_limit_stress_tests -- --ignored --nocapture
async fn stress_test_extreme_concurrency() {
    println!("\n🔥 STRESS TEST: Extreme Concurrency (10,000 concurrent tasks)");

    let config = RateLimitConfig::new(50, Duration::from_secs(60)).with_grace_period(0);
    let server = Arc::new(create_test_server(config));
    let metrics = Arc::new(StressMetrics::new());

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // Spawn 10,000 concurrent tasks
    for i in 0..10_000 {
        let server = Arc::clone(&server);
        let metrics = Arc::clone(&metrics);
        let ip = format!("10.{}.{}.{}", (i / 65536) % 256, (i / 256) % 256, i % 256);

        tasks.spawn(async move {
            let req_start = Instant::now();
            let response = server.get("/").add_header("X-Forwarded-For", &ip).await;
            let duration_us = req_start.elapsed().as_micros() as u64;
            metrics.record(response.status_code().as_u16(), duration_us);
        });
    }

    while tasks.join_next().await.is_some() {}
    let elapsed = start.elapsed();

    metrics.print_report(elapsed);

    // Assertions
    let total = metrics.total_requests.load(Ordering::Relaxed);
    assert_eq!(total, 10_000, "All requests should complete");
}

#[tokio::test]
#[ignore]
async fn stress_test_sustained_load() {
    println!("\n🔥 STRESS TEST: Sustained Load (30 seconds @ max throughput)");

    let config = RateLimitConfig::new(100, Duration::from_secs(60)).with_grace_period(0);
    let server = Arc::new(create_test_server(config));
    let metrics = Arc::new(StressMetrics::new());
    let running = Arc::new(AtomicUsize::new(1));

    let start = Instant::now();
    let test_duration = Duration::from_secs(30);

    // Spawn 500 concurrent workers that continuously make requests
    let mut tasks = JoinSet::new();
    for i in 0..500 {
        let server = Arc::clone(&server);
        let metrics = Arc::clone(&metrics);
        let running = Arc::clone(&running);
        let ip = format!("192.168.{}.{}", i / 256, i % 256);

        tasks.spawn(async move {
            let mut request_count = 0;
            while running.load(Ordering::Relaxed) == 1 {
                let req_start = Instant::now();
                let response = server.get("/").add_header("X-Forwarded-For", &ip).await;
                let duration_us = req_start.elapsed().as_micros() as u64;
                metrics.record(response.status_code().as_u16(), duration_us);
                request_count += 1;
            }
            request_count
        });
    }

    // Let it run for the test duration
    tokio::time::sleep(test_duration).await;
    running.store(0, Ordering::Relaxed);

    // Wait for all tasks to complete
    let mut total_worker_requests = 0u64;
    while let Some(result) = tasks.join_next().await {
        if let Ok(count) = result {
            total_worker_requests += count;
        }
    }

    let elapsed = start.elapsed();
    metrics.print_report(elapsed);

    println!("\n📊 Additional Metrics:");
    println!("   Worker requests: {}", total_worker_requests);
    println!(
        "   Requests/worker: {:.0}",
        total_worker_requests as f64 / 500.0
    );
}

#[tokio::test]
#[ignore]
async fn stress_test_memory_pressure() {
    println!("\n🔥 STRESS TEST: Memory Pressure (100,000 unique IPs)");

    let config = RateLimitConfig::new(10, Duration::from_secs(60)).with_grace_period(0);
    let server = Arc::new(create_test_server(config));
    let metrics = Arc::new(StressMetrics::new());

    let start = Instant::now();

    // Create 100,000 unique IP entries in the rate limit cache
    // Each IP makes 5 requests to ensure they stay in cache
    for batch in 0..100 {
        let mut tasks = JoinSet::new();

        for i in 0..1000 {
            let server = Arc::clone(&server);
            let metrics = Arc::clone(&metrics);
            let ip_num = batch * 1000 + i;
            let ip = format!(
                "10.{}.{}.{}",
                (ip_num / 65536) % 256,
                (ip_num / 256) % 256,
                ip_num % 256
            );

            tasks.spawn(async move {
                for _ in 0..5 {
                    let req_start = Instant::now();
                    let response = server.get("/").add_header("X-Forwarded-For", &ip).await;
                    let duration_us = req_start.elapsed().as_micros() as u64;
                    metrics.record(response.status_code().as_u16(), duration_us);
                }
            });
        }

        while tasks.join_next().await.is_some() {}

        if batch % 10 == 0 {
            println!(
                "   Processed {} / 100 batches ({} IPs)",
                batch,
                batch * 1000
            );
        }
    }

    let elapsed = start.elapsed();
    metrics.print_report(elapsed);

    let total = metrics.total_requests.load(Ordering::Relaxed);
    assert_eq!(
        total, 500_000,
        "Should have processed 100k IPs × 5 requests"
    );
}

#[tokio::test]
#[ignore]
async fn stress_test_cache_thrashing() {
    println!("\n🔥 STRESS TEST: Cache Thrashing (alternating cache hit/miss)");

    let config = RateLimitConfig::new(1000, Duration::from_secs(60))
        .with_grace_period(0)
        .with_cache_refund_ratio(0.9);
    let server = Arc::new(create_test_server(config));
    let metrics = Arc::new(StressMetrics::new());

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // 1000 IPs making rapid requests, alternating between cache hit and miss
    for i in 0..1000 {
        let server = Arc::clone(&server);
        let metrics = Arc::clone(&metrics);
        let ip = format!("172.16.{}.{}", i / 256, i % 256);

        tasks.spawn(async move {
            for j in 0..100 {
                let req_start = Instant::now();
                let response = if j % 2 == 0 {
                    // Cache hit
                    server
                        .get("/cache")
                        .add_header("X-Forwarded-For", &ip)
                        .add_header("If-None-Match", "etag123")
                        .await
                } else {
                    // Cache miss
                    server
                        .get("/cache")
                        .add_header("X-Forwarded-For", &ip)
                        .await
                };
                let duration_us = req_start.elapsed().as_micros() as u64;
                metrics.record(response.status_code().as_u16(), duration_us);
            }
        });
    }

    while tasks.join_next().await.is_some() {}
    let elapsed = start.elapsed();

    metrics.print_report(elapsed);

    let total = metrics.total_requests.load(Ordering::Relaxed);
    assert_eq!(total, 100_000, "1000 IPs × 100 requests");
}

#[tokio::test]
#[ignore]
async fn stress_test_thundering_herd() {
    println!("\n🔥 STRESS TEST: Thundering Herd (50,000 simultaneous requests to same IP)");

    let config = RateLimitConfig::new(50, Duration::from_secs(60)).with_grace_period(0);
    let server = Arc::new(create_test_server(config));
    let metrics = Arc::new(StressMetrics::new());

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // All 50,000 requests use the SAME IP - maximum contention on DashMap entry
    for _ in 0..50_000 {
        let server = Arc::clone(&server);
        let metrics = Arc::clone(&metrics);

        tasks.spawn(async move {
            let req_start = Instant::now();
            let response = server
                .get("/")
                .add_header("X-Forwarded-For", "10.0.0.1")
                .await;
            let duration_us = req_start.elapsed().as_micros() as u64;
            metrics.record(response.status_code().as_u16(), duration_us);
        });
    }

    while tasks.join_next().await.is_some() {}
    let elapsed = start.elapsed();

    metrics.print_report(elapsed);

    let success = metrics.success_2xx.load(Ordering::Relaxed);
    let rate_limited = metrics.rate_limited_429.load(Ordering::Relaxed);

    assert_eq!(success, 50, "Only 50 requests should succeed");
    assert_eq!(rate_limited, 49_950, "Rest should be rate limited");
}

#[tokio::test]
#[ignore]
async fn stress_test_mixed_extreme_load() {
    println!("\n🔥 STRESS TEST: Mixed Extreme Load (simultaneous legitimate + attack traffic)");

    let config = RateLimitConfig::new(50, Duration::from_secs(60))
        .with_grace_period(0)
        .with_error_penalty(1.0);
    let server = Arc::new(create_test_server(config));
    let metrics = Arc::new(StressMetrics::new());

    let start = Instant::now();
    let mut tasks = JoinSet::new();

    // 5,000 legitimate users (10 req each)
    for i in 0..5_000 {
        let server = Arc::clone(&server);
        let metrics = Arc::clone(&metrics);
        let ip = format!("192.168.{}.{}", i / 256, i % 256);

        tasks.spawn(async move {
            for _ in 0..10 {
                let req_start = Instant::now();
                let response = server.get("/").add_header("X-Forwarded-For", &ip).await;
                let duration_us = req_start.elapsed().as_micros() as u64;
                metrics.record(response.status_code().as_u16(), duration_us);
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        });
    }

    // 1,000 attackers (200 req each, hitting 404s)
    for i in 0..1_000 {
        let server = Arc::clone(&server);
        let metrics = Arc::clone(&metrics);
        let ip = format!("203.0.{}.{}", i / 256, i % 256);

        tasks.spawn(async move {
            for _ in 0..200 {
                let req_start = Instant::now();
                let response = server
                    .get("/notfound")
                    .add_header("X-Forwarded-For", &ip)
                    .await;
                let duration_us = req_start.elapsed().as_micros() as u64;
                metrics.record(response.status_code().as_u16(), duration_us);
            }
        });
    }

    while tasks.join_next().await.is_some() {}
    let elapsed = start.elapsed();

    metrics.print_report(elapsed);

    let total = metrics.total_requests.load(Ordering::Relaxed);
    println!("\n📊 Breakdown:");
    println!("   Expected total: 250,000 requests");
    println!("   Actual total:   {} requests", total);
}
