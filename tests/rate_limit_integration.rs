use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::{from_fn, from_fn_with_state},
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use basic_axum_rate_limit::{
    rate_limit_middleware, security_context_middleware, NoOpOnBlocked, RateLimitConfig, RateLimiter,
};
use std::time::Duration;
use tower::ServiceExt;

// Test handlers
async fn handler_ok() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

async fn handler_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "Not Found")
}

async fn handler_cache(req: Request<Body>) -> impl IntoResponse {
    // Check for If-None-Match header
    if req.headers().contains_key("if-none-match") {
        (StatusCode::NOT_MODIFIED, "")
    } else {
        (StatusCode::OK, "Content")
    }
}

// Helper to create test app
fn create_test_app(config: RateLimitConfig) -> Router {
    let rate_limiter = RateLimiter::new(config, NoOpOnBlocked);

    Router::new()
        .route("/", get(handler_ok))
        .route("/notfound", get(handler_not_found))
        .route("/cache", get(handler_cache))
        .layer(from_fn_with_state(rate_limiter, rate_limit_middleware))
        .layer(from_fn(security_context_middleware))
}

// Helper to send request with custom IP via X-Forwarded-For
async fn send_request_with_ip(app: &Router, path: &str, ip: &str) -> Response {
    let request = Request::builder()
        .uri(path)
        .header("X-Forwarded-For", ip)
        .body(Body::empty())
        .unwrap();

    let service = app.clone().into_service();

    service.oneshot(request).await.unwrap()
}

// Helper to send request with cache header
async fn send_cache_request(app: &Router, ip: &str, etag: &str) -> Response {
    let request = Request::builder()
        .uri("/cache")
        .header("X-Forwarded-For", ip)
        .header("If-None-Match", etag)
        .body(Body::empty())
        .unwrap();

    let service = app.clone().into_service();
    service.oneshot(request).await.unwrap()
}

#[tokio::test]
async fn test_basic_rate_limiting() {
    let config = RateLimitConfig::new(5, Duration::from_secs(60)).with_grace_period(0); // Disable grace period
    let app = create_test_app(config);

    let test_ip = "10.0.0.1";

    // First 5 requests should succeed
    for i in 1..=5 {
        let response = send_request_with_ip(&app, "/", test_ip).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} should succeed",
            i
        );
    }

    // 6th request should be rate limited
    let response = send_request_with_ip(&app, "/", test_ip).await;
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "6th request should be rate limited"
    );
}

#[tokio::test]
async fn test_grace_period_allows_burst() {
    let config = RateLimitConfig::new(10, Duration::from_secs(60)).with_grace_period(2);
    let app = create_test_app(config);

    let test_ip = "10.0.0.2";

    // Send 20 requests rapidly (should all succeed during grace period)
    for i in 1..=20 {
        let response = send_request_with_ip(&app, "/", test_ip).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} should succeed during grace period",
            i
        );
    }

    // Wait for grace period to expire
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Now normal rate limiting applies
    for i in 1..=10 {
        let response = send_request_with_ip(&app, "/", test_ip).await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Request {} after grace should succeed",
            i
        );
    }

    // 11th should be blocked
    let response = send_request_with_ip(&app, "/", test_ip).await;
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Should be rate limited after grace period"
    );
}

#[tokio::test]
async fn test_cache_response_refund() {
    let config = RateLimitConfig::new(10, Duration::from_secs(60))
        .with_grace_period(0)
        .with_cache_refund_ratio(0.9);
    let app = create_test_app(config);

    let test_ip = "10.0.0.3";

    // First request without cache header (costs 1.0 token)
    let response = send_request_with_ip(&app, "/cache", test_ip).await;
    assert_eq!(response.status(), StatusCode::OK);

    // Next 90 requests with cache header (304 responses, 0.1 tokens each = 9 tokens total)
    // Total: 1 + 9 = 10 tokens
    for i in 1..=90 {
        let response = send_cache_request(&app, test_ip, "etag123").await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_MODIFIED,
            "Cache request {} should return 304",
            i
        );
    }

    // Should still have capacity (used exactly 10 tokens)
    // But next regular request should be blocked
    let response = send_request_with_ip(&app, "/", test_ip).await;
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Should be rate limited after consuming all tokens"
    );
}

#[tokio::test]
async fn test_error_penalty() {
    let config = RateLimitConfig::new(10, Duration::from_secs(60))
        .with_grace_period(0)
        .with_error_penalty(1.0);
    let app = create_test_app(config);

    let test_ip = "10.0.0.4";

    // 5 404 errors should consume 10 tokens (5 * 2.0)
    for i in 1..=5 {
        let response = send_request_with_ip(&app, "/notfound", test_ip).await;
        assert_eq!(
            response.status(),
            StatusCode::NOT_FOUND,
            "Request {} should return 404",
            i
        );
    }

    // Next request should be rate limited
    let response = send_request_with_ip(&app, "/", test_ip).await;
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Should be rate limited after error penalties"
    );
}

#[tokio::test]
async fn test_different_ips_independent() {
    let config = RateLimitConfig::new(3, Duration::from_secs(60)).with_grace_period(0);
    let app = create_test_app(config);

    // Exhaust quota for first IP
    for _ in 1..=3 {
        send_request_with_ip(&app, "/", "10.0.0.10").await;
    }

    // First IP should be blocked
    let response = send_request_with_ip(&app, "/", "10.0.0.10").await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Second IP should still work
    for i in 1..=3 {
        let response = send_request_with_ip(&app, "/", "10.0.0.11").await;
        assert_eq!(
            response.status(),
            StatusCode::OK,
            "Second IP request {} should succeed",
            i
        );
    }
}

#[tokio::test]
async fn test_mixed_traffic_pattern() {
    let config = RateLimitConfig::new(20, Duration::from_secs(60))
        .with_grace_period(0)
        .with_cache_refund_ratio(0.9)
        .with_error_penalty(1.0);
    let app = create_test_app(config);

    let test_ip = "10.0.0.20";

    // Mixed pattern:
    // 10 successful requests (10 tokens)
    for _ in 1..=10 {
        let response = send_request_with_ip(&app, "/", test_ip).await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    // 5 cache hits (0.5 tokens total)
    for _ in 1..=5 {
        let response = send_cache_request(&app, test_ip, "etag").await;
        assert_eq!(response.status(), StatusCode::NOT_MODIFIED);
    }

    // 5 errors (10 tokens)
    for _ in 1..=5 {
        let response = send_request_with_ip(&app, "/notfound", test_ip).await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    // Total: 10 + 0.5 + 10 = 20.5 tokens (should be blocked)
    let response = send_request_with_ip(&app, "/", test_ip).await;
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Should be rate limited after mixed traffic"
    );
}

#[tokio::test]
async fn test_x_forwarded_for_parsing() {
    let config = RateLimitConfig::new(2, Duration::from_secs(60)).with_grace_period(0);
    let app = create_test_app(config);

    // Test with single IP
    send_request_with_ip(&app, "/", "192.168.1.1").await;
    send_request_with_ip(&app, "/", "192.168.1.1").await;
    let response = send_request_with_ip(&app, "/", "192.168.1.1").await;
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    // Test with multiple IPs (comma-separated, should use first)
    send_request_with_ip(&app, "/", "10.0.1.1, 10.0.1.2").await;
    send_request_with_ip(&app, "/", "10.0.1.1, 10.0.1.3").await;
    let response = send_request_with_ip(&app, "/", "10.0.1.1, 10.0.1.4").await;
    assert_eq!(
        response.status(),
        StatusCode::TOO_MANY_REQUESTS,
        "Should treat comma-separated IPs as same client"
    );
}
