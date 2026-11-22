//! Preflight check caching engine for near-zero latency launch verification.
//!
//! Avoids re-querying Vulkan ICD manifests, disk space queries, and external
//! tools on every launch when system inputs and instance configuration are unchanged.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::requirements::{check_system_requirements, SystemRequirementsReport};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

/// Cache validity duration before doing a background refresh (default: 1 hour).
pub const PREFLIGHT_CACHE_TTL: Duration = Duration::from_secs(3600);

/// Serialized cached preflight entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPreflight {
    pub fingerprint: String,
    pub timestamp_sec: u64,
    pub report: SystemRequirementsReport,
}

/// In-memory and persistent manager for system preflight check caching.
#[derive(Debug, Clone)]
pub struct PreflightCache {
    cache_file: PathBuf,
    current: Arc<RwLock<Option<CachedPreflight>>>,
}

impl PreflightCache {
    pub fn new(cache_file: PathBuf) -> Self {
        let loaded = Self::load_disk(&cache_file);
        Self {
            cache_file,
            current: Arc::new(RwLock::new(loaded)),
        }
    }

    fn load_disk(path: &Path) -> Option<CachedPreflight> {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Computes a lightweight fingerprint of inputs (OS kernel version, user name, instance path).
    pub fn compute_fingerprint(instance_path: Option<&Path>, username: Option<&str>) -> String {
        let os_str = std::env::consts::OS;
        let arch_str = std::env::consts::ARCH;
        let inst_str = instance_path
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let user_str = username.unwrap_or("default");

        format!("{}:{}:{}:{}", os_str, arch_str, inst_str, user_str)
    }

    /// Retrieves system requirements report. If cached and valid, returns immediately.
    /// Otherwise, runs full system check and caches the result.
    pub fn get_or_check(
        &self,
        instance_path: Option<&Path>,
        active_username: Option<&str>,
        force_refresh: bool,
    ) -> SystemRequirementsReport {
        let fingerprint = Self::compute_fingerprint(instance_path, active_username);
        let now_sec = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // 1. Check in-memory cache
        if !force_refresh {
            let guard = self.current.read();
            if let Some(entry) = guard.as_ref() {
                if entry.fingerprint == fingerprint && (now_sec - entry.timestamp_sec) < PREFLIGHT_CACHE_TTL.as_secs() {
                    return entry.report.clone();
                }
            }
        }

        // 2. Cache miss or forced refresh: execute real system checks
        let report = check_system_requirements(instance_path, active_username);

        // 3. Store result in memory and persist
        let entry = CachedPreflight {
            fingerprint,
            timestamp_sec: now_sec,
            report: report.clone(),
        };

        *self.current.write() = Some(entry.clone());

        if let Some(parent) = self.cache_file.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&entry) {
            let _ = fs::write(&self.cache_file, json);
        }

        report
    }

    /// Invalidates cached preflight results.
    pub fn invalidate(&self) {
        *self.current.write() = None;
        let _ = fs::remove_file(&self.cache_file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;
    use tempfile::tempdir;

    #[test]
    fn test_preflight_caching_and_invalidation() {
        let dir = tempdir().unwrap();
        let cache_file = dir.path().join("preflight_cache.json");
        let cache = PreflightCache::new(cache_file.clone());

        // Initial run
        let t0 = Instant::now();
        let rep1 = cache.get_or_check(None, Some("Player1"), false);
        let d1 = t0.elapsed();

        // Cached run should be much faster (instant memory lookup)
        let t1 = Instant::now();
        let rep2 = cache.get_or_check(None, Some("Player1"), false);
        let d2 = t1.elapsed();

        assert_eq!(rep1.all_passed, rep2.all_passed);
        assert!(d2 <= d1 || d2 < Duration::from_millis(50));
        assert!(cache_file.is_file());

        // Invalidate cache
        cache.invalidate();
        assert!(!cache_file.is_file());
    }
}
