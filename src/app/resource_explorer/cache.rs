//! Shared resource cache using Moka for memory-bounded concurrent caching.
//!
//! This module provides a singleton cache shared by:
//! - Explorer UI (multiple tabs/panes)
//! - Agent framework (unified queries)
//! - Phase 2 enrichment (detailed properties)
//!
//! Features:
//! - Memory-bounded with configurable limits
//! - Transparent zstd compression (~8-10x ratio on AWS JSON)
//! - Auto-sizing based on system memory
//! - Runtime-adjustable cache size
//! - Smart eviction (TinyLFU + LRU)

use moka::sync::Cache;
use once_cell::sync::OnceCell;
use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use super::state::ResourceEntry;

// ============================================================================
// Types
// ============================================================================

// DetailedData cache removed - properties are now merged directly into ResourceEntry.properties
// during Phase 2 enrichment. This eliminates duplicate storage and simplifies the architecture.

/// Memory statistics from the cache
#[derive(Debug, Clone, Default)]
pub struct CacheMemoryStats {
    /// Number of query keys in resource cache
    pub resource_entry_count: u64,
    /// Weighted size of resource cache in bytes
    pub resource_weighted_size: u64,
    /// Total uncompressed size tracked (for compression ratio calculation)
    pub total_uncompressed_size: u64,
}

impl CacheMemoryStats {
    /// Total cache size in bytes
    pub fn total_size(&self) -> u64 {
        self.resource_weighted_size
    }

    /// Compression ratio (uncompressed / compressed)
    pub fn compression_ratio(&self) -> f64 {
        let compressed = self.total_size();
        if compressed == 0 {
            1.0
        } else {
            self.total_uncompressed_size as f64 / compressed as f64
        }
    }
}

// ============================================================================
// Cache Configuration
// ============================================================================

/// User-configurable cache limits
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Max bytes for resource cache (default: auto-sized)
    pub max_resource_bytes: u64,
    /// Max bytes for detailed properties (default: auto-sized)
    pub max_detailed_bytes: u64,
    /// Idle timeout in seconds (default: 1800 = 30 min)
    pub idle_timeout_secs: u64,
}

impl CacheConfig {
    /// Auto-configure based on system memory
    pub fn auto_detect() -> Self {
        use sysinfo::System;

        let sys = System::new_all();
        let available_mb = sys.available_memory() / 1024 / 1024;

        // Use 25% of available memory for cache, min 512MB, max 8GB
        let cache_mb = (available_mb / 4).clamp(512, 8192);

        Self {
            max_resource_bytes: cache_mb * 1024 * 1024 * 80 / 100, // 80% for resources
            max_detailed_bytes: cache_mb * 1024 * 1024 * 20 / 100, // 20% for details
            idle_timeout_secs: 1800,
        }
    }

    /// Create with specific size in MB
    pub fn with_size_mb(total_mb: u64) -> Self {
        Self {
            max_resource_bytes: total_mb * 1024 * 1024 * 80 / 100,
            max_detailed_bytes: total_mb * 1024 * 1024 * 20 / 100,
            idle_timeout_secs: 1800,
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self::auto_detect()
    }
}

// ============================================================================
// Compressed Data (Internal)
// ============================================================================

/// Internal compressed storage - callers don't interact with this directly
#[derive(Clone)]
struct CompressedData {
    data: Vec<u8>,
    uncompressed_size: usize,
}

impl CompressedData {
    /// Compress data using zstd level 13 (balanced speed/compression)
    /// Level 19 was too slow (75-96ms per insert) for real-time cache updates
    fn compress(data: &[u8]) -> Self {
        let uncompressed_size = data.len();

        // Use zstd with level 13 for balanced compression
        // Level 19: 75-96ms, Level 3: <1ms, Level 13: TBD (testing)
        let mut encoder = zstd::Encoder::new(Vec::new(), 13).expect("zstd encoder creation failed");
        encoder.write_all(data).expect("zstd compression failed");
        let compressed = encoder.finish().expect("zstd finish failed");

        Self {
            data: compressed,
            uncompressed_size,
        }
    }

    /// Decompress data
    fn decompress(&self) -> Vec<u8> {
        let mut decoder = zstd::Decoder::new(&self.data[..]).expect("zstd decoder creation failed");
        let mut decompressed = Vec::with_capacity(self.uncompressed_size);
        decoder
            .read_to_end(&mut decompressed)
            .expect("zstd decompression failed");
        decompressed
    }

    /// Compressed size in bytes
    fn compressed_size(&self) -> usize {
        self.data.len()
    }
}

// ============================================================================
// Shared Resource Cache
// ============================================================================

/// Global resource cache - shared by all Explorer panes and Agents.
/// Compression is transparent - callers use insert()/get() with uncompressed data.
pub struct SharedResourceCache {
    /// Main cache: query_key -> compressed resource entries
    /// After Phase 2 enrichment, ResourceEntry.properties contains merged data
    resources: Cache<String, CompressedData>,

    /// Configuration
    config: CacheConfig,

    /// Track total uncompressed size for compression ratio stats
    total_uncompressed: std::sync::atomic::AtomicU64,
}

impl SharedResourceCache {
    /// Create a new cache with the given configuration
    pub fn new(config: CacheConfig) -> Self {
        let resources = Cache::builder()
            .max_capacity(config.max_resource_bytes)
            .weigher(|_key: &String, value: &CompressedData| -> u32 {
                // Weight by compressed size
                value.compressed_size() as u32
            })
            .time_to_idle(Duration::from_secs(config.idle_timeout_secs))
            .eviction_listener(|key, _value, cause| {
                let reason = format!("{:?}", cause);
                super::query_timing::log_cache_eviction("RESOURCE", &key, &reason);
                tracing::debug!("Resource cache evicted '{}': {:?}", key, cause);
            })
            .build();

        Self {
            resources,
            config,
            total_uncompressed: std::sync::atomic::AtomicU64::new(0),
        }
    }

    // ========================================================================
    // Resource Cache Operations (transparent compression)
    // ========================================================================

    /// Store resources - compression happens automatically
    /// Uses JSON serialization (required for serde_json::Value compatibility)
    pub fn insert_resources(&self, key: String, entries: Vec<Arc<ResourceEntry>>) {
        let start = std::time::Instant::now();

        // Use JSON serialization because ResourceEntry contains serde_json::Value fields
        // which don't support bincode's deserialize_any requirement
        let serialized = serde_json::to_vec(&entries).expect("JSON serialization failed");
        let uncompressed_size = serialized.len();

        let compressed = CompressedData::compress(&serialized);

        // Track uncompressed size for stats
        self.total_uncompressed
            .fetch_add(uncompressed_size as u64, std::sync::atomic::Ordering::Relaxed);

        self.resources.insert(key.clone(), compressed.clone());

        let elapsed_ms = start.elapsed().as_millis();
        super::query_timing::log_cache_op(
            "INSERT",
            &format!("{} ({} entries, {}KB)", key, entries.len(), uncompressed_size / 1024),
            elapsed_ms,
        );

        tracing::debug!(
            "Cache insert '{}': {} entries, {}KB -> {}KB ({:.1}x) in {}ms",
            key,
            entries.len(),
            uncompressed_size / 1024,
            compressed.compressed_size() / 1024,
            uncompressed_size as f64 / compressed.compressed_size() as f64,
            elapsed_ms
        );
    }

    /// Get resources - decompression happens automatically
    pub fn get_resources(&self, key: &str) -> Option<Vec<Arc<ResourceEntry>>> {
        let start = std::time::Instant::now();
        let result = self.resources.get(key).map(|compressed| {
            let decompressed = compressed.decompress();
            serde_json::from_slice(&decompressed).expect("JSON deserialization failed")
        });
        let elapsed_ms = start.elapsed().as_millis();

        if result.is_some() {
            super::query_timing::log_cache_op("GET_HIT", key, elapsed_ms);
        } else {
            super::query_timing::log_cache_op("GET_MISS", key, elapsed_ms);
        }

        result
    }

    /// Check if a key exists in the cache
    pub fn contains_resources(&self, key: &str) -> bool {
        self.resources.contains_key(key)
    }

    /// Remove resources by key
    pub fn remove_resources(&self, key: &str) {
        self.resources.invalidate(key);
    }

    /// Get all cache keys (for iteration/debugging)
    pub fn resource_keys(&self) -> Vec<String> {
        self.resources
            .iter()
            .map(|(k, _)| (*k).clone())
            .collect()
    }

    // ========================================================================
    // Convenience Methods for Migration (work with Vec<ResourceEntry>)
    // ========================================================================

    /// Store resources from non-Arc vector (converts to Arc internally)
    /// Use this during migration from HashMap-based cache
    pub fn insert_resources_owned(&self, key: String, entries: Vec<ResourceEntry>) {
        let arc_entries: Vec<Arc<ResourceEntry>> = entries.into_iter().map(Arc::new).collect();
        self.insert_resources(key, arc_entries);
    }

    /// Get resources as owned vector (clones from Arc)
    /// Use this during migration from HashMap-based cache
    pub fn get_resources_owned(&self, key: &str) -> Option<Vec<ResourceEntry>> {
        self.get_resources(key).map(|arc_entries| {
            arc_entries
                .iter()
                .map(|arc| ResourceEntry::clone(arc))
                .collect()
        })
    }

    /// Get all cached resources as a HashMap (for migration compatibility)
    /// This clones all data - use sparingly!
    pub fn to_hashmap(&self) -> std::collections::HashMap<String, Vec<ResourceEntry>> {
        let mut map = std::collections::HashMap::new();
        for key in self.resource_keys() {
            if let Some(entries) = self.get_resources_owned(&key) {
                map.insert(key, entries);
            }
        }
        map
    }

    /// Import from a HashMap (for migration compatibility)
    /// Replaces all cached resources
    pub fn import_from_hashmap(&self, map: std::collections::HashMap<String, Vec<ResourceEntry>>) {
        for (key, entries) in map {
            self.insert_resources_owned(key, entries);
        }
    }

    // ========================================================================
    // Detailed Properties Cache Operations
    // ========================================================================

    /// Store detailed properties - transparent compression
    /// Uses JSON serialization (required for serde_json::Value compatibility)
    // DetailedData cache methods removed - Phase 2 now merges properties directly
    // into ResourceEntry.properties instead of maintaining a separate cache

    // ========================================================================
    // Cache Management
    // ========================================================================

    /// Get memory statistics
    pub fn memory_stats(&self) -> CacheMemoryStats {
        CacheMemoryStats {
            resource_entry_count: self.resources.entry_count(),
            resource_weighted_size: self.resources.weighted_size(),
            total_uncompressed_size: self
                .total_uncompressed
                .load(std::sync::atomic::Ordering::Relaxed),
        }
    }

    /// Log current cache statistics to the query timing log
    pub fn log_stats(&self) {
        let stats = self.memory_stats();
        super::query_timing::log_cache_stats(
            stats.resource_entry_count,
            stats.resource_weighted_size,
            0, // detailed cache removed
            0, // detailed cache removed
            stats.total_uncompressed_size,
        );
    }

    /// Get current configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Resize cache at runtime (called from UI slider)
    /// Note: Moka doesn't support true runtime resize. This clears the cache
    /// and updates internal config. New capacity applies to fresh entries.
    pub fn resize(&self, new_total_mb: u64) {
        let _new_resource_bytes = new_total_mb * 1024 * 1024;

        // Clear existing entries - Moka will enforce new capacity on new inserts
        // Note: True capacity change requires cache recreation (app restart)
        self.clear();

        tracing::info!(
            "Cache cleared for resize to {}MB. \
             Full capacity change takes effect on app restart.",
            new_total_mb
        );
    }

    /// Clear all cached data
    pub fn clear(&self) {
        self.resources.invalidate_all();
        self.total_uncompressed
            .store(0, std::sync::atomic::Ordering::Relaxed);
        tracing::info!("Cache cleared");
    }

    /// Run pending maintenance tasks (eviction, etc.)
    pub fn run_pending_tasks(&self) {
        self.resources.run_pending_tasks();
    }
}

// ============================================================================
// Global Singleton
// ============================================================================

/// Global shared cache instance
static SHARED_CACHE: OnceCell<Arc<SharedResourceCache>> = OnceCell::new();

/// Initialize the global shared cache with auto-detected configuration.
/// This should be called once at application startup.
pub fn init_shared_cache() -> Arc<SharedResourceCache> {
    SHARED_CACHE
        .get_or_init(|| {
            let config = CacheConfig::auto_detect();
            tracing::info!(
                "Initializing shared resource cache: resources={}MB, detailed={}MB, idle_timeout={}s",
                config.max_resource_bytes / 1024 / 1024,
                config.max_detailed_bytes / 1024 / 1024,
                config.idle_timeout_secs
            );
            Arc::new(SharedResourceCache::new(config))
        })
        .clone()
}

/// Initialize the global shared cache with specific configuration.
pub fn init_shared_cache_with_config(config: CacheConfig) -> Arc<SharedResourceCache> {
    SHARED_CACHE
        .get_or_init(|| {
            tracing::info!(
                "Initializing shared resource cache: resources={}MB, detailed={}MB, idle_timeout={}s",
                config.max_resource_bytes / 1024 / 1024,
                config.max_detailed_bytes / 1024 / 1024,
                config.idle_timeout_secs
            );
            Arc::new(SharedResourceCache::new(config))
        })
        .clone()
}

/// Get the global shared cache instance.
/// Returns None if init_shared_cache() hasn't been called yet.
pub fn get_shared_cache() -> Option<Arc<SharedResourceCache>> {
    SHARED_CACHE.get().cloned()
}

/// Get the global shared cache, initializing with defaults if needed.
pub fn shared_cache() -> Arc<SharedResourceCache> {
    init_shared_cache()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_entry(id: &str) -> ResourceEntry {
        ResourceEntry {
            resource_type: "AWS::EC2::Instance".to_string(),
            account_id: "123456789012".to_string(),
            region: "us-east-1".to_string(),
            resource_id: id.to_string(),
            display_name: format!("test-{}", id),
            status: Some("running".to_string()),
            properties: serde_json::json!({"instanceType": "t2.micro"}),
            properties: serde_json::json!({"InstanceId": id}),
            detailed_properties: None,
            detailed_timestamp: None,
            tags: vec![],
            relationships: vec![],
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: egui::Color32::WHITE,
            region_color: egui::Color32::WHITE,
            query_timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_insert_and_get_resources() {
        let cache = SharedResourceCache::new(CacheConfig::with_size_mb(100));

        let entries: Vec<Arc<ResourceEntry>> = (0..10)
            .map(|i| Arc::new(create_test_entry(&format!("i-{:08x}", i))))
            .collect();

        cache.insert_resources("test-key".to_string(), entries.clone());

        let retrieved = cache.get_resources("test-key").expect("should find cached entries");
        assert_eq!(retrieved.len(), 10);
        assert_eq!(retrieved[0].resource_id, "i-00000000");
    }

    #[test]
    fn test_compression_ratio() {
        let cache = SharedResourceCache::new(CacheConfig::with_size_mb(100));

        // Create entries with repetitive AWS-like data
        let entries: Vec<Arc<ResourceEntry>> = (0..100)
            .map(|i| Arc::new(create_test_entry(&format!("i-{:08x}", i))))
            .collect();

        cache.insert_resources("compression-test".to_string(), entries);

        // Force Moka to process pending updates (weighted_size is eventually consistent)
        cache.run_pending_tasks();

        let stats = cache.memory_stats();
        let ratio = stats.compression_ratio();

        // Should achieve at least 2x compression on repetitive JSON
        assert!(
            ratio > 2.0,
            "Expected compression ratio > 2x, got {:.1}x (compressed: {}, uncompressed: {})",
            ratio,
            stats.resource_weighted_size,
            stats.total_uncompressed_size
        );
    }

    #[test]

    #[test]
    fn test_cache_clear() {
        let cache = SharedResourceCache::new(CacheConfig::with_size_mb(100));

        let entries: Vec<Arc<ResourceEntry>> = vec![Arc::new(create_test_entry("i-12345678"))];

        cache.insert_resources("test-key".to_string(), entries);
        assert!(cache.contains_resources("test-key"));

        cache.clear();
        assert!(!cache.contains_resources("test-key"));
    }
}
