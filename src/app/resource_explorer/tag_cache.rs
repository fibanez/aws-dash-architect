use super::state::ResourceTag;
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Thread-safe cache for resource tags with TTL and LRU eviction
///
/// This cache minimizes AWS API calls by storing tags with a configurable TTL.
/// It's designed to handle thousands of resources efficiently while staying
/// within memory limits.
///
/// # Features
///
/// - **Thread-safe**: Uses `Arc<RwLock>` for concurrent access
/// - **TTL-based expiration**: Configurable time-to-live (default: 15 minutes)
/// - **LRU eviction**: Automatically removes least-recently-used entries when limit reached
/// - **Statistics tracking**: Monitor hit/miss rates and cache size
/// - **Automatic cleanup**: Removes stale entries periodically
///
/// # Example
///
/// ```rust,ignore
/// use tag_cache::TagCache;
///
/// let cache = TagCache::new();
///
/// // Store tags
/// cache.set("ec2:instance", "i-123", "123456", "us-east-1", tags).await;
///
/// // Retrieve tags (returns None if stale or missing)
/// if let Some(tags) = cache.get("ec2:instance", "i-123", "123456", "us-east-1").await {
///     println!("Cache hit! Found {} tags", tags.len());
/// }
///
/// // Get statistics
/// let stats = cache.get_stats().await;
/// println!("Hit rate: {:.1}%", stats.hit_rate() * 100.0);
/// ```
pub struct TagCache {
    cache: Arc<RwLock<HashMap<String, CachedEntry>>>,
    ttl_minutes: i64,
    max_entries: usize,
    stats: Arc<RwLock<CacheStats>>,
    insert_counter: Arc<RwLock<usize>>,
}

/// A cached entry containing tags and metadata
#[derive(Clone)]
struct CachedEntry {
    tags: Vec<ResourceTag>,
    timestamp: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
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

    /// Cleanup interval (run cleanup every N inserts)
    const CLEANUP_INTERVAL: usize = 100;

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
    /// * `max_entries` - Maximum number of entries before LRU eviction
    pub fn with_config(ttl_minutes: i64, max_entries: usize) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            ttl_minutes,
            max_entries,
            stats: Arc::new(RwLock::new(CacheStats::default())),
            insert_counter: Arc::new(RwLock::new(0)),
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
    /// or `None` if the entry is missing, stale, or expired.
    ///
    /// This method updates the last-accessed timestamp for LRU tracking.
    ///
    /// # Arguments
    ///
    /// * `resource_type` - AWS resource type (e.g., "ec2:instance")
    /// * `resource_id` - Resource identifier (e.g., "i-1234567890abcdef0")
    /// * `account` - AWS account ID
    /// * `region` - AWS region
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// if let Some(tags) = cache.get("ec2:instance", "i-123", "123456", "us-east-1").await {
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
        let now = Utc::now();

        let mut cache = self.cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get_mut(&key) {
            // Check if entry is still fresh
            let age = now - entry.timestamp;
            if age < Duration::minutes(self.ttl_minutes) {
                // Update last accessed time for LRU
                entry.last_accessed = now;
                stats.hits += 1;
                tracing::debug!("Tag cache HIT for key: {}", key);
                return Some(entry.tags.clone());
            } else {
                // Entry is stale, remove it
                cache.remove(&key);
                tracing::debug!("Tag cache STALE (age: {:?}) for key: {}", age, key);
            }
        }

        stats.misses += 1;
        tracing::debug!("Tag cache MISS for key: {}", key);
        None
    }

    /// Store tags in the cache
    ///
    /// This method stores tags with the current timestamp. If the cache is at
    /// capacity, it will evict the least-recently-used entry first.
    ///
    /// Automatic cleanup of stale entries runs every 100 inserts.
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
    /// cache.set("ec2:instance", "i-123", "123456", "us-east-1", tags).await;
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

        let mut cache = self.cache.write().await;

        // Check if we need to evict entries (LRU)
        if cache.len() >= self.max_entries && !cache.contains_key(&key) {
            self.evict_lru(&mut cache).await;
        }

        // Insert or update entry
        cache.insert(
            key.clone(),
            CachedEntry {
                tags,
                timestamp: now,
                last_accessed: now,
            },
        );

        tracing::debug!("Tag cache SET for key: {} ({} entries)", key, cache.len());

        // Update insert counter and maybe run cleanup
        let mut counter = self.insert_counter.write().await;
        *counter += 1;
        if *counter >= Self::CLEANUP_INTERVAL {
            *counter = 0;
            drop(cache); // Release lock before cleanup
            self.cleanup_stale_entries().await;
        }
    }

    /// Evict the least-recently-used entry from the cache
    ///
    /// This is called automatically when the cache reaches max capacity.
    async fn evict_lru(&self, cache: &mut HashMap<String, CachedEntry>) {
        if cache.is_empty() {
            return;
        }

        // Find the entry with the oldest last_accessed time
        let mut oldest_key: Option<String> = None;
        let mut oldest_time = Utc::now();

        for (key, entry) in cache.iter() {
            if entry.last_accessed < oldest_time {
                oldest_time = entry.last_accessed;
                oldest_key = Some(key.clone());
            }
        }

        // Remove the oldest entry
        if let Some(key) = oldest_key {
            cache.remove(&key);
            let mut stats = self.stats.write().await;
            stats.evictions += 1;
            tracing::debug!("Tag cache EVICTED (LRU): {}", key);
        }
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
    /// cache.invalidate("ec2:instance", "i-123", "123456", "us-east-1").await;
    /// ```
    pub async fn invalidate(
        &self,
        resource_type: &str,
        resource_id: &str,
        account: &str,
        region: &str,
    ) {
        let key = Self::make_key(resource_type, resource_id, account, region);
        let mut cache = self.cache.write().await;
        if cache.remove(&key).is_some() {
            tracing::debug!("Tag cache INVALIDATED: {}", key);
        }
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
        let mut cache = self.cache.write().await;
        let count = cache.len();
        cache.clear();
        tracing::info!("Tag cache INVALIDATED ALL ({} entries cleared)", count);
    }

    /// Remove all stale entries from the cache
    ///
    /// This is called automatically every 100 inserts, but can also be
    /// called manually for cache maintenance.
    ///
    /// Returns the number of entries removed.
    pub async fn cleanup_stale_entries(&self) -> usize {
        let now = Utc::now();
        let mut cache = self.cache.write().await;
        let ttl = Duration::minutes(self.ttl_minutes);

        let keys_to_remove: Vec<String> = cache
            .iter()
            .filter(|(_, entry)| now - entry.timestamp > ttl)
            .map(|(key, _)| key.clone())
            .collect();

        let count = keys_to_remove.len();
        for key in keys_to_remove {
            cache.remove(&key);
        }

        if count > 0 {
            tracing::debug!("Tag cache CLEANUP: removed {} stale entries", count);
        }

        count
    }

    /// Get cache statistics
    ///
    /// Returns statistics about cache performance including hit rate,
    /// total entries, and eviction count.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let stats = cache.get_stats().await;
    /// println!("Cache hit rate: {:.1}%", stats.hit_rate() * 100.0);
    /// println!("Total entries: {}", stats.total_entries);
    /// println!("Evictions: {}", stats.evictions);
    /// ```
    pub async fn get_stats(&self) -> CacheStats {
        let cache = self.cache.read().await;
        let mut stats = self.stats.read().await.clone();
        stats.total_entries = cache.len();
        stats
    }

    /// Get the current number of cached entries
    pub async fn len(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
    }

    /// Check if the cache is empty
    pub async fn is_empty(&self) -> bool {
        let cache = self.cache.read().await;
        cache.is_empty()
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
            .set("ec2:instance", "i-123", "123456", "us-east-1", tags.clone())
            .await;

        assert_eq!(cache.len().await, 1);

        // Get tags (should hit)
        let retrieved = cache
            .get("ec2:instance", "i-123", "123456", "us-east-1")
            .await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().len(), 2);

        // Stats should show 1 hit
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
    }

    #[tokio::test]
    async fn test_cache_miss() {
        let cache = TagCache::new();

        // Get non-existent entry
        let result = cache
            .get("ec2:instance", "i-999", "123456", "us-east-1")
            .await;
        assert!(result.is_none());

        // Stats should show 1 miss
        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
    }

    #[tokio::test]
    async fn test_cache_invalidate() {
        let cache = TagCache::new();
        let tags = create_test_tags();

        // Set and verify
        cache
            .set("ec2:instance", "i-123", "123456", "us-east-1", tags)
            .await;
        assert_eq!(cache.len().await, 1);

        // Invalidate
        cache
            .invalidate("ec2:instance", "i-123", "123456", "us-east-1")
            .await;
        assert_eq!(cache.len().await, 0);

        // Should be a miss now
        let result = cache
            .get("ec2:instance", "i-123", "123456", "us-east-1")
            .await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_cache_invalidate_all() {
        let cache = TagCache::new();
        let tags = create_test_tags();

        // Add multiple entries
        cache
            .set("ec2:instance", "i-123", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("ec2:instance", "i-456", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("s3:bucket", "my-bucket", "123456", "us-east-1", tags)
            .await;

        assert_eq!(cache.len().await, 3);

        // Invalidate all
        cache.invalidate_all().await;
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        // Create cache with very short TTL (1 minute)
        let cache = TagCache::with_config(1, 10_000);
        let tags = create_test_tags();

        // Set tags
        cache
            .set("ec2:instance", "i-123", "123456", "us-east-1", tags)
            .await;

        // Manually expire the entry by accessing internal state
        // (In production, we'd wait for time to pass)
        {
            let mut cache_map = cache.cache.write().await;
            if let Some(entry) = cache_map.get_mut("ec2:instance:123456:us-east-1:i-123") {
                entry.timestamp = Utc::now() - Duration::minutes(2); // 2 minutes old
            }
        }

        // Should be a miss now (stale)
        let result = cache
            .get("ec2:instance", "i-123", "123456", "us-east-1")
            .await;
        assert!(result.is_none());

        // Entry should be removed
        assert_eq!(cache.len().await, 0);
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        // Create cache with small capacity
        let cache = TagCache::with_config(15, 3);
        let tags = create_test_tags();

        // Fill cache to capacity
        cache
            .set("ec2:instance", "i-1", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("ec2:instance", "i-2", "123456", "us-east-1", tags.clone())
            .await;
        cache
            .set("ec2:instance", "i-3", "123456", "us-east-1", tags.clone())
            .await;

        assert_eq!(cache.len().await, 3);

        // Access i-1 and i-3 to make i-2 the LRU
        cache
            .get("ec2:instance", "i-1", "123456", "us-east-1")
            .await;
        cache
            .get("ec2:instance", "i-3", "123456", "us-east-1")
            .await;

        // Add one more entry, should evict i-2 (least recently used)
        cache
            .set("ec2:instance", "i-4", "123456", "us-east-1", tags)
            .await;

        // Should still be 3 entries
        assert_eq!(cache.len().await, 3);

        // i-2 should be evicted
        let result = cache
            .get("ec2:instance", "i-2", "123456", "us-east-1")
            .await;
        assert!(result.is_none());

        // i-1, i-3, and i-4 should still exist
        assert!(cache
            .get("ec2:instance", "i-1", "123456", "us-east-1")
            .await
            .is_some());
        assert!(cache
            .get("ec2:instance", "i-3", "123456", "us-east-1")
            .await
            .is_some());
        assert!(cache
            .get("ec2:instance", "i-4", "123456", "us-east-1")
            .await
            .is_some());

        // Check eviction count
        let stats = cache.get_stats().await;
        assert_eq!(stats.evictions, 1);
    }

    #[tokio::test]
    async fn test_cache_hit_rate() {
        let cache = TagCache::new();
        let tags = create_test_tags();

        // 1 set, 2 hits, 1 miss
        cache
            .set("ec2:instance", "i-123", "123456", "us-east-1", tags)
            .await;
        cache
            .get("ec2:instance", "i-123", "123456", "us-east-1")
            .await; // hit
        cache
            .get("ec2:instance", "i-123", "123456", "us-east-1")
            .await; // hit
        cache
            .get("ec2:instance", "i-999", "123456", "us-east-1")
            .await; // miss

        let stats = cache.get_stats().await;
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01); // ~66.7%
    }
}
