//! Persistent wgpu and Vulkan pipeline cache manager.
//!
//! Stores pre-compiled pipeline states and SPIR-V/WGSL shader binaries per
//! GPU device ID and driver version. On warm start, restores pipelines
//! with zero shader compile latency.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

/// GPU and Driver hardware identification key.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct GpuDriverIdentity {
    pub vendor_id: u32,
    pub device_id: u32,
    pub driver_version: u32,
    pub device_name: String,
    pub engine_version: String,
}

impl GpuDriverIdentity {
    pub fn compute_cache_key(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.vendor_id.to_le_bytes());
        hasher.update(self.device_id.to_le_bytes());
        hasher.update(self.driver_version.to_le_bytes());
        hasher.update(self.device_name.as_bytes());
        hasher.update(self.engine_version.as_bytes());
        hex::encode(hasher.finalize())
    }
}

/// Pipeline cache container for persisting binary shader states to disk.
pub struct WgpuPipelineCache {
    base_cache_dir: PathBuf,
}

impl WgpuPipelineCache {
    pub fn new(base_cache_dir: PathBuf) -> Self {
        Self { base_cache_dir }
    }

    /// Resolves the specific cache directory for a GPU and driver identity.
    pub fn pipeline_dir_for(&self, identity: &GpuDriverIdentity) -> PathBuf {
        let key = identity.compute_cache_key();
        self.base_cache_dir.join(&key[0..16])
    }

    /// Checks if a valid pre-compiled pipeline cache exists for this GPU.
    pub fn has_cached_pipelines(&self, identity: &GpuDriverIdentity) -> bool {
        let path = self.pipeline_dir_for(identity).join("pipeline_cache.bin");
        path.is_file()
    }

    /// Saves binary pipeline cache blob.
    pub fn save_pipeline_blob(&self, identity: &GpuDriverIdentity, blob: &[u8]) -> Result<PathBuf> {
        let dir = self.pipeline_dir_for(identity);
        fs::create_dir_all(&dir)?;

        let dest = dir.join("pipeline_cache.bin");
        fs::write(&dest, blob)?;

        // Also save identity metadata
        let meta_dest = dir.join("identity.json");
        let json = serde_json::to_string_pretty(identity)?;
        fs::write(meta_dest, json)?;

        Ok(dest)
    }

    /// Loads binary pipeline cache blob. Returns None if cache does not exist.
    pub fn load_pipeline_blob(&self, identity: &GpuDriverIdentity) -> Option<Vec<u8>> {
        let dest = self.pipeline_dir_for(identity).join("pipeline_cache.bin");
        fs::read(dest).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wgpu_pipeline_cache_persistence() {
        let dir = tempdir().unwrap();
        let cache = WgpuPipelineCache::new(dir.path().to_path_buf());

        let id = GpuDriverIdentity {
            vendor_id: 0x10DE, // NVIDIA
            device_id: 0x2484,
            driver_version: 550,
            device_name: "NVIDIA GeForce RTX 4070".to_string(),
            engine_version: "26.3.0".to_string(),
        };

        assert!(!cache.has_cached_pipelines(&id));
        assert!(cache.load_pipeline_blob(&id).is_none());

        // Save simulated binary pipeline blob
        let sample_blob = vec![0xCA, 0xFE, 0xBA, 0xBE, 1, 2, 3, 4];
        let saved_path = cache.save_pipeline_blob(&id, &sample_blob).unwrap();
        assert!(saved_path.is_file());

        assert!(cache.has_cached_pipelines(&id));
        let loaded = cache.load_pipeline_blob(&id).unwrap();
        assert_eq!(loaded, sample_blob);
    }
}
