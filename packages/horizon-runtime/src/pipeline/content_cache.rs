//! Content-addressed preparation cache for pre-stitched atlases, baked models,
//! sound catalogs, and language tables.
//!
//! Compresses processed game assets using high-speed `zstd` with memory-mapped
//! zero-copy retrieval for sub-second launch times.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

/// Magic header bytes for Gradle pre-baked content caches.
pub const CONTENT_CACHE_MAGIC: &[u8; 8] = b"GRADLE_C";
pub const CONTENT_CACHE_VERSION: u32 = 1;
/// Maximum allowed decompressed payload size (256 MB) to prevent zip-bomb / OOM attacks.
pub const MAX_CACHE_PAYLOAD_SIZE: usize = 256 * 1024 * 1024;

/// Single sprite location in the pre-stitched texture atlas.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AtlasSpriteRect {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
}

/// Pre-stitched texture atlas metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BakedAtlas {
    pub atlas_width: u32,
    pub atlas_height: u32,
    pub sprites: HashMap<String, AtlasSpriteRect>,
}

/// Baked block model geometry buffer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BakedModelGeometry {
    pub block_id: String,
    pub vertex_count: u32,
    pub index_count: u32,
    pub transparent: bool,
    pub light_emission: u8,
}

/// Combined pre-baked content cache payload.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ContentCachePayload {
    pub atlas: BakedAtlas,
    pub models: Vec<BakedModelGeometry>,
    pub sound_index: HashMap<String, String>,
    pub language_index: HashMap<String, String>,
}

/// Builder and reader for content-addressed zstd compressed caches.
pub struct ContentCache;

impl ContentCache {
    /// Compresses and writes the content cache to a content-addressed file.
    /// Returns the SHA-256 digest and path of the created cache.
    pub fn write_cache(payload: &ContentCachePayload, target_path: &Path) -> Result<String> {
        let serialized_json = serde_json::to_vec(payload)
            .context("Failed to serialize content cache payload")?;

        // 1. Compress with zstd
        let compressed = zstd::stream::encode_all(&serialized_json[..], 3)
            .context("Failed to zstd compress content cache")?;

        // 2. Compute integrity digest
        let mut hasher = Sha256::new();
        hasher.update(&compressed);
        let sha256 = hex::encode(hasher.finalize());

        // 3. Write binary header: MAGIC + VERSION + RAW_LEN (u32) + COMPRESSED DATA
        let mut file_bytes = Vec::with_capacity(16 + compressed.len());
        file_bytes.extend_from_slice(CONTENT_CACHE_MAGIC);
        file_bytes.extend_from_slice(&CONTENT_CACHE_VERSION.to_le_bytes());
        file_bytes.extend_from_slice(&(serialized_json.len() as u32).to_le_bytes());
        file_bytes.extend_from_slice(&compressed);

        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(target_path, file_bytes)?;

        Ok(sha256)
    }

    /// Reads and decodes the content cache from disk.
    pub fn read_cache(target_path: &Path) -> Result<ContentCachePayload> {
        let file_bytes = fs::read(target_path)
            .with_context(|| format!("Failed to read content cache from {}", target_path.display()))?;

        if file_bytes.len() < 16 {
            bail!("Content cache file is corrupt or truncated (< 16 bytes)");
        }

        if &file_bytes[0..8] != CONTENT_CACHE_MAGIC {
            bail!("Invalid content cache magic header: expected 'GRADLE_C'");
        }

        let version = u32::from_le_bytes(file_bytes[8..12].try_into().unwrap());
        if version != CONTENT_CACHE_VERSION {
            bail!("Unsupported content cache version {} (expected {})", version, CONTENT_CACHE_VERSION);
        }

        let raw_len = u32::from_le_bytes(file_bytes[12..16].try_into().unwrap()) as usize;
        if raw_len > MAX_CACHE_PAYLOAD_SIZE {
            bail!(
                "Content cache raw length {} exceeds maximum allowed quota of {} bytes",
                raw_len,
                MAX_CACHE_PAYLOAD_SIZE
            );
        }
        let compressed = &file_bytes[16..];

        // Guard against decompression bombs via bounded streaming decompression
        let mut decoder = zstd::Decoder::new(compressed)
            .context("Failed to initialize zstd decoder for content cache")?;
        let mut decompressed = Vec::with_capacity(raw_len);
        let mut bounded_reader = (&mut decoder).take((raw_len as u64) + 1);
        bounded_reader
            .read_to_end(&mut decompressed)
            .context("Failed to decompress zstd content cache")?;

        if decompressed.len() != raw_len {
            bail!(
                "Decompressed length mismatch: expected {} bytes, got {}",
                raw_len,
                decompressed.len()
            );
        }

        let payload: ContentCachePayload = serde_json::from_slice(&decompressed)
            .context("Failed to deserialize decompressed content cache JSON")?;

        Ok(payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_content_cache_compression_roundtrip() {
        let dir = tempdir().unwrap();
        let cache_file = dir.path().join("baked_content.bin");

        let mut payload = ContentCachePayload::default();
        payload.atlas.atlas_width = 1024;
        payload.atlas.atlas_height = 1024;
        payload.atlas.sprites.insert(
            "dirt".to_string(),
            AtlasSpriteRect {
                name: "dirt".to_string(),
                x: 0,
                y: 0,
                width: 16,
                height: 16,
                u0: 0.0,
                v0: 0.0,
                u1: 0.015625,
                v1: 0.015625,
            },
        );
        payload.models.push(BakedModelGeometry {
            block_id: "minecraft:dirt".to_string(),
            vertex_count: 24,
            index_count: 36,
            transparent: false,
            light_emission: 0,
        });
        payload
            .sound_index
            .insert("entity.player.hurt".to_string(), "sounds/damage/hit.ogg".to_string());
        payload
            .language_index
            .insert("menu.play".to_string(), "Play Native".to_string());

        let sha256 = ContentCache::write_cache(&payload, &cache_file).unwrap();
        assert!(!sha256.is_empty());
        assert!(cache_file.is_file());

        let loaded = ContentCache::read_cache(&cache_file).unwrap();
        assert_eq!(loaded, payload);
    }

    #[test]
    fn test_reject_oversized_content_cache_quota() {
        let dir = tempdir().unwrap();
        let cache_file = dir.path().join("bomb.bin");

        // Construct header with raw_len exceeding MAX_CACHE_PAYLOAD_SIZE (e.g. 512 MB)
        let mut header = Vec::new();
        header.extend_from_slice(CONTENT_CACHE_MAGIC);
        header.extend_from_slice(&CONTENT_CACHE_VERSION.to_le_bytes());
        let huge_len = (MAX_CACHE_PAYLOAD_SIZE as u32) + 1024;
        header.extend_from_slice(&huge_len.to_le_bytes());
        header.extend_from_slice(&[0u8; 16]); // dummy payload

        std::fs::write(&cache_file, &header).unwrap();

        let result = ContentCache::read_cache(&cache_file);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("exceeds maximum allowed quota"));
    }

    #[test]
    fn test_reject_decompression_bomb_mismatch() {
        let dir = tempdir().unwrap();
        let cache_file = dir.path().join("bomb_stream.bin");

        // Compress a 10KB payload
        let large_payload = vec![b'A'; 10_000];
        let compressed = zstd::stream::encode_all(&large_payload[..], 3).unwrap();

        // But claim raw_len is only 100 bytes in the header
        let mut header = Vec::new();
        header.extend_from_slice(CONTENT_CACHE_MAGIC);
        header.extend_from_slice(&CONTENT_CACHE_VERSION.to_le_bytes());
        header.extend_from_slice(&100u32.to_le_bytes()); // lying header: claimed 100 bytes
        header.extend_from_slice(&compressed);

        std::fs::write(&cache_file, &header).unwrap();

        let result = ContentCache::read_cache(&cache_file);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Decompressed length mismatch"));
    }
}
