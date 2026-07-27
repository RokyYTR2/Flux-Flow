use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;

use rusqlite::Connection;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tracing::warn;

use crate::error::ApiError;
use crate::models::TeamDatabase;

const RATE_LIMIT_WINDOW_SECS: u64 = 60;
const RATE_LIMIT_CLEANUP_INTERVAL: usize = 100;

pub const RATE_LIMIT_MAX_JOIN_ATTEMPTS: u32 = 5;
pub const RATE_LIMIT_MAX_CREATE_ATTEMPTS: u32 = 3;

pub struct RateLimiter {
    attempts: HashMap<IpAddr, Vec<Instant>>,
    call_count: usize,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            attempts: HashMap::new(),
            call_count: 0,
        }
    }

    pub fn check(&mut self, ip: IpAddr, max_attempts: u32) -> Result<(), ApiError> {
        self.call_count += 1;
        if self.call_count % RATE_LIMIT_CLEANUP_INTERVAL == 0 {
            self.cleanup();
        }

        let now = Instant::now();
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        let entries = self.attempts.entry(ip).or_default();
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= max_attempts as usize {
            warn!(ip = %ip, "Rate limit exceeded");
            return Err(ApiError::TooManyRequests(format!(
                "Too many requests. Try again in {} seconds.",
                RATE_LIMIT_WINDOW_SECS
            )));
        }

        entries.push(now);
        Ok(())
    }

    fn cleanup(&mut self) {
        let now = Instant::now();
        let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);
        self.attempts.retain(|_, entries| {
            entries.retain(|t| now.duration_since(*t) < window);
            !entries.is_empty()
        });
    }
}

#[derive(Clone)]
pub struct AppState {
    pub conn: Arc<Mutex<Connection>>,
    pub db: Arc<Mutex<TeamDatabase>>,
    pub join_limiter: Arc<Mutex<RateLimiter>>,
    pub create_limiter: Arc<Mutex<RateLimiter>>,
}