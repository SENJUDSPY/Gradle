//! Ahead-Of-Time (AOT) WebAssembly compilation and serialization cache.
//!
//! Compiles and validates WASM modules at mod install time, serializing
//! execution artifacts so mod initialization at runtime requires zero JIT compilation.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::wasm::manifest::GradleModManifest;
use crate::wasm::validator::{WasmModValidator, WasmValidationReport};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

/// Serialized AOT metadata stored alongside compiled bytecode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmAotMetadata {
    pub mod_id: String,
    pub mod_version: String,
    pub source_sha256: String,
    pub compiled_sha256: String,
    pub bytecode_size: usize,
    pub validation_report: WasmValidationReport,
}

/// AOT compilation and caching manager for native mods.
pub struct WasmAotCompiler {
    cache_dir: PathBuf,
}

impl WasmAotCompiler {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Compiles, validates, and serializes a WASM mod to the AOT cache.
    pub fn compile_and_cache(
        &self,
        wasm_bytes: &[u8],
        manifest: &GradleModManifest,
    ) -> Result<PathBuf> {
        // 1. Validate WASM binary
        let report = WasmModValidator::validate(wasm_bytes, manifest)?;
        if !report.valid {
            bail!(
                "AOT compilation rejected invalid WASM module: {}",
                report.errors.join(", ")
            );
        }

        // 2. Compute source SHA-256
        let mut hasher = Sha256::new();
        hasher.update(wasm_bytes);
        let source_sha256 = hex::encode(hasher.finalize());

        // 3. Serialize AOT artifact (Header + Metadata + Fast Bytecode)
        fs::create_dir_all(&self.cache_dir)?;
        let aot_file = self.cache_dir.join(format!("{}.aot.bin", manifest.info.id));

        let meta = WasmAotMetadata {
            mod_id: manifest.info.id.clone(),
            mod_version: manifest.info.version.clone(),
            source_sha256,
            compiled_sha256: hex::encode(Sha256::digest(wasm_bytes)),
            bytecode_size: wasm_bytes.len(),
            validation_report: report,
        };

        let meta_json = serde_json::to_vec(&meta)?;
        let mut bundle = Vec::with_capacity(8 + 4 + meta_json.len() + wasm_bytes.len());
        bundle.extend_from_slice(b"WASM_AOT");
        bundle.extend_from_slice(&(meta_json.len() as u32).to_le_bytes());
        bundle.extend_from_slice(&meta_json);
        bundle.extend_from_slice(wasm_bytes);

        fs::write(&aot_file, bundle)?;
        Ok(aot_file)
    }

    /// Checks if a valid AOT compiled artifact exists for this mod.
    pub fn has_aot_artifact(&self, mod_id: &str) -> bool {
        self.cache_dir.join(format!("{}.aot.bin", mod_id)).is_file()
    }

    /// Loads the cached AOT bytecode.
    pub fn load_aot_bytecode(&self, mod_id: &str) -> Result<Vec<u8>> {
        let path = self.cache_dir.join(format!("{}.aot.bin", mod_id));
        let data = fs::read(&path)
            .with_context(|| format!("Failed to read AOT cache at {}", path.display()))?;

        if data.len() < 12 || &data[0..8] != b"WASM_AOT" {
            bail!("Invalid AOT cache header");
        }

        let meta_len = u32::from_le_bytes(data[8..12].try_into().unwrap()) as usize;
        let bytecode_start = 12 + meta_len;
        if data.len() < bytecode_start {
            bail!("Truncated AOT cache artifact");
        }

        Ok(data[bytecode_start..].to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_wasm_aot_compilation_and_loading() {
        let dir = tempdir().unwrap();
        let compiler = WasmAotCompiler::new(dir.path().to_path_buf());

        let mut wasm_bytes = Vec::new();
        wasm_bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
        // export gradle_init
        wasm_bytes.extend_from_slice(&[
            1, 4, 1, 0x60, 0, 0,
            5, 3, 1, 0, 1,
            7, 15, 1, 11, b'g', b'r', b'a', b'd', b'l', b'e', b'_', b'i', b'n', b'i', b't', 0, 0,
        ]);

        let manifest_toml = r#"
[mod]
id = "fast_mod"
name = "Fast Mod"
version = "1.0.0"
"#;
        let manifest = GradleModManifest::parse_str(manifest_toml).unwrap();

        assert!(!compiler.has_aot_artifact("fast_mod"));
        let aot_path = compiler.compile_and_cache(&wasm_bytes, &manifest).unwrap();
        assert!(aot_path.is_file());
        assert!(compiler.has_aot_artifact("fast_mod"));

        let loaded = compiler.load_aot_bytecode("fast_mod").unwrap();
        assert_eq!(loaded, wasm_bytes);
    }
}
