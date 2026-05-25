//! 速率限制器

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration};

/// 速率限制配置
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// 最大请求数
    pub max_per_window: u32,
    /// 时间窗口（秒）
    pub window_secs: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_per_window: 10,
            window_secs: 86400, // 24 hours
        }
    }
}

/// 速率限制条目
#[derive(Debug, Clone)]
struct RateLimitEntry {
    count: u32,
    window_start: DateTime<Utc>,
}

/// 速率限制器
#[derive(Debug, Clone)]
pub struct RateLimiter {
    config: RateLimitConfig,
    entries: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
}

impl RateLimiter {
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 检查是否允许请求
    pub async fn check(&self, key: &str) -> bool {
        let mut entries = self.entries.write().await;
        let now = Utc::now();
        let window_start = now - Duration::seconds(self.config.window_secs as i64);

        let entry = entries.entry(key.to_string()).or_insert(RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // 重置过期窗口
        if entry.window_start < window_start {
            entry.count = 0;
            entry.window_start = now;
        }

        if entry.count < self.config.max_per_window {
            entry.count += 1;
            true
        } else {
            false
        }
    }

    /// 获取剩余请求数
    pub async fn remaining(&self, key: &str) -> u32 {
        let entries = self.entries.read().await;
        let now = Utc::now();
        let window_start = now - Duration::seconds(self.config.window_secs as i64);

        if let Some(entry) = entries.get(key) {
            if entry.window_start >= window_start {
                return self.config.max_per_window.saturating_sub(entry.count);
            }
        }

        self.config.max_per_window
    }

    /// 重置限制
    pub async fn reset(&self, key: &str) {
        let mut entries = self.entries.write().await;
        entries.remove(key);
    }

    /// 清理过期条目（定期调用）
    pub async fn cleanup(&self) {
        let mut entries = self.entries.write().await;
        let now = Utc::now();
        let window_start = now - Duration::seconds(self.config.window_secs as i64);

        entries.retain(|_, entry| entry.window_start >= window_start);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_per_window: 3,
            window_secs: 60,
        });

        // 前3次应该通过
        assert!(limiter.check("test_key").await);
        assert!(limiter.check("test_key").await);
        assert!(limiter.check("test_key").await);

        // 第4次应该被限制
        assert!(!limiter.check("test_key").await);

        // 不同key不受影响
        assert!(limiter.check("other_key").await);

        // 重置后应该可以通过
        limiter.reset("test_key").await;
        assert!(limiter.check("test_key").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_remaining() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_per_window: 5,
            window_secs: 60,
        });

        assert_eq!(limiter.remaining("key1").await, 5);
        limiter.check("key1").await;
        assert_eq!(limiter.remaining("key1").await, 4);
        limiter.check("key1").await;
        limiter.check("key1").await;
        assert_eq!(limiter.remaining("key1").await, 2);
    }

    #[tokio::test]
    async fn test_rate_limiter_remaining_unknown_key() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_per_window: 10,
            window_secs: 60,
        });

        assert_eq!(limiter.remaining("unknown_key").await, 10);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_per_window: 2,
            window_secs: 60,
        });

        assert!(limiter.check("key").await);
        assert!(limiter.check("key").await);
        assert!(!limiter.check("key").await);

        limiter.reset("key").await;
        assert!(limiter.check("key").await);
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup() {
        let limiter = RateLimiter::new(RateLimitConfig {
            max_per_window: 10,
            window_secs: 60,
        });

        limiter.check("key1").await;
        limiter.check("key2").await;

        limiter.cleanup().await;

        assert_eq!(limiter.remaining("key1").await, 9);
        assert_eq!(limiter.remaining("key2").await, 9);
    }
}
