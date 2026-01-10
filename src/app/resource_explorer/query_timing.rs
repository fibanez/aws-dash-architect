//! Query timing logs for troubleshooting resource queries.
//!
//! This module provides a separate log file for query timing information
//! to help debug performance issues and contention in the query pipeline.
//!
//! Features:
//! - Tracks all expected queries vs completed queries
//! - Detects stuck/hanging queries (started but not finished)
//! - Logs anomalies like missing queries or timeouts
//! - Provides summary at end of each phase
//!
//! NOTE: This module is only active in debug builds. In release builds,
//! all functions are no-ops with zero overhead.

// ============================================================================
// Debug build implementation
// ============================================================================

#[cfg(debug_assertions)]
mod inner {
    use std::collections::{HashMap, HashSet};
    use std::fs::{File, OpenOptions};
    use std::io::Write;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Mutex;
    use std::time::Instant;

    use once_cell::sync::Lazy;

    /// Global query timing logger
    static QUERY_LOGGER: Lazy<Mutex<QueryTimingLogger>> =
        Lazy::new(|| Mutex::new(QueryTimingLogger::new()));

    // ========================================================================
    // In-Flight Tag Fetch Registry
    // ========================================================================
    // Tracks individual tag fetch operations to identify exactly which ones
    // are stuck, not just the count.

    /// Metadata for an in-flight tag fetch operation
    #[derive(Debug, Clone)]
    pub struct InFlightTagFetch {
        pub id: u64,
        pub service: String,
        pub resource_id: String,
        pub region: String,
        pub account: String,
        pub start_time: Instant,
    }

    /// Unique ID generator for tag fetch operations
    static TAG_FETCH_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

    /// Registry of in-flight tag fetch operations
    static IN_FLIGHT_TAG_REGISTRY: Lazy<Mutex<HashMap<u64, InFlightTagFetch>>> =
        Lazy::new(|| Mutex::new(HashMap::new()));

    /// Register a tag fetch operation and return its unique ID
    fn register_tag_fetch(service: &str, resource_id: &str, region: &str, account: &str) -> u64 {
        let id = TAG_FETCH_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let entry = InFlightTagFetch {
            id,
            service: service.to_string(),
            resource_id: resource_id.to_string(),
            region: region.to_string(),
            account: account.to_string(),
            start_time: Instant::now(),
        };

        if let Ok(mut registry) = IN_FLIGHT_TAG_REGISTRY.lock() {
            registry.insert(id, entry);
        }
        id
    }

    /// Unregister a tag fetch operation by ID
    fn unregister_tag_fetch(id: u64) -> Option<InFlightTagFetch> {
        if let Ok(mut registry) = IN_FLIGHT_TAG_REGISTRY.lock() {
            registry.remove(&id)
        } else {
            None
        }
    }

    /// Get all tag fetches that have been running longer than the threshold
    pub fn get_stuck_tag_fetches(threshold_secs: u64) -> Vec<InFlightTagFetch> {
        let threshold = std::time::Duration::from_secs(threshold_secs);
        if let Ok(registry) = IN_FLIGHT_TAG_REGISTRY.lock() {
            registry
                .values()
                .filter(|entry| entry.start_time.elapsed() > threshold)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Dump all stuck tag fetches to the query timing log
    pub fn dump_stuck_tag_fetches(threshold_secs: u64) {
        let stuck = get_stuck_tag_fetches(threshold_secs);
        if stuck.is_empty() {
            return;
        }

        log_query_event(&format!(
            "!!! STUCK TAG FETCHES ({} operations running > {}s):",
            stuck.len(),
            threshold_secs
        ));

        // Sort by duration (longest first)
        let mut sorted = stuck;
        sorted.sort_by(|a, b| b.start_time.elapsed().cmp(&a.start_time.elapsed()));

        for entry in sorted.iter().take(20) {
            let elapsed = entry.start_time.elapsed();
            log_query_event(&format!(
                "    [{:>6.1}s] {} {} region={} account={}",
                elapsed.as_secs_f64(),
                entry.service,
                entry.resource_id,
                entry.region,
                entry.account
            ));
        }

        if sorted.len() > 20 {
            log_query_event(&format!(
                "    ... and {} more stuck operations",
                sorted.len() - 20
            ));
        }
    }

    /// Get count of currently in-flight tag fetches
    pub fn get_in_flight_tag_count() -> usize {
        if let Ok(registry) = IN_FLIGHT_TAG_REGISTRY.lock() {
            registry.len()
        } else {
            0
        }
    }

    /// Logger for query timing events
    pub struct QueryTimingLogger {
        file: Option<File>,
        start_time: Instant,
        /// Track in-flight queries: key -> start time
        in_flight: HashMap<String, Instant>,
        /// Track expected queries for current phase
        expected_queries: HashSet<String>,
        /// Track completed queries for current phase
        completed_queries: HashSet<String>,
        /// Track failed queries
        failed_queries: HashSet<String>,
        /// Current phase name
        current_phase: Option<String>,
    }

    impl QueryTimingLogger {
        fn new() -> Self {
            let log_path = Self::log_path();
            let file = OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true) // Start fresh each session
                .open(&log_path)
                .ok();

            if file.is_some() {
                tracing::info!("Query timing log: {:?}", log_path);
            } else {
                tracing::warn!("Failed to create query timing log at {:?}", log_path);
            }

            Self {
                file,
                start_time: Instant::now(),
                in_flight: HashMap::new(),
                expected_queries: HashSet::new(),
                completed_queries: HashSet::new(),
                failed_queries: HashSet::new(),
                current_phase: None,
            }
        }

        fn log_path() -> PathBuf {
            if let Some(data_dir) = directories::ProjectDirs::from("com", "awsdash", "awsdash") {
                let log_dir = data_dir.data_local_dir().join("logs");
                std::fs::create_dir_all(&log_dir).ok();
                log_dir.join("query_timing.log")
            } else {
                PathBuf::from("query_timing.log")
            }
        }

        fn write_line(&mut self, line: &str) {
            let elapsed = self.start_time.elapsed();
            let timestamp = format!("[{:>8.3}s]", elapsed.as_secs_f64());

            if let Some(ref mut file) = self.file {
                writeln!(file, "{} {}", timestamp, line).ok();
                file.flush().ok();
            }

            // Also log to tracing for immediate visibility
            tracing::debug!(target: "query_timing", "{}", line);
        }

        fn start_phase(&mut self, phase: &str, expected: Vec<String>) {
            self.current_phase = Some(phase.to_string());
            self.expected_queries = expected.into_iter().collect();
            self.completed_queries.clear();
            self.failed_queries.clear();
            self.in_flight.clear();

            self.write_line("");
            self.write_line("============================================================");
            self.write_line(&format!(">>> {} START", phase));
            self.write_line(&format!(
                "    Expected queries: {}",
                self.expected_queries.len()
            ));

            // Sort for consistent output - collect to owned strings to avoid borrow issues
            let mut sorted: Vec<String> = self.expected_queries.iter().cloned().collect();
            sorted.sort();
            let count = sorted.len();
            let to_show: Vec<String> = sorted.into_iter().take(20).collect();
            for q in &to_show {
                self.write_line(&format!("      - {}", q));
            }
            if count > 20 {
                self.write_line(&format!("      ... and {} more", count - 20));
            }
        }

        fn end_phase(&mut self, phase: &str) {
            // Check for stuck queries
            let stuck: Vec<_> = self
                .in_flight
                .iter()
                .map(|(k, start)| (k.clone(), start.elapsed().as_secs()))
                .collect();

            // Check for missing queries (expected but never started)
            let never_started: Vec<_> = self
                .expected_queries
                .difference(&self.completed_queries)
                .filter(|q| !self.failed_queries.contains(*q))
                .filter(|q| !self.in_flight.contains_key(*q))
                .cloned()
                .collect();

            self.write_line("");
            self.write_line(&format!("<<< {} END", phase));
            self.write_line(&format!(
                "    Completed: {}/{}",
                self.completed_queries.len(),
                self.expected_queries.len()
            ));
            self.write_line(&format!("    Failed: {}", self.failed_queries.len()));
            self.write_line(&format!("    In-flight: {}", self.in_flight.len()));

            if !stuck.is_empty() {
                self.write_line("");
                self.write_line("!!! STUCK QUERIES (started but not finished):");
                for (query, secs) in &stuck {
                    self.write_line(&format!("      [STUCK {}s] {}", secs, query));
                }
            }

            if !never_started.is_empty() {
                self.write_line("");
                self.write_line("!!! MISSING QUERIES (expected but never started):");
                for query in never_started.iter().take(20) {
                    self.write_line(&format!("      [MISSING] {}", query));
                }
                if never_started.len() > 20 {
                    self.write_line(&format!(
                        "      ... and {} more missing",
                        never_started.len() - 20
                    ));
                }
            }

            if !self.failed_queries.is_empty() {
                self.write_line("");
                self.write_line("!!! FAILED QUERIES:");
                // Collect to owned strings to avoid borrow issues
                let failed: Vec<String> = self.failed_queries.iter().take(20).cloned().collect();
                for query in &failed {
                    self.write_line(&format!("      [FAILED] {}", query));
                }
            }

            self.write_line("============================================================");
            self.current_phase = None;
        }

        fn query_start(&mut self, key: &str) {
            self.in_flight.insert(key.to_string(), Instant::now());
            self.write_line(&format!("    [>] START {}", key));
        }

        fn query_done(&mut self, key: &str, result_info: &str) {
            let duration = self
                .in_flight
                .remove(key)
                .map(|start| start.elapsed().as_millis())
                .unwrap_or(0);
            self.completed_queries.insert(key.to_string());
            self.write_line(&format!(
                "    [<] DONE  {} ({}ms) {}",
                key, duration, result_info
            ));
        }

        fn query_failed(&mut self, key: &str, error: &str) {
            let duration = self
                .in_flight
                .remove(key)
                .map(|start| start.elapsed().as_millis())
                .unwrap_or(0);
            self.failed_queries.insert(key.to_string());
            self.write_line(&format!("    [X] FAIL  {} ({}ms) {}", key, duration, error));
        }

        fn check_stuck_queries(&mut self) {
            let stuck: Vec<_> = self
                .in_flight
                .iter()
                .filter(|(_, start)| start.elapsed().as_secs() > 30)
                .map(|(k, start)| (k.clone(), start.elapsed().as_secs()))
                .collect();

            for (query, secs) in stuck {
                self.write_line(&format!("!!! SLOW QUERY ({}s): {}", secs, query));
            }
        }
    }

    /// Log a query timing event (raw line)
    pub fn log_query_event(event: &str) {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.write_line(event);
        }
    }

    /// Start a phase with list of expected queries
    pub fn start_phase(phase: &str, expected_queries: Vec<String>) {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.start_phase(phase, expected_queries);
        }
    }

    /// End a phase and log summary with anomalies
    pub fn end_phase(phase: &str) {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.end_phase(phase);
        }
    }

    /// Log query start (tracks in-flight)
    pub fn query_start(key: &str) {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.query_start(key);
        }
    }

    /// Log query completion (tracks completed)
    pub fn query_done(key: &str, result_info: &str) {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.query_done(key, result_info);
        }
    }

    /// Log query failure
    pub fn query_failed(key: &str, error: &str) {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.query_failed(key, error);
        }
    }

    /// Check for slow/stuck queries (call periodically)
    pub fn check_stuck_queries() {
        if let Ok(mut logger) = QUERY_LOGGER.lock() {
            logger.check_stuck_queries();
        }
    }

    /// Log the start of a phase with context (simple version)
    pub fn log_phase_start(phase: &str, context: &str) {
        log_query_event(&format!(">>> {} START: {}", phase, context));
    }

    /// Log the end of a phase with duration (simple version)
    pub fn log_phase_end(phase: &str, duration_ms: u128, context: &str) {
        log_query_event(&format!(
            "<<< {} END: {} ({}ms)",
            phase, context, duration_ms
        ));
    }

    /// Log a query operation (simple logging)
    pub fn log_query_op(phase: &str, operation: &str, details: &str) {
        log_query_event(&format!("    [{}] {}: {}", phase, operation, details));
    }

    /// Log cache operation timing
    pub fn log_cache_op(operation: &str, key: &str, duration_ms: u128) {
        log_query_event(&format!(
            "    [CACHE] {} '{}' ({}ms)",
            operation, key, duration_ms
        ));
    }

    /// Log cache statistics summary
    pub fn log_cache_stats(
        resource_entries: u64,
        resource_size_bytes: u64,
        detailed_entries: u64,
        detailed_size_bytes: u64,
        uncompressed_bytes: u64,
    ) {
        let total_compressed = resource_size_bytes + detailed_size_bytes;
        let compression_ratio = if total_compressed > 0 {
            uncompressed_bytes as f64 / total_compressed as f64
        } else {
            1.0
        };

        log_query_event(&format!(
            "    [CACHE STATS] Resources: {} keys, {:.1}MB | Details: {} keys, {:.1}MB | Total: {:.1}MB ({:.1}x compression)",
            resource_entries,
            resource_size_bytes as f64 / 1024.0 / 1024.0,
            detailed_entries,
            detailed_size_bytes as f64 / 1024.0 / 1024.0,
            total_compressed as f64 / 1024.0 / 1024.0,
            compression_ratio
        ));
    }

    /// Log cache eviction event
    pub fn log_cache_eviction(cache_type: &str, key: &str, reason: &str) {
        log_query_event(&format!(
            "    [CACHE EVICT] {} '{}' - {}",
            cache_type, key, reason
        ));
    }

    /// Log a retry/transient error event
    pub fn log_retry_event(
        query_key: &str,
        state: &Option<crate::app::resource_explorer::retry_tracker::QueryRetryState>,
    ) {
        if let Some(state) = state {
            let error_type = state
                .last_error
                .as_ref()
                .map(|e| e.short_label())
                .unwrap_or("unknown");
            log_query_event(&format!(
                "    [RETRY] {} - {} (attempt {})",
                query_key, error_type, state.transient_errors
            ));
        }
    }

    /// Log a throttling event specifically
    pub fn log_throttled(query_key: &str, service: &str) {
        log_query_event(&format!(
            "    [THROTTLE] {} - {} rate limited",
            query_key, service
        ));
    }

    /// Log recovery from transient errors
    pub fn log_recovery(query_key: &str, error_count: u32) {
        log_query_event(&format!(
            "    [RECOVERED] {} - succeeded after {} transient error(s)",
            query_key, error_count
        ));
    }

    /// Timer helper for measuring durations
    pub struct QueryTimer {
        phase: String,
        context: String,
        start: std::time::Instant,
    }

    impl QueryTimer {
        pub fn new(phase: &str, context: &str) -> Self {
            log_phase_start(phase, context);
            Self {
                phase: phase.to_string(),
                context: context.to_string(),
                start: std::time::Instant::now(),
            }
        }

        pub fn elapsed_ms(&self) -> u128 {
            self.start.elapsed().as_millis()
        }

        pub fn log_checkpoint(&self, checkpoint: &str) {
            log_query_op(
                &self.phase,
                checkpoint,
                &format!("{}ms elapsed", self.elapsed_ms()),
            );
        }
    }

    impl Drop for QueryTimer {
        fn drop(&mut self) {
            log_phase_end(&self.phase, self.elapsed_ms(), &self.context);
        }
    }

    /// Initialize the query timing logger (call at app startup)
    pub fn init_query_timing_log() {
        // Force lazy initialization by logging the start message
        log_query_event("=== Query Timing Log Started ===");
    }

    // ========================================================================
    // Credential and Config Creation Instrumentation
    // ========================================================================

    /// Global counters for concurrent operations tracking
    static CONCURRENT_CONFIG_CREATIONS: AtomicU64 = AtomicU64::new(0);
    static CONCURRENT_CREDENTIAL_FETCHES: AtomicU64 = AtomicU64::new(0);
    static TOTAL_CONFIG_CREATIONS: AtomicU64 = AtomicU64::new(0);
    static TOTAL_CREDENTIAL_FETCHES: AtomicU64 = AtomicU64::new(0);

    /// Per-region statistics (simple counters for common regions)
    static CONFIG_US_EAST_1: AtomicU64 = AtomicU64::new(0);
    static CONFIG_US_WEST_2: AtomicU64 = AtomicU64::new(0);
    static CONFIG_US_EAST_2: AtomicU64 = AtomicU64::new(0);
    static CONFIG_OTHER_REGIONS: AtomicU64 = AtomicU64::new(0);

    /// Log start of config creation with concurrency tracking
    pub fn config_creation_start(account_id: &str, region: &str) -> u64 {
        let concurrent = CONCURRENT_CONFIG_CREATIONS.fetch_add(1, Ordering::SeqCst) + 1;
        let total = TOTAL_CONFIG_CREATIONS.fetch_add(1, Ordering::SeqCst) + 1;

        // Track per-region
        match region {
            "us-east-1" => {
                CONFIG_US_EAST_1.fetch_add(1, Ordering::SeqCst);
            }
            "us-west-2" => {
                CONFIG_US_WEST_2.fetch_add(1, Ordering::SeqCst);
            }
            "us-east-2" => {
                CONFIG_US_EAST_2.fetch_add(1, Ordering::SeqCst);
            }
            _ => {
                CONFIG_OTHER_REGIONS.fetch_add(1, Ordering::SeqCst);
            }
        }

        log_query_event(&format!(
            "    [CONFIG] START create_aws_config account={} region={} (concurrent={}, total={})",
            account_id, region, concurrent, total
        ));
        concurrent
    }

    /// Log end of config creation
    pub fn config_creation_end(
        account_id: &str,
        region: &str,
        cred_fetch_ms: u128,
        config_load_ms: u128,
        total_ms: u128,
        success: bool,
    ) {
        let concurrent = CONCURRENT_CONFIG_CREATIONS.fetch_sub(1, Ordering::SeqCst) - 1;
        let status = if success { "OK" } else { "FAILED" };

        log_query_event(&format!(
            "    [CONFIG] END   create_aws_config account={} region={} {}ms (cred={}ms, load={}ms) [{}] (concurrent={})",
            account_id, region, total_ms, cred_fetch_ms, config_load_ms, status, concurrent
        ));

        // Warn if slow
        if total_ms > 5000 {
            log_query_event(&format!(
                "!!! SLOW CONFIG CREATION: account={} region={} took {}ms (cred={}ms, load={}ms)",
                account_id, region, total_ms, cred_fetch_ms, config_load_ms
            ));
        }
    }

    /// Log start of credential fetch
    pub fn credential_fetch_start(account_id: &str, cache_hit: bool) -> u64 {
        let concurrent = CONCURRENT_CREDENTIAL_FETCHES.fetch_add(1, Ordering::SeqCst) + 1;
        let total = TOTAL_CREDENTIAL_FETCHES.fetch_add(1, Ordering::SeqCst) + 1;
        let hit_str = if cache_hit { "HIT" } else { "MISS" };

        log_query_event(&format!(
            "    [CRED] START get_credentials account={} cache={} (concurrent={}, total={})",
            account_id, hit_str, concurrent, total
        ));
        concurrent
    }

    /// Log end of credential fetch
    pub fn credential_fetch_end(
        account_id: &str,
        duration_ms: u128,
        success: bool,
        from_cache: bool,
    ) {
        let concurrent = CONCURRENT_CREDENTIAL_FETCHES.fetch_sub(1, Ordering::SeqCst) - 1;
        let status = if success { "OK" } else { "FAILED" };
        let source = if from_cache { "cache" } else { "fresh" };

        log_query_event(&format!(
            "    [CRED] END   get_credentials account={} {}ms [{}] source={} (concurrent={})",
            account_id, duration_ms, status, source, concurrent
        ));

        // Warn if slow
        if duration_ms > 3000 {
            log_query_event(&format!(
                "!!! SLOW CREDENTIAL FETCH: account={} took {}ms source={}",
                account_id, duration_ms, source
            ));
        }
    }

    /// Log Identity Center lock acquisition timing
    pub fn identity_center_lock_timing(account_id: &str, duration_ms: u128, success: bool) {
        let status = if success { "OK" } else { "FAILED" };
        log_query_event(&format!(
            "    [IC-LOCK] account={} {}ms [{}]",
            account_id, duration_ms, status
        ));

        if duration_ms > 1000 {
            log_query_event(&format!(
                "!!! SLOW IC LOCK: account={} took {}ms - possible contention",
                account_id, duration_ms
            ));
        }
    }

    /// Log Identity Center API call timing
    pub fn identity_center_api_timing(
        account_id: &str,
        role: &str,
        duration_ms: u128,
        success: bool,
    ) {
        let status = if success { "OK" } else { "FAILED" };
        log_query_event(&format!(
            "    [IC-API] get_role_credentials account={} role={} {}ms [{}]",
            account_id, role, duration_ms, status
        ));

        if duration_ms > 5000 {
            log_query_event(&format!(
                "!!! SLOW IC API: account={} role={} took {}ms",
                account_id, role, duration_ms
            ));
        }
    }

    /// Get current concurrency stats for logging
    pub fn get_concurrency_stats() -> (u64, u64, u64, u64) {
        (
            CONCURRENT_CONFIG_CREATIONS.load(Ordering::SeqCst),
            CONCURRENT_CREDENTIAL_FETCHES.load(Ordering::SeqCst),
            TOTAL_CONFIG_CREATIONS.load(Ordering::SeqCst),
            TOTAL_CREDENTIAL_FETCHES.load(Ordering::SeqCst),
        )
    }

    /// Log concurrency summary (call periodically or at end of phase)
    pub fn log_concurrency_summary() {
        let (concurrent_configs, concurrent_creds, total_configs, total_creds) =
            get_concurrency_stats();
        let us_east_1 = CONFIG_US_EAST_1.load(Ordering::SeqCst);
        let us_west_2 = CONFIG_US_WEST_2.load(Ordering::SeqCst);
        let us_east_2 = CONFIG_US_EAST_2.load(Ordering::SeqCst);
        let other = CONFIG_OTHER_REGIONS.load(Ordering::SeqCst);

        log_query_event(&format!(
            "    [CONCURRENCY] configs: current={} total={} | creds: current={} total={}",
            concurrent_configs, total_configs, concurrent_creds, total_creds
        ));
        log_query_event(&format!(
            "    [REGION STATS] us-east-1={} us-west-2={} us-east-2={} other={}",
            us_east_1, us_west_2, us_east_2, other
        ));
    }

    /// Reset concurrency counters (call at start of new query phase)
    pub fn reset_concurrency_counters() {
        CONCURRENT_CONFIG_CREATIONS.store(0, Ordering::SeqCst);
        CONCURRENT_CREDENTIAL_FETCHES.store(0, Ordering::SeqCst);
        TOTAL_CONFIG_CREATIONS.store(0, Ordering::SeqCst);
        TOTAL_CREDENTIAL_FETCHES.store(0, Ordering::SeqCst);
        CONFIG_US_EAST_1.store(0, Ordering::SeqCst);
        CONFIG_US_WEST_2.store(0, Ordering::SeqCst);
        CONFIG_US_EAST_2.store(0, Ordering::SeqCst);
        CONFIG_OTHER_REGIONS.store(0, Ordering::SeqCst);
        // Reset tag fetch counters
        CONCURRENT_TAG_FETCHES.store(0, Ordering::SeqCst);
        TOTAL_TAG_FETCHES.store(0, Ordering::SeqCst);
        TAG_FETCH_S3.store(0, Ordering::SeqCst);
        TAG_FETCH_LOGS.store(0, Ordering::SeqCst);
        TAG_FETCH_LAMBDA.store(0, Ordering::SeqCst);
        TAG_FETCH_EC2.store(0, Ordering::SeqCst);
        TAG_FETCH_IAM.store(0, Ordering::SeqCst);
        TAG_FETCH_OTHER.store(0, Ordering::SeqCst);
    }

    // ========================================================================
    // Tag Fetch Instrumentation
    // ========================================================================

    /// Global counters for tag fetch tracking
    static CONCURRENT_TAG_FETCHES: AtomicU64 = AtomicU64::new(0);
    static TOTAL_TAG_FETCHES: AtomicU64 = AtomicU64::new(0);

    /// Per-service tag fetch counters
    static TAG_FETCH_S3: AtomicU64 = AtomicU64::new(0);
    static TAG_FETCH_LOGS: AtomicU64 = AtomicU64::new(0);
    static TAG_FETCH_LAMBDA: AtomicU64 = AtomicU64::new(0);
    static TAG_FETCH_EC2: AtomicU64 = AtomicU64::new(0);
    static TAG_FETCH_IAM: AtomicU64 = AtomicU64::new(0);
    static TAG_FETCH_OTHER: AtomicU64 = AtomicU64::new(0);

    /// Log start of tag fetch with concurrency tracking and registry
    ///
    /// Returns a unique operation ID that must be passed to `tag_fetch_end`.
    /// This ID is used to track individual operations in the in-flight registry,
    /// allowing us to identify exactly which operations are stuck.
    pub fn tag_fetch_start(service: &str, resource_id: &str, region: &str, account: &str) -> u64 {
        let concurrent = CONCURRENT_TAG_FETCHES.fetch_add(1, Ordering::SeqCst) + 1;
        let total = TOTAL_TAG_FETCHES.fetch_add(1, Ordering::SeqCst) + 1;

        // Register in the in-flight registry for stuck detection
        let op_id = register_tag_fetch(service, resource_id, region, account);

        // Track per-service
        match service {
            "S3" => {
                TAG_FETCH_S3.fetch_add(1, Ordering::SeqCst);
            }
            "Logs" | "CloudWatch" => {
                TAG_FETCH_LOGS.fetch_add(1, Ordering::SeqCst);
            }
            "Lambda" => {
                TAG_FETCH_LAMBDA.fetch_add(1, Ordering::SeqCst);
            }
            "EC2" => {
                TAG_FETCH_EC2.fetch_add(1, Ordering::SeqCst);
            }
            "IAM" => {
                TAG_FETCH_IAM.fetch_add(1, Ordering::SeqCst);
            }
            _ => {
                TAG_FETCH_OTHER.fetch_add(1, Ordering::SeqCst);
            }
        }

        log_query_event(&format!(
            "    [TAG] START #{} {} {} region={} account={} (concurrent={}, total={})",
            op_id, service, resource_id, region, account, concurrent, total
        ));
        op_id
    }

    /// Log end of tag fetch and remove from in-flight registry
    ///
    /// The `op_id` parameter must be the value returned by `tag_fetch_start`.
    pub fn tag_fetch_end(
        op_id: u64,
        service: &str,
        resource_id: &str,
        region: &str,
        duration_ms: u128,
        tag_count: usize,
        success: bool,
    ) {
        // Remove from in-flight registry
        unregister_tag_fetch(op_id);

        let concurrent = CONCURRENT_TAG_FETCHES.fetch_sub(1, Ordering::SeqCst) - 1;
        let status = if success { "OK" } else { "FAILED" };

        log_query_event(&format!(
            "    [TAG] END   #{} {} {} region={} {}ms tags={} [{}] (concurrent={})",
            op_id, service, resource_id, region, duration_ms, tag_count, status, concurrent
        ));

        // Warn if slow (cross-region calls can be slow)
        if duration_ms > 10000 {
            log_query_event(&format!(
                "!!! VERY SLOW TAG FETCH: #{} {} {} region={} took {}ms",
                op_id, service, resource_id, region, duration_ms
            ));
        } else if duration_ms > 5000 {
            log_query_event(&format!(
                "!!! SLOW TAG FETCH: #{} {} {} region={} took {}ms",
                op_id, service, resource_id, region, duration_ms
            ));
        }
    }

    /// Log tag fetch summary
    pub fn log_tag_fetch_summary() {
        let concurrent = CONCURRENT_TAG_FETCHES.load(Ordering::SeqCst);
        let total = TOTAL_TAG_FETCHES.load(Ordering::SeqCst);
        let s3 = TAG_FETCH_S3.load(Ordering::SeqCst);
        let logs = TAG_FETCH_LOGS.load(Ordering::SeqCst);
        let lambda = TAG_FETCH_LAMBDA.load(Ordering::SeqCst);
        let ec2 = TAG_FETCH_EC2.load(Ordering::SeqCst);
        let iam = TAG_FETCH_IAM.load(Ordering::SeqCst);
        let other = TAG_FETCH_OTHER.load(Ordering::SeqCst);

        log_query_event(&format!(
            "    [TAG SUMMARY] current={} total={} | S3={} Logs={} Lambda={} EC2={} IAM={} Other={}",
            concurrent, total, s3, logs, lambda, ec2, iam, other
        ));

        // Warn if many concurrent fetches still in flight (potential stuck queries)
        if concurrent > 10 {
            log_query_event(&format!(
                "!!! WARNING: {} tag fetches still in flight - possible stuck queries",
                concurrent
            ));
        }
    }

    /// Get current tag fetch stats
    pub fn get_tag_fetch_stats() -> (u64, u64) {
        (
            CONCURRENT_TAG_FETCHES.load(Ordering::SeqCst),
            TOTAL_TAG_FETCHES.load(Ordering::SeqCst),
        )
    }

    // ========================================================================
    // Stuck Query Detection with Detailed Diagnostics
    // ========================================================================

    /// Check for stuck operations and log detailed diagnostics
    ///
    /// Call this periodically during long-running operations to detect and report
    /// queries that appear to be stuck.
    pub fn diagnose_stuck_operations() {
        let (concurrent_configs, concurrent_creds, total_configs, total_creds) =
            get_concurrency_stats();
        let (concurrent_tags, total_tags) = get_tag_fetch_stats();
        let in_flight_registry_count = get_in_flight_tag_count();

        // Log current state
        log_query_event("");
        log_query_event("=== STUCK OPERATION DIAGNOSTICS ===");
        log_query_event(&format!(
            "Config creations: {} in-flight / {} total",
            concurrent_configs, total_configs
        ));
        log_query_event(&format!(
            "Credential fetches: {} in-flight / {} total",
            concurrent_creds, total_creds
        ));
        log_query_event(&format!(
            "Tag fetches: {} in-flight / {} total (registry: {})",
            concurrent_tags, total_tags, in_flight_registry_count
        ));

        // Check stuck queries from the main logger
        check_stuck_queries();

        // Dump individual stuck tag fetches (> 10 seconds)
        dump_stuck_tag_fetches(10);

        // Diagnose potential issues based on concurrency
        if concurrent_configs > 50 {
            log_query_event("!!! DIAGNOSIS: Very high concurrent config creations may indicate DNS resolution bottleneck or connection pool exhaustion");
        }
        if concurrent_creds > 20 {
            log_query_event("!!! DIAGNOSIS: High concurrent credential fetches may indicate Identity Center API rate limiting");
        }
        if concurrent_tags > 100 {
            log_query_event("!!! DIAGNOSIS: Very high concurrent tag fetches may overwhelm async runtime or socket pool");
        }

        // Check for imbalance (many started but few completing)
        if total_configs > 0 && concurrent_configs as f64 / total_configs as f64 > 0.5 {
            log_query_event("!!! DIAGNOSIS: More than 50% of config creations still in flight - possible contention");
        }
        if total_tags > 0 && concurrent_tags as f64 / total_tags as f64 > 0.3 {
            log_query_event("!!! DIAGNOSIS: More than 30% of tag fetches still in flight - possible stuck operations");
        }

        // Region stats for cross-region issues
        let us_east_1 = CONFIG_US_EAST_1.load(Ordering::SeqCst);
        let us_west_2 = CONFIG_US_WEST_2.load(Ordering::SeqCst);
        let us_east_2 = CONFIG_US_EAST_2.load(Ordering::SeqCst);
        let other = CONFIG_OTHER_REGIONS.load(Ordering::SeqCst);

        log_query_event(&format!(
            "Region distribution: us-east-1={} us-west-2={} us-east-2={} other={}",
            us_east_1, us_west_2, us_east_2, other
        ));

        if us_west_2 > 0 || us_east_2 > 0 || other > 0 {
            log_query_event(
                "NOTE: Cross-region requests detected - these can be slower due to network latency",
            );
        }

        log_query_event("=== END DIAGNOSTICS ===");
        log_query_event("");
    }

    /// Log a watchdog message showing current state (call from a timer/watchdog thread)
    pub fn watchdog_pulse() {
        let (concurrent_configs, concurrent_creds, total_configs, total_creds) =
            get_concurrency_stats();
        let (concurrent_tags, total_tags) = get_tag_fetch_stats();
        let in_flight_count = get_in_flight_tag_count();

        if concurrent_configs > 0 || concurrent_creds > 0 || concurrent_tags > 0 {
            log_query_event(&format!(
                "    [WATCHDOG] configs={}/{} creds={}/{} tags={}/{} registry={}",
                concurrent_configs,
                total_configs,
                concurrent_creds,
                total_creds,
                concurrent_tags,
                total_tags,
                in_flight_count
            ));

            // If there are in-flight operations, check for stuck ones (> 15 seconds)
            if in_flight_count > 0 {
                let stuck = get_stuck_tag_fetches(15);
                if !stuck.is_empty() {
                    log_query_event(&format!(
                        "    [WATCHDOG] {} tag fetches stuck > 15s:",
                        stuck.len()
                    ));
                    for entry in stuck.iter().take(5) {
                        log_query_event(&format!(
                            "        [{:.1}s] {} {} region={}",
                            entry.start_time.elapsed().as_secs_f64(),
                            entry.service,
                            entry.resource_id,
                            entry.region
                        ));
                    }
                    if stuck.len() > 5 {
                        log_query_event(&format!("        ... and {} more", stuck.len() - 5));
                    }
                }
            }
        }
    }
}

// ============================================================================
// Release build no-op implementation (zero overhead)
// ============================================================================

#[cfg(not(debug_assertions))]
mod inner {
    /// No-op timer for release builds
    pub struct QueryTimer;

    impl QueryTimer {
        #[inline(always)]
        pub fn new(_phase: &str, _context: &str) -> Self {
            Self
        }

        #[inline(always)]
        pub fn elapsed_ms(&self) -> u128 {
            0
        }

        #[inline(always)]
        pub fn log_checkpoint(&self, _checkpoint: &str) {}
    }

    #[inline(always)]
    pub fn log_query_event(_event: &str) {}

    #[inline(always)]
    pub fn start_phase(_phase: &str, _expected_queries: Vec<String>) {}

    #[inline(always)]
    pub fn end_phase(_phase: &str) {}

    #[inline(always)]
    pub fn query_start(_key: &str) {}

    #[inline(always)]
    pub fn query_done(_key: &str, _result_info: &str) {}

    #[inline(always)]
    pub fn query_failed(_key: &str, _error: &str) {}

    #[inline(always)]
    pub fn check_stuck_queries() {}

    #[inline(always)]
    pub fn log_phase_start(_phase: &str, _context: &str) {}

    #[inline(always)]
    pub fn log_phase_end(_phase: &str, _duration_ms: u128, _context: &str) {}

    #[inline(always)]
    pub fn log_query_op(_phase: &str, _operation: &str, _details: &str) {}

    #[inline(always)]
    pub fn log_cache_op(_operation: &str, _key: &str, _duration_ms: u128) {}

    #[inline(always)]
    pub fn log_cache_stats(
        _resource_entries: u64,
        _resource_size_bytes: u64,
        _detailed_entries: u64,
        _detailed_size_bytes: u64,
        _uncompressed_bytes: u64,
    ) {
    }

    #[inline(always)]
    pub fn log_cache_eviction(_cache_type: &str, _key: &str, _reason: &str) {}

    #[inline(always)]
    pub fn log_retry_event(
        _query_key: &str,
        _state: &Option<crate::app::resource_explorer::retry_tracker::QueryRetryState>,
    ) {
    }

    #[inline(always)]
    pub fn log_throttled(_query_key: &str, _service: &str) {}

    #[inline(always)]
    pub fn log_recovery(_query_key: &str, _error_count: u32) {}

    #[inline(always)]
    pub fn init_query_timing_log() {}

    // Credential instrumentation no-ops
    #[inline(always)]
    pub fn config_creation_start(_account_id: &str, _region: &str) -> u64 {
        0
    }

    #[inline(always)]
    pub fn config_creation_end(
        _account_id: &str,
        _region: &str,
        _cred_fetch_ms: u128,
        _config_load_ms: u128,
        _total_ms: u128,
        _success: bool,
    ) {
    }

    #[inline(always)]
    pub fn credential_fetch_start(_account_id: &str, _cache_hit: bool) -> u64 {
        0
    }

    #[inline(always)]
    pub fn credential_fetch_end(
        _account_id: &str,
        _duration_ms: u128,
        _success: bool,
        _from_cache: bool,
    ) {
    }

    #[inline(always)]
    pub fn identity_center_lock_timing(_account_id: &str, _duration_ms: u128, _success: bool) {}

    #[inline(always)]
    pub fn identity_center_api_timing(
        _account_id: &str,
        _role: &str,
        _duration_ms: u128,
        _success: bool,
    ) {
    }

    #[inline(always)]
    pub fn get_concurrency_stats() -> (u64, u64, u64, u64) {
        (0, 0, 0, 0)
    }

    #[inline(always)]
    pub fn log_concurrency_summary() {}

    #[inline(always)]
    pub fn reset_concurrency_counters() {}

    // Tag fetch no-ops
    #[inline(always)]
    pub fn tag_fetch_start(
        _service: &str,
        _resource_id: &str,
        _region: &str,
        _account: &str,
    ) -> u64 {
        0
    }

    #[inline(always)]
    pub fn tag_fetch_end(
        _op_id: u64,
        _service: &str,
        _resource_id: &str,
        _region: &str,
        _duration_ms: u128,
        _tag_count: usize,
        _success: bool,
    ) {
    }

    #[inline(always)]
    pub fn log_tag_fetch_summary() {}

    #[inline(always)]
    pub fn get_tag_fetch_stats() -> (u64, u64) {
        (0, 0)
    }

    // In-flight registry no-ops
    #[inline(always)]
    pub fn dump_stuck_tag_fetches(_threshold_secs: u64) {}

    #[inline(always)]
    pub fn get_in_flight_tag_count() -> usize {
        0
    }

    // Stuck query detection no-ops
    #[inline(always)]
    pub fn diagnose_stuck_operations() {}

    #[inline(always)]
    pub fn watchdog_pulse() {}
}

// Re-export from the appropriate module
pub use inner::*;
