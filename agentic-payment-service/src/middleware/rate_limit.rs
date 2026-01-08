use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

// Simple in-memory rate limiter
lazy_static::lazy_static! {
    static ref RATE_LIMITER: Arc<Mutex<RateLimiter>> = Arc::new(Mutex::new(RateLimiter::new()));
}

struct RateLimiter {
    requests: HashMap<String, Vec<Instant>>,
    window: Duration,
    max_requests: usize,
}

impl RateLimiter {
    fn new() -> Self {
        Self {
            requests: HashMap::new(),
            window: Duration::from_secs(60),
            max_requests: 60,
        }
    }

    fn check_rate_limit(&mut self, key: &str) -> bool {
        let now = Instant::now();
        let requests = self.requests.entry(key.to_string()).or_insert_with(Vec::new);

        // Remove old requests outside the window
        requests.retain(|&t| now.duration_since(t) < self.window);

        if requests.len() >= self.max_requests {
            return false;
        }

        requests.push(now);
        true
    }
}

pub async fn rate_limit_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Skip rate limiting for health check
    if request.uri().path() == "/health" {
        return Ok(next.run(request).await);
    }

    // Use IP address as rate limit key
    let key = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    let mut limiter = RATE_LIMITER.lock().await;
    
    if !limiter.check_rate_limit(&key) {
        tracing::warn!("Rate limit exceeded for: {}", key);
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    drop(limiter);
    Ok(next.run(request).await)
}