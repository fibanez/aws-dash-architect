use super::state::ResourceTag;
use chrono::{DateTime, Utc};
use moka::sync::Cache;
use std::time::Duration;

/// Thread-safe cache for resource tags with TTL and LRU eviction
///
/// This cache minimizes AWS API calls by storing tags with a configurable TTL.
/// Uses Moka for lock-free concurrent access and automatic cache management.
///
/// # Features
///
/// - **Lock-free concurrency**: Moka's concurrent HashMap - no deadlocks possible
/// - **TTL-based expiration**: Automatic time-to-live eviction (default: 15 minutes)
/// - **Smart eviction**: TinyLFU + LRU algorithm for optimal cache hit rates
/// - **Automatic cleanup**: No manual cleanup needed - Moka handles everything
/// - **High performance**: Optimized for concurrent access (tested with 126+ simultaneous writes)
///
/// # Example
///
/// ```rust,ignore
/// use tag_cache::TagCache;
///
/// let cache = TagCache::new();
///
/// // Store tags (safe for concurrent access)
/// cache.set("ec2:instance", "i-123", "123456", "us-east-1", tags).await;
///
/// // Retrieve tags (returns None if stale or missing)
/// if let Some(tags) = cache.get("ec2:instance", "i-123", "123456", "us-east-1").await {
///     println!("Cache hit! Found {} tags", tags.len());
/// }
///
/// // Get statistics
/// let stats = cache.get_stats().await;
/// println!("Total entries: {}", stats.total_entries);
/// ```
pub struct TagCache {
    /// Moka cache handles TTL, eviction, and concurrency automatically
    cache: Cache<String, CachedEntry>,
    _ttl_minutes: i64,
}

/// A cached entry containing tags and metadata
#[derive(Clone)]
struct CachedEntry {
    tags: Vec<ResourceTag>,
    _timestamp: DateTime<Utc>,
    // Note: Moka tracks last_accessed automatically for LRU
}

/// Statistics about cache performance
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub total_entries: usize,
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate cache hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl TagCache {
    /// Default TTL in minutes
    pub const DEFAULT_TTL_MINUTES: i64 = 15;

    /// Default maximum cache entries
    pub const DEFAULT_MAX_ENTRIES: usize = 10_000;

    /// Create a new tag cache with default settings
    ///
    /// Default TTL: 15 minutes
    /// Default max entries: 10,000
    pub fn new() -> Self {
        Self::with_config(Self::DEFAULT_TTL_MINUTES, Self::DEFAULT_MAX_ENTRIES)
    }

    /// Create a new tag cache with custom configuration
    ///
    /// # Arguments
    ///
    /// * `ttl_minutes` - Time-to-live for cached entries in minutes
    /// * `max_entries` - Maximum number of entries before eviction
    pub fn with_config(ttl_minutes: i64, max_entries: usize) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_entries as u64)
            .time_to_live(Duration::from_secs((ttl_minutes * 60) as u64))
            .time_to_idle(Duration::from_secs((ttl_minutes * 60) as u64))
            .build();

        Self {
            cache,
            _ttl_minutes: ttl_minutes,
        }
    }

    /// Generate a cache key from resource identifiers
    ///
    /// Format: `{resource_type}:{account}:{region}:{resource_id}`
    fn make_key(resource_type: &str, resource_id: &str, account: &str, region: &str) -> String {
        format!("{}:{}:{}:{}", resource_type, account, region, resource_id)
    }

    /// Get cached tags for a resource
    ///
    /// Returns `Some(tags)` if the entry exists and is fresh (within TTL),
    /// or `None` if the entry is missing or expired.
    ///
    /// Moka automatically handles TTL checking and LRU tracking.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - AWS resource type (e.g., "AWS::EC2::Instance")
    /// * `resource_id` - Resource identifier (e.g., "i-1234567890abcdef0")
    /// * `account` - AWS account ID
    /// * `region` - AWS region
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(tags) = cache.get("AWS::EC2::Instance", "i-123", "123456", "us-east-1").await {
    ///     println!("Found {} tags in cache", tags.len());
    /// } else {
    ///     println!("Cache miss - need to fetch from AWS");
    /// }
    /// ```
    pub async fn get(
        &self,
        resource_type: &str,
        resource_id: &str,
        account: &str,
        region: &str,
    ) -> Option<Vec<ResourceTag>> {
        let key = Self::make_key(resource_type, resource_id, account, region);

        // Moka handles TTL, LRU, and concurrency automatically
        if let Some(entry) = self.cache.get(&key) {
            tracing::debug!("Tag cache HIT for key: {}", key);
            Some(entry.tags)
        } else {
            tracing::debug!("Tag cache MISS for key: {}", key);
            None
        }
    }

    /// Store tags in the cache
    ///
    /// This method stores tags with the current timestamp. Moka automatically
    /// handles eviction when capacity is reached using TinyLFU + LRU algorithm.
    ///
    /// Safe for concurrent access - no locks, no deadlocks possible.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - AWS resource type
    /// * `resource_id` - Resource identifier
    /// * `account` - AWS account ID
    /// * `region` - AWS region
    /// * `tags` - Vector of tags to cache
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let tags = vec![
    ///     ResourceTag { key: "Environment".into(), value: "Production".into() },
    ///     ResourceTag { key: "Team".into(), value: "Platform".into() },
    /// ];
    /// cache.set("AWS::EC2::Instance", "i-123", "123456", "us-east-1", tags).await;
    /// ```
    pub async fn set(
        &self,
        resource_type: &str,
        resource_id: &str,
        account: &str,
        region: &str,
        tags: Vec<ResourceTag>,
    ) {
        let key = Self::make_key(resource_type, resource_id, account, region);
        let now = Utc::now();

        // Moka handles: TTL, eviction, concurrency automatically
        // NO LOCKS - safe for 126+ concurrent writes with zero deadlock risk
        self.cache.insert(
            key.clone(),
            CachedEntry {
                tags,
                _timestamp: now,
            },
        );

        tracing::debug!(
            "Tag cache SET for key: {} ({} entries)",
            key,
            self.cache.entry_count()
        );
    }

    /// Invalidate cached tags for a specific resource
    ///
    /// Use this when a resource is updated or deleted to ensure stale data
    /// isn't served from cache.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - AWS resource type
    /// * `resource_id` - Resource identifier
    /// * `account` - AWS account ID
    /// * `region` - AWS region
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // After updating a resource's tags via AWS API
    /// cache.invalidate("AWS::EC2::Instance", "i-123", "123456", "us-east-1").await;
    /// ```
    pub async fn invalidate(
        &self,
        resource_type: &str,
        resource_id: &str,
        account: &str,
        region: &str,
    ) {
        let key = Self::make_key(resource_type, resource_id, account, region);
        self.cache.invalidate(&key);
        tracing::debug!("Tag cache INVALIDATED: {}", key);
    }

    /// Invalidate all cached entries
    ///
    /// Use this when the user clicks "Refresh All" or when you want to
    /// force a complete cache refresh.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// // User clicked "Refresh All" button
    /// cache.invalidate_all().await;
    /// ```
    pub async fn invalidate_all(&self) {
        let count = self.cache.entry_count();
        self.cache.invalidate_all();
        tracing::info!("Tag cache INVALIDATED ALL ({} entries cleared)", count);
    }

    /// Get cache statistics
    ///
    /// Returns statistics about cache performance.
    ///
    /// Note: Moka's sync API doesn't expose hit/miss counters, so those fields
    /// will be 0. Use entry_count for monitoring cache size.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let stats = cache.get_stats().await;
    /// println!("Total entries: {}", stats.total_entries);
    /// ```
    pub async fn get_stats(&self) -> CacheStats {
        CacheStats {
            hits: 0,  // Moka sync API doesn't expose these
            misses: 0,
            total_entries: self.cache.entry_count() as usize,
            evictions: 0,
        }
    }

    /// Get the current number of cached entries
    pub async fn len(&self) -> usize {
        self.cache.entry_count() as usize
    }

    /// Check if the cache is empty
    pub async fn is_empty(&self) -> bool {
        self.cache.entry_count() == 0
    }
}

impl Default for TagCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_tags() -> Vec<ResourceTag> {
        vec![
            ResourceTag {
                key: "Environment".to_string(),
                value: "Production".to_string(),
            },
            ResourceTag {
                key: "Team".to_string(),
                value: "Platform".to_string(),
            },
        ]
    }

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let cache = TagCache::new();
        let tags = create_test_tags();

        // Initially empty
        assert!(cache.is_empty().await);

        // Set tags
        cache
            .set("AWS::EC2::Instance", "i-123", "123456", "us-east-1", tags.clone())
            .await;

        assert_eq!(cache.len().await, 1);

        // Get tags (should hit)
        let retrieved = cache
            .get("AWS::EC2::Instance", "i-123", "123456", "us-east-1")
            .await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = TagCache::new();

        // Get non-existent entry
        let result = cache
            .get("AWS::EC2::Instance", "i-999", "123456", "us-east-1")
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = TagCache::new();
        let tags = create_test_tags();

        // Set and verify
        cache
            .set("AWS::EC2::Instance", "i-123", "123456", "us-east-1", tags)
            .await;
        assert_eq!(cache.len().await, 1);

        // Invalidate
        cache
            .invalidate("AWS::EC2::Instance", "i-123", "123456", "us-east-1")
            .await;

        // Moka may not immediately report 0 due to async cleanup
        // Just verify we get a miss
        let result = cache
            .get("AWS::EC2::Instance", "i-123", "123456", "us-east-1")
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate_all() {
        let cache = TagCache::new();
        let tags = create_test_tags();

        // Add multiple entries
        cache
            .set("AWS::EC2::Instance", "i-123", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("AWS::EC2::Instance", "i-456", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("AWS::S3::Bucket", "my-bucket", "123456", "us-east-1", tags)
            .await;

        assert_eq!(cache.len().await, 3);

        // Invalidate all
        cache.invalidate_all().await;

        // Verify all entries are gone
        assert!(cache
            .get("AWS::EC2::Instance", "i-123", "123456", "us-east-1")
            .await
            .is_none());
        assert!(cache
            .get("AWS::EC2::Instance", "i-456", "123456", "us-east-1")
            .await
            .is_none());
        assert!(cache
            .get("AWS::S3::Bucket", "my-bucket", "123456", "us-east-1")
            .await
            .is_none());
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        // Create cache with very short TTL (1 second for testing)
        let cache = Cache::builder()
            .max_capacity(10_000)
            .time_to_live(Duration::from_secs(1))
            .build();

        let test_cache = TagCache {
            cache,
            _ttl_minutes: 1,
        };

        let tags = create_test_tags();

        // Set tags
        test_cache
            .set("AWS::EC2::Instance", "i-123", "123456", "us-east-1", tags)
            .await;

        // Should exist immediately
        assert!(test_cache
            .get("AWS::EC2::Instance", "i-123", "123456", "us-east-1")
            .await
            .is_some());

        // Wait for TTL to expire
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Should be expired now
        let result = test_cache
            .get("AWS::EC2::Instance", "i-123", "123456", "us-east-1")
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        // Create cache with small capacity
        let cache = TagCache::with_config(15, 3);
        let tags = create_test_tags();

        // Fill cache beyond capacity
        cache
            .set("AWS::EC2::Instance", "i-1", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("AWS::EC2::Instance", "i-2", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("AWS::EC2::Instance", "i-3", "123456", "us-east-1", tags.clone())
            .await;

        // Access i-1 and i-3 to boost their frequency
        cache
            .get("AWS::EC2::Instance", "i-1", "123456", "us-east-1")
            .await;
        cache
            .get("AWS::EC2::Instance", "i-3", "123456", "us-east-1")
            .await;

        // Add one more entry, Moka should evict based on TinyLFU
        cache
            .set("AWS::EC2::Instance", "i-4", "123456", "us-east-1", tags)
            .await;

        // Should be at or near capacity
        let size = cache.len().await;
        assert!(size <= 3, "Cache size should not exceed max_capacity");
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        // Test the deadlock fix - 126 concurrent writes
        // Old implementation with RwLock would deadlock after ~60-70 writes
        // Moka implementation handles all 126 concurrently without blocking
        let cache = std::sync::Arc::new(TagCache::new());
        let tags = create_test_tags();

        let mut handles = vec![];
        for i in 0..126 {
            let cache_clone = cache.clone();
            let tags_clone = tags.clone();
            let handle = tokio::spawn(async move {
                cache_clone
                    .set(
                        "AWS::Bedrock::Model",
                        &format!("model-{}", i),
                        "123456789012",
                        "us-east-1",
                        tags_clone,
                    )
                    .await;
            });
            handles.push(handle);
        }

        // Should complete within 2 seconds (old implementation would hang for 34+ seconds)
        let start = std::time::Instant::now();
        let result = tokio::time::timeout(
            Duration::from_secs(2),
            futures::future::join_all(handles),
        )
        .await;
        let elapsed = start.elapsed();

        // Primary assertion: No deadlock
        assert!(result.is_ok(), "Concurrent writes should not deadlock! Elapsed: {:?}", elapsed);

        // Secondary check: Should complete quickly
        assert!(elapsed < Duration::from_secs(1), "Should complete in <1s, took {:?}", elapsed);

        // Verify at least some entries were written
        let final_count = cache.len().await;
        assert!(
            final_count > 0,
            "Cache should contain entries after concurrent writes (got {})",
            final_count
        );
    }
}
