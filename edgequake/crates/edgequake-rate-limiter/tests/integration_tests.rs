use edgequake_rate_limiter::{RateLimitConfig, RateLimiter};
use std::time::Duration;
use tokio::time;

#[tokio::test]
async fn test_basic_rate_limiting() {
    let config = RateLimitConfig::strict(5, 10); // 5 requests per 10 seconds, NO BURST
    let limiter = RateLimiter::new(config);

    // First 5 requests should succeed
    for i in 0..5 {
        let (allowed, retry_after) = limiter.check_rate_limit("test-tenant");
        assert!(allowed, "Request {} should be allowed", i);
        assert!(retry_after.is_none());
    }

    // 6th request should fail
    let (allowed, retry_after) = limiter.check_rate_limit("test-tenant");
    assert!(!allowed, "Request 6 should be blocked");
    assert!(retry_after.is_some());
    assert!(retry_after.unwrap() > 0);
}

#[tokio::test]
async fn test_token_refill_over_time() {
    let config = RateLimitConfig::strict(10, 5); // 10 requests per 5 seconds = 2 req/sec, NO BURST
    let limiter = RateLimiter::new(config);

    // Consume all tokens
    for _ in 0..10 {
        assert!(limiter.check_rate_limit("test-tenant").0);
    }

    // Should be blocked now
    assert!(!limiter.check_rate_limit("test-tenant").0);

    // Wait for 1 second - should get ~2 tokens back
    time::sleep(Duration::from_secs(1)).await;

    assert!(limiter.check_rate_limit("test-tenant").0);
    assert!(limiter.check_rate_limit("test-tenant").0);
    assert!(!limiter.check_rate_limit("test-tenant").0);
}

#[tokio::test]
async fn test_burst_allowance() {
    let config = RateLimitConfig {
        requests_per_window: 100,
        window_seconds: 60,
        burst_size: 50, // Allow 50 extra requests
        refill_rate: 100.0 / 60.0,
    };
    let limiter = RateLimiter::new(config);

    // Should allow 150 requests (100 + 50 burst)
    for i in 0..150 {
        let (allowed, _) = limiter.check_rate_limit("test-tenant");
        assert!(allowed, "Request {} should be allowed with burst", i);
    }

    // 151st request should fail
    let (allowed, _) = limiter.check_rate_limit("test-tenant");
    assert!(!allowed, "Request 151 should be blocked");
}

#[tokio::test]
async fn test_strict_rate_limit() {
    let config = RateLimitConfig::strict(10, 10); // No burst allowed
    let limiter = RateLimiter::new(config);

    // Should allow exactly 10 requests
    for i in 0..10 {
        assert!(
            limiter.check_rate_limit("test-tenant").0,
            "Request {} should be allowed",
            i
        );
    }

    // 11th request should fail immediately
    assert!(!limiter.check_rate_limit("test-tenant").0);
}

#[tokio::test]
async fn test_lenient_rate_limit() {
    let config = RateLimitConfig::lenient(100, 60); // 50% burst
    let limiter = RateLimiter::new(config);

    // Should allow 150 requests (100 + 50 burst)
    for i in 0..150 {
        assert!(
            limiter.check_rate_limit("test-tenant").0,
            "Request {} should be allowed",
            i
        );
    }

    assert!(!limiter.check_rate_limit("test-tenant").0);
}

#[tokio::test]
async fn test_tenant_isolation() {
    let config = RateLimitConfig::strict(5, 10); // NO BURST for predictable limits
    let limiter = RateLimiter::new(config);

    // Tenant A exhausts their quota
    for _ in 0..5 {
        assert!(limiter.check_rate_limit("tenant-a").0);
    }
    assert!(!limiter.check_rate_limit("tenant-a").0);

    // Tenant B should still have full quota
    for i in 0..5 {
        assert!(
            limiter.check_rate_limit("tenant-b").0,
            "Tenant B request {} should be allowed",
            i
        );
    }
    assert!(!limiter.check_rate_limit("tenant-b").0);

    // Tenant C should also have full quota
    for i in 0..5 {
        assert!(
            limiter.check_rate_limit("tenant-c").0,
            "Tenant C request {} should be allowed",
            i
        );
    }
}

#[tokio::test]
async fn test_workspace_isolation() {
    let config = RateLimitConfig::strict(5, 10); // NO BURST for predictable limits
    let limiter = RateLimiter::new(config);

    let tenant_id = "tenant-123";
    let workspace_a = format!("{}:workspace-a", tenant_id);
    let workspace_b = format!("{}:workspace-b", tenant_id);

    // Workspace A exhausts quota
    for _ in 0..5 {
        assert!(limiter.check_rate_limit(&workspace_a).0);
    }
    assert!(!limiter.check_rate_limit(&workspace_a).0);

    // Workspace B should have separate quota
    for i in 0..5 {
        assert!(
            limiter.check_rate_limit(&workspace_b).0,
            "Workspace B request {} should be allowed",
            i
        );
    }
}

#[tokio::test]
async fn test_custom_cost() {
    let config = RateLimitConfig::strict(100, 60); // 100 tokens per minute, NO BURST
    let limiter = RateLimiter::new(config);

    // Expensive operation costs 10 tokens
    for i in 0..10 {
        let (allowed, _) = limiter.check_rate_limit_with_cost("test-tenant", 10.0);
        assert!(allowed, "Expensive request {} should be allowed", i);
    }

    // Should have used all 100 tokens
    assert!(!limiter.check_rate_limit("test-tenant").0);
}

#[tokio::test]
async fn test_mixed_cost_operations() {
    let config = RateLimitConfig::strict(100, 60); // NO BURST for predictable limits
    let limiter = RateLimiter::new(config);

    // 5 cheap operations (1 token each) = 5 tokens
    for _ in 0..5 {
        assert!(limiter.check_rate_limit_with_cost("test-tenant", 1.0).0);
    }

    // 3 expensive operations (10 tokens each) = 30 tokens
    for _ in 0..3 {
        assert!(limiter.check_rate_limit_with_cost("test-tenant", 10.0).0);
    }

    // 1 very expensive operation (65 tokens)
    assert!(limiter.check_rate_limit_with_cost("test-tenant", 65.0).0);

    // Total used: 5 + 30 + 65 = 100 tokens
    // Next request should fail
    assert!(!limiter.check_rate_limit("test-tenant").0);
}

#[tokio::test]
async fn test_concurrent_requests() {
    let config = RateLimitConfig::new(100, 60);
    let limiter = RateLimiter::new(config);

    let mut handles = vec![];

    // Spawn 50 concurrent tasks, each making 2 requests
    for i in 0..50 {
        let limiter_clone = limiter.clone();
        let handle = tokio::spawn(async move {
            let tenant_key = format!("tenant-{}", i % 10); // 10 different tenants
            let result1 = limiter_clone.check_rate_limit(&tenant_key).0;
            let result2 = limiter_clone.check_rate_limit(&tenant_key).0;
            (result1, result2)
        });
        handles.push(handle);
    }

    // Wait for all tasks
    let mut success_count = 0;
    for handle in handles {
        let (r1, r2) = handle.await.unwrap();
        if r1 {
            success_count += 1;
        }
        if r2 {
            success_count += 1;
        }
    }

    // Each of 10 tenants should allow ~10 requests (100 total)
    // But some might be rejected due to timing
    assert!(success_count >= 90, "At least 90 requests should succeed");
}

#[tokio::test]
async fn test_reset_rate_limit() {
    let config = RateLimitConfig::strict(5, 10); // NO BURST for predictable limits
    let limiter = RateLimiter::new(config);

    // Exhaust quota
    for _ in 0..5 {
        assert!(limiter.check_rate_limit("test-tenant").0);
    }
    assert!(!limiter.check_rate_limit("test-tenant").0);

    // Reset the limit
    limiter.reset("test-tenant");

    // Should have full quota again
    for i in 0..5 {
        assert!(
            limiter.check_rate_limit("test-tenant").0,
            "Request {} after reset should be allowed",
            i
        );
    }
}

#[tokio::test]
async fn test_get_state() {
    let config = RateLimitConfig::strict(10, 10); // NO BURST for predictable state
    let limiter = RateLimiter::new(config);

    // Make some requests
    for _ in 0..3 {
        limiter.check_rate_limit("test-tenant");
    }

    // Check state
    let state = limiter.get_state("test-tenant").unwrap();
    assert_eq!(state.capacity, 10);
    assert!(state.available_tokens <= 7); // Might be slightly less due to timing
    assert!(state.refill_rate > 0.0);
}

#[tokio::test]
async fn test_cleanup_stale_buckets() {
    let config = RateLimitConfig::new(10, 60);
    let limiter = RateLimiter::new(config);

    // Create some buckets
    limiter.check_rate_limit("tenant-1");
    limiter.check_rate_limit("tenant-2");
    limiter.check_rate_limit("tenant-3");

    // Wait a bit
    time::sleep(Duration::from_millis(100)).await;

    // Cleanup with short max age (should remove all)
    limiter.cleanup_stale_buckets(Duration::from_millis(50));

    // State should be None for all tenants
    assert!(limiter.get_state("tenant-1").is_none());
    assert!(limiter.get_state("tenant-2").is_none());
    assert!(limiter.get_state("tenant-3").is_none());
}

#[tokio::test]
async fn test_background_cleanup_task() {
    let config = RateLimitConfig::new(10, 60);
    let limiter = RateLimiter::new(config);

    // Create a bucket
    limiter.check_rate_limit("test-tenant");
    assert!(limiter.get_state("test-tenant").is_some());

    // Start cleanup task
    limiter
        .clone()
        .start_cleanup_task(Duration::from_millis(100), Duration::from_millis(50));

    // Wait for cleanup to run
    time::sleep(Duration::from_millis(200)).await;

    // Bucket should be cleaned up
    assert!(limiter.get_state("test-tenant").is_none());
}

#[tokio::test]
async fn test_rapid_fire_requests() {
    let config = RateLimitConfig::strict(1000, 60); // 1000 requests per minute, NO BURST
    let limiter = RateLimiter::new(config);

    let start = std::time::Instant::now();

    // Make 1000 requests as fast as possible
    for i in 0..1000 {
        let (allowed, _) = limiter.check_rate_limit("test-tenant");
        assert!(allowed, "Request {} should be allowed", i);
    }

    let elapsed = start.elapsed();

    // Should complete very quickly (< 100ms)
    assert!(
        elapsed < Duration::from_millis(100),
        "1000 checks took {:?}, should be < 100ms",
        elapsed
    );

    // 1001st should fail
    assert!(!limiter.check_rate_limit("test-tenant").0);
}
