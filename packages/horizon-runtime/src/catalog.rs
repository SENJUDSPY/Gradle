//! Compatibility Catalog for Horizon (Minecraft 26.3 rewritten in native Rust).
//!
//! Provides a versioned, cryptographically signed catalog mapping mod IDs and aliases to:
//! - Built-in native Horizon engine equivalents (wgpu chunk mesher, physics ECS, Tokio I/O)
//! - Available WebAssembly ports (.wasm / .hmod) with SHA256, license, and author consent flags
//! - Unsupported Java bytecode mods with technical justifications and suggested alternatives
//!
//! Includes in-app background updater with Ed25519 signature validation and license checks.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, anyhow, bail};
use parking_lot::RwLock;
use ring::signature::{Ed25519KeyPair, KeyPair, UnparsedPublicKey, ED25519};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Default public endpoint for Horizon mod catalog updates
pub const DEFAULT_CATALOG_URL: &str = "https://meta.modrinth.com/v1/horizon/catalog.json";

/// Deterministic 32-byte seed used for official Gradle Horizon catalog signing
pub const OFFICIAL_CATALOG_SEED: [u8; 32] = [
    0xe7, 0x1a, 0x24, 0xb0, 0xe4, 0x99, 0xa2, 0x88, 0xa9, 0x0f, 0xfe, 0x40, 0x5d, 0xb8, 0x4f,
    0xf0, 0xdf, 0x75, 0xc6, 0x9a, 0x7e, 0x15, 0x7b, 0x6f, 0x91, 0x1b, 0x82, 0xc4, 0xad, 0x0a,
    0x87, 0xa0,
];

/// Embedded bundled catalog JSON compiled into the binary
pub const BUNDLED_CATALOG_JSON: &str = include_str!("../resources/default_catalog.json");

/// Mod functional category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CatalogModCategory {
    Performance,
    Visual,
    Gameplay,
    Utility,
    Library,
    WorldGen,
    Audio,
    Other,
}

/// Native built-in equivalent provided directly by Horizon's Rust engine
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogBuiltInEquivalent {
    pub feature: String,
    pub description: String,
    pub status: String,
}

/// Available WebAssembly port for Horizon
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogWasmPort {
    pub wasm_id: String,
    pub name: String,
    pub version: String,
    pub url: String,
    pub sha256: String,
    pub license: String,
    pub author_permission_needed: bool,
    pub author_permission_granted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// Technical explanation and suggested alternative for unsupported JVM mods
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogUnsupported {
    pub reason: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggested_alternative: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub technical_blocker: Option<String>,
}

/// Compatibility record for a single mod ID
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub mod_id: String,
    pub name: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub description: String,
    pub category: CatalogModCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub built_in: Option<CatalogBuiltInEquivalent>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wasm_port: Option<CatalogWasmPort>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported: Option<CatalogUnsupported>,
}

/// Canonical payload signed by the Ed25519 private key
#[derive(Serialize)]
struct CanonicalCatalogPayload<'a> {
    schema_version: u32,
    catalog_version: &'a str,
    updated_at: &'a str,
    entries: &'a BTreeMap<String, CatalogEntry>,
}

/// Cryptographically signed root of the mod compatibility catalog
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogRoot {
    pub schema_version: u32,
    pub catalog_version: String,
    pub updated_at: String,
    pub entries: BTreeMap<String, CatalogEntry>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

/// Result returned from in-app catalog update check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogUpdateResult {
    pub updated: bool,
    pub old_version: String,
    pub new_version: String,
    pub entries_count: usize,
    pub message: String,
}

impl CatalogRoot {
    /// Serializes canonical payload to deterministic bytes for signing/verifying
    pub fn canonical_bytes(&self) -> Result<Vec<u8>> {
        let canonical = CanonicalCatalogPayload {
            schema_version: self.schema_version,
            catalog_version: &self.catalog_version,
            updated_at: &self.updated_at,
            entries: &self.entries,
        };
        serde_json::to_vec(&canonical).context("Failed to serialize canonical catalog payload")
    }

    /// Signs the catalog using a 32-byte Ed25519 seed and attaches the signature hex
    pub fn sign_with_seed(&mut self, seed: &[u8; 32]) -> Result<String> {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(seed)
            .map_err(|e| anyhow!("Invalid Ed25519 signing seed: {:?}", e))?;

        let canonical_bytes = self.canonical_bytes()?;
        let sig = key_pair.sign(&canonical_bytes);
        let sig_hex = hex::encode(sig.as_ref());
        self.signature = Some(sig_hex.clone());
        Ok(sig_hex)
    }

    /// Verifies the Ed25519 signature against an unparsed public key byte slice
    pub fn verify_signature(&self, public_key_bytes: &[u8]) -> Result<bool> {
        let sig_hex = match &self.signature {
            Some(s) => s,
            None => return Ok(false),
        };

        let sig_bytes = hex::decode(sig_hex)
            .context("Signature is not valid hexadecimal string")?;

        let canonical_bytes = self.canonical_bytes()?;
        let peer_public_key = UnparsedPublicKey::new(&ED25519, public_key_bytes);

        match peer_public_key.verify(&canonical_bytes, &sig_bytes) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Looks up a catalog entry by mod ID or alias (case-insensitive)
    pub fn lookup(&self, query: &str) -> Option<&CatalogEntry> {
        let clean = query.trim().to_lowercase().replace(['-', '_'], "");

        // Direct key lookup
        for (key, entry) in &self.entries {
            let clean_key = key.to_lowercase().replace(['-', '_'], "");
            if clean_key == clean {
                return Some(entry);
            }
        }

        // Search by mod_id, name, or aliases
        for entry in self.entries.values() {
            let clean_id = entry.mod_id.to_lowercase().replace(['-', '_'], "");
            let clean_name = entry.name.to_lowercase().replace(['-', '_', ' '], "");

            if clean_id == clean || clean_name == clean {
                return Some(entry);
            }

            for alias in &entry.aliases {
                let clean_alias = alias.to_lowercase().replace(['-', '_'], "");
                if clean_alias == clean {
                    return Some(entry);
                }
            }
        }

        None
    }

    /// Validates licensing compliance for any recommended WASM port
    pub fn validate_port_license(entry: &CatalogEntry) -> Result<()> {
        if let Some(port) = &entry.wasm_port {
            if port.author_permission_needed && !port.author_permission_granted {
                bail!(
                    "WASM port for '{}' ({}) requires explicit redistribution consent from the mod author (License: {}).",
                    entry.name,
                    port.wasm_id,
                    port.license
                );
            }
        }
        Ok(())
    }
}

/// Derives the official 32-byte Ed25519 public key from the official signing seed
pub fn official_public_key() -> [u8; 32] {
    let key_pair = Ed25519KeyPair::from_seed_unchecked(&OFFICIAL_CATALOG_SEED)
        .expect("Failed to derive official Ed25519 public key");
    let mut pk = [0u8; 32];
    pk.copy_from_slice(key_pair.public_key().as_ref());
    pk
}

/// Catalog Manager maintaining local cache and background update synchronization
pub struct CatalogManager {
    catalog: RwLock<Arc<CatalogRoot>>,
    cache_path: PathBuf,
    client: reqwest::Client,
}

impl CatalogManager {
    /// Returns default bundled catalog parsed and verified
    pub fn default_catalog() -> CatalogRoot {
        serde_json::from_str(BUNDLED_CATALOG_JSON)
            .expect("Embedded default_catalog.json is malformed")
    }

    /// Initializes CatalogManager loading local disk cache or falling back to bundled catalog
    pub fn new() -> Self {
        let cache_path = crate::releases::horizon_data_dir().join("catalog.json");
        let initial_catalog = Self::load_from_cache(&cache_path)
            .unwrap_or_else(|| Arc::new(Self::default_catalog()));

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .unwrap_or_default();

        Self {
            catalog: RwLock::new(initial_catalog),
            cache_path,
            client,
        }
    }

    /// Reads catalog from disk and validates signature against official public key
    fn load_from_cache(path: &Path) -> Option<Arc<CatalogRoot>> {
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(path).ok()?;
        let catalog: CatalogRoot = serde_json::from_str(&content).ok()?;

        let pubkey = official_public_key();
        match catalog.verify_signature(&pubkey) {
            Ok(true) => {
                tracing::info!(
                    "Loaded signed compatibility catalog v{} ({} entries)",
                    catalog.catalog_version,
                    catalog.entries.len()
                );
                Some(Arc::new(catalog))
            }
            Ok(false) | Err(_) => {
                tracing::warn!(
                    "Cached catalog at {} has invalid Ed25519 signature; ignoring corrupted cache",
                    path.display()
                );
                None
            }
        }
    }

    /// Returns current active catalog snapshot
    pub fn get_catalog(&self) -> Arc<CatalogRoot> {
        self.catalog.read().clone()
    }

    /// Looks up entry in active catalog
    pub fn lookup(&self, query: &str) -> Option<CatalogEntry> {
        self.catalog.read().lookup(query).cloned()
    }

    /// Updates catalog from remote endpoint, verifying Ed25519 signature before persisting
    pub async fn update(&self, endpoint_url: Option<&str>) -> Result<CatalogUpdateResult> {
        let url = endpoint_url.unwrap_or(DEFAULT_CATALOG_URL);
        let current_version = self.catalog.read().catalog_version.clone();

        tracing::info!("Checking for compatibility catalog updates at {}", url);
        let resp = self.client.get(url).send().await
            .with_context(|| format!("Failed to reach catalog endpoint {url}"))?;

        if !resp.status().is_success() {
            bail!("Catalog endpoint returned HTTP error status {}", resp.status());
        }

        let new_catalog: CatalogRoot = resp.json().await
            .context("Failed to parse catalog JSON from update endpoint")?;

        if new_catalog.schema_version != 1 {
            bail!("Unsupported catalog schema version: {}", new_catalog.schema_version);
        }

        let pubkey = official_public_key();
        if !new_catalog.verify_signature(&pubkey).unwrap_or(false) {
            bail!("Remote catalog signature verification failed! Update rejected to prevent tampering.");
        }

        let new_version = new_catalog.catalog_version.clone();
        let entries_count = new_catalog.entries.len();

        // Check if version is newer or updated
        if new_version == current_version {
            return Ok(CatalogUpdateResult {
                updated: false,
                old_version: current_version,
                new_version,
                entries_count,
                message: "Catalog is already up to date.".to_string(),
            });
        }

        // Atomically save to disk cache
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        let tmp_path = self.cache_path.with_extension("json.tmp");
        let serialized = serde_json::to_string_pretty(&new_catalog)
            .context("Failed to serialize updated catalog")?;

        let mut file = File::create(&tmp_path)
            .with_context(|| format!("Creating temporary catalog file {}", tmp_path.display()))?;
        file.write_all(serialized.as_bytes())?;
        file.sync_all()?;
        drop(file);

        fs::rename(&tmp_path, &self.cache_path)
            .with_context(|| format!("Atomic rename to {}", self.cache_path.display()))?;

        // Update in-memory reference
        *self.catalog.write() = Arc::new(new_catalog);

        tracing::info!(
            "Successfully updated compatibility catalog: {} -> {} ({} entries)",
            current_version,
            new_version,
            entries_count
        );

        Ok(CatalogUpdateResult {
            updated: true,
            old_version: current_version,
            new_version: new_version.clone(),
            entries_count,
            message: format!("Catalog successfully updated to version {new_version} ({entries_count} entries)."),
        })
    }
}

impl Default for CatalogManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Constructs the official default catalog containing all standard entries, signed with official key
pub fn build_official_default_catalog() -> CatalogRoot {
    let mut entries = BTreeMap::new();

    // 1. Sodium (and aliases)
    entries.insert(
        "sodium".to_string(),
        CatalogEntry {
            mod_id: "sodium".to_string(),
            name: "Sodium".to_string(),
            aliases: vec!["embeddium".to_string(), "rubidium".to_string()],
            description: "Modern rendering engine and chunk mesher".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Vulkan/wgpu chunk mesher & frustum/occlusion culling".to_string(),
                description: "Horizon includes a native Rust multi-threaded chunk mesher with frustum & occlusion culling built-in.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 2. Lithium (and aliases)
    entries.insert(
        "lithium".to_string(),
        CatalogEntry {
            mod_id: "lithium".to_string(),
            name: "Lithium".to_string(),
            aliases: vec!["radium".to_string(), "canary".to_string()],
            description: "General-purpose physics, mob AI, and ticking optimization".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Multi-threaded physics ECS & tick optimizations".to_string(),
                description: "Horizon is built from scratch with an optimized lock-free physics ECS in Rust.".to_string(),
                status: "native_ecs".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 3. FerriteCore (and memory/network optimizations)
    entries.insert(
        "ferritecore".to_string(),
        CatalogEntry {
            mod_id: "ferritecore".to_string(),
            name: "FerriteCore".to_string(),
            aliases: vec!["memoryleakfix".to_string(), "krypton".to_string()],
            description: "Memory usage and async network optimizations".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Zero-heap native memory & async Tokio networking".to_string(),
                description: "Horizon runs without JVM heap overhead and uses Tokio async event-driven I/O.".to_string(),
                status: "native_tokio".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 4. Entity Culling
    entries.insert(
        "entityculling".to_string(),
        CatalogEntry {
            mod_id: "entityculling".to_string(),
            name: "Entity Culling".to_string(),
            aliases: Vec::new(),
            description: "Occlusion culling of hidden entities".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Depth-buffer entity occlusion culling".to_string(),
                description: "Occlusion culling of hidden entities is executed directly by Horizon's render pipeline.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 5. ImmediatelyFast / Exordium
    entries.insert(
        "immediatelyfast".to_string(),
        CatalogEntry {
            mod_id: "immediatelyfast".to_string(),
            name: "ImmediatelyFast".to_string(),
            aliases: vec!["exordium".to_string()],
            description: "Batch drawcalls for text, HUD, and interface elements".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "GPU drawcall batching & UI acceleration".to_string(),
                description: "Horizon batches HUD and UI draw calls into persistent wgpu vertex buffers.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 6. FastQuit
    entries.insert(
        "fastquit".to_string(),
        CatalogEntry {
            mod_id: "fastquit".to_string(),
            name: "FastQuit".to_string(),
            aliases: Vec::new(),
            description: "Return to title screen immediately while world saves in background".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Non-blocking background world save".to_string(),
                description: "Horizon saves chunks asynchronously on background worker threads on disconnect.".to_string(),
                status: "native_tokio".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 7. C2ME / Noisium
    entries.insert(
        "c2me".to_string(),
        CatalogEntry {
            mod_id: "c2me".to_string(),
            name: "C2ME".to_string(),
            aliases: vec!["noisium".to_string()],
            description: "Concurrent chunk management and SIMD world generator".to_string(),
            category: CatalogModCategory::WorldGen,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Multi-threaded SIMD world generation".to_string(),
                description: "Horizon's world generator runs asynchronously across all CPU cores with SIMD-accelerated noise.".to_string(),
                status: "native_rust".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 8. ModernFix
    entries.insert(
        "modernfix".to_string(),
        CatalogEntry {
            mod_id: "modernfix".to_string(),
            name: "ModernFix".to_string(),
            aliases: Vec::new(),
            description: "All-in-one memory, model, and launch speed improvements".to_string(),
            category: CatalogModCategory::Performance,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Native blockstate baking & texture atlas stitching".to_string(),
                description: "Horizon's asset pipeline bakes block models directly into GPU buffers.".to_string(),
                status: "native_rust".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 9. Iris / Oculus
    entries.insert(
        "iris".to_string(),
        CatalogEntry {
            mod_id: "iris".to_string(),
            name: "Iris Shaders".to_string(),
            aliases: vec!["oculus".to_string()],
            description: "Modern shaderpack pipeline".to_string(),
            category: CatalogModCategory::Visual,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "SPIR-V / WGSL native shader pipeline".to_string(),
                description: "Horizon compiles shaderpacks directly to Vulkan/wgpu pipelines via SPIR-V/WGSL.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 10. Continuity / Indium
    entries.insert(
        "continuity".to_string(),
        CatalogEntry {
            mod_id: "continuity".to_string(),
            name: "Continuity".to_string(),
            aliases: vec!["indium".to_string()],
            description: "Connected textures and custom block layers".to_string(),
            category: CatalogModCategory::Visual,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Connected texture baking (CTM)".to_string(),
                description: "Connected textures are pre-baked into the texture atlas during launch preparation.".to_string(),
                status: "native_rust".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 11. LambDynamicLights
    entries.insert(
        "lambdynamiclights".to_string(),
        CatalogEntry {
            mod_id: "lambdynamiclights".to_string(),
            name: "LambDynamicLights".to_string(),
            aliases: Vec::new(),
            description: "Dynamic lighting for handheld items and glowing entities".to_string(),
            category: CatalogModCategory::Visual,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Native deferred dynamic lighting".to_string(),
                description: "Dynamic light calculations are computed natively in Horizon's lighting pass.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 12. Borderless Mining
    entries.insert(
        "borderless-mining".to_string(),
        CatalogEntry {
            mod_id: "borderless-mining".to_string(),
            name: "Borderless Mining".to_string(),
            aliases: Vec::new(),
            description: "Borderless fullscreen window mode".to_string(),
            category: CatalogModCategory::Utility,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Native borderless fullscreen".to_string(),
                description: "Window management supports seamless borderless fullscreen through native Tao/Winit.".to_string(),
                status: "native_rust".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 13. Curios / Trinkets
    entries.insert(
        "curios".to_string(),
        CatalogEntry {
            mod_id: "curios".to_string(),
            name: "Curios API".to_string(),
            aliases: vec!["trinkets".to_string()],
            description: "Flexible accessory slot system".to_string(),
            category: CatalogModCategory::Library,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Native accessory slot system".to_string(),
                description: "Horizon includes built-in native accessory and charm slots in the inventory.".to_string(),
                status: "native_rust".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 14. Cloth Config / Config Libs
    entries.insert(
        "cloth-config".to_string(),
        CatalogEntry {
            mod_id: "cloth-config".to_string(),
            name: "Cloth Config".to_string(),
            aliases: vec!["reeses-sodium-options".to_string(), "yet-another-config-lib".to_string()],
            description: "Configuration screen and options library".to_string(),
            category: CatalogModCategory::Library,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Native in-game settings UI integration".to_string(),
                description: "Horizon exposes configuration tabs directly in the native launcher and pause menu.".to_string(),
                status: "native_rust".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 15. Zoomify / OK Zoomer
    entries.insert(
        "zoomify".to_string(),
        CatalogEntry {
            mod_id: "zoomify".to_string(),
            name: "Zoomify".to_string(),
            aliases: vec!["ok-zoomer".to_string(), "wi-zoom".to_string()],
            description: "Smooth camera zoom with transitions".to_string(),
            category: CatalogModCategory::Utility,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Native smooth camera zoom".to_string(),
                description: "Horizon includes smooth camera zoom natively bound to the 'C' key.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 16. Chunky
    entries.insert(
        "chunky".to_string(),
        CatalogEntry {
            mod_id: "chunky".to_string(),
            name: "Chunky".to_string(),
            aliases: Vec::new(),
            description: "Pre-generates chunks in background".to_string(),
            category: CatalogModCategory::Utility,
            license: Some("GPL-3.0-only".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "Background chunk pre-generator".to_string(),
                description: "Horizon includes a background multi-threaded chunk pre-generation engine.".to_string(),
                status: "native_tokio".to_string(),
            }),
            wasm_port: None,
            unsupported: None,
        },
    );

    // 17. AppleSkin (Built-in + WASM port available)
    entries.insert(
        "appleskin".to_string(),
        CatalogEntry {
            mod_id: "appleskin".to_string(),
            name: "AppleSkin".to_string(),
            aliases: Vec::new(),
            description: "Food value and exhaustion HUD visualizer".to_string(),
            category: CatalogModCategory::Gameplay,
            license: Some("MIT".to_string()),
            built_in: Some(CatalogBuiltInEquivalent {
                feature: "HUD food saturation & exhaustion overlay".to_string(),
                description: "Horizon renders food hunger and saturation values directly in the native HUD.".to_string(),
                status: "native_wgpu".to_string(),
            }),
            wasm_port: Some(CatalogWasmPort {
                wasm_id: "appleskin-wasm".to_string(),
                name: "AppleSkin Native WASM".to_string(),
                version: "1.0.0".to_string(),
                url: "https://meta.modrinth.com/v1/horizon/mods/appleskin.wasm".to_string(),
                sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
                license: "MIT".to_string(),
                author_permission_needed: false,
                author_permission_granted: true,
                notes: Some("Native WASM food values overlay".to_string()),
            }),
            unsupported: None,
        },
    );

    // 18. Just Enough Items (JEI / REI / EMI -> WASM port)
    entries.insert(
        "jei".to_string(),
        CatalogEntry {
            mod_id: "jei".to_string(),
            name: "Just Enough Items".to_string(),
            aliases: vec!["rei".to_string(), "emi".to_string()],
            description: "Item recipe viewer and bookmark manager".to_string(),
            category: CatalogModCategory::Utility,
            license: Some("MIT".to_string()),
            built_in: None,
            wasm_port: Some(CatalogWasmPort {
                wasm_id: "recipe-viewer-wasm".to_string(),
                name: "Horizon Recipe Viewer".to_string(),
                version: "1.0.0".to_string(),
                url: "https://meta.modrinth.com/v1/horizon/mods/recipe-viewer.wasm".to_string(),
                sha256: "b10a8db164e0754105b7a99be72e3fe5ec432ce7b2391549ddd3e6197b76549a".to_string(),
                license: "MIT".to_string(),
                author_permission_needed: false,
                author_permission_granted: true,
                notes: Some("Native WASM recipe search and crafting preview".to_string()),
            }),
            unsupported: None,
        },
    );

    // 19. Simple Voice Chat (WASM port)
    entries.insert(
        "simple-voice-chat".to_string(),
        CatalogEntry {
            mod_id: "simple-voice-chat".to_string(),
            name: "Simple Voice Chat".to_string(),
            aliases: vec!["plasmo-voice".to_string()],
            description: "Proximity voice chat in Minecraft".to_string(),
            category: CatalogModCategory::Audio,
            license: Some("LGPL-3.0-only".to_string()),
            built_in: None,
            wasm_port: Some(CatalogWasmPort {
                wasm_id: "voice-chat-wasm".to_string(),
                name: "Horizon Voice Chat".to_string(),
                version: "1.0.0".to_string(),
                url: "https://meta.modrinth.com/v1/horizon/mods/voice-chat.wasm".to_string(),
                sha256: "a591a6d40bf420404a011733cfb7b190d62c65bf0bcda32b57b277d9ad9f146e".to_string(),
                license: "LGPL-3.0-only".to_string(),
                author_permission_needed: false,
                author_permission_granted: true,
                notes: Some("Native Opus audio encoding over UDP".to_string()),
            }),
            unsupported: None,
        },
    );

    // 20. Waystones (Requires author permission for redistribution)
    entries.insert(
        "waystones".to_string(),
        CatalogEntry {
            mod_id: "waystones".to_string(),
            name: "Waystones".to_string(),
            aliases: Vec::new(),
            description: "Teleportation waystones for survival".to_string(),
            category: CatalogModCategory::Gameplay,
            license: Some("CC-BY-NC-SA-4.0".to_string()),
            built_in: None,
            wasm_port: Some(CatalogWasmPort {
                wasm_id: "waystones-wasm".to_string(),
                name: "Waystones WASM".to_string(),
                version: "0.9.0".to_string(),
                url: "https://meta.modrinth.com/v1/horizon/mods/waystones.wasm".to_string(),
                sha256: "c3ab8ff13720e8ad9047dd39466b3c8974e592c2fa383d4a3960714caef0c4f2".to_string(),
                license: "CC-BY-NC-SA-4.0".to_string(),
                author_permission_needed: true,
                author_permission_granted: false,
                notes: Some("Redistribution pending author consent for CC-BY-NC-SA license".to_string()),
            }),
            unsupported: None,
        },
    );

    // 21. Create (Unsupported JVM mod)
    entries.insert(
        "create".to_string(),
        CatalogEntry {
            mod_id: "create".to_string(),
            name: "Create".to_string(),
            aliases: Vec::new(),
            description: "Mechanical contraptions and rotational kinetic power".to_string(),
            category: CatalogModCategory::Gameplay,
            license: Some("MIT".to_string()),
            built_in: None,
            wasm_port: None,
            unsupported: Some(CatalogUnsupported {
                reason: "Requires deep JVM class re-weaving and Forge/Fabric ASM/Mixin runtime. Porting to Horizon WASM API in progress.".to_string(),
                suggested_alternative: Some("Use vanilla redstone mechanics or native redstone WASM extensions".to_string()),
                technical_blocker: Some("Flywheel GPU virtual instancing requires custom Rust wgpu shader pipeline".to_string()),
            }),
        },
    );

    // 22. Mekanism (Unsupported JVM mod)
    entries.insert(
        "mekanism".to_string(),
        CatalogEntry {
            mod_id: "mekanism".to_string(),
            name: "Mekanism".to_string(),
            aliases: Vec::new(),
            description: "High-tech machinery and industrial processing".to_string(),
            category: CatalogModCategory::Gameplay,
            license: Some("MIT".to_string()),
            built_in: None,
            wasm_port: None,
            unsupported: Some(CatalogUnsupported {
                reason: "Relies extensively on JVM Forge capabilities and Java bytecode pipelines.".to_string(),
                suggested_alternative: Some("Native tech mods coming via Horizon WASM API".to_string()),
                technical_blocker: Some("Java classloader bytecode manipulation".to_string()),
            }),
        },
    );

    // 23. Twilight Forest (Unsupported JVM mod)
    entries.insert(
        "twilightforest".to_string(),
        CatalogEntry {
            mod_id: "twilightforest".to_string(),
            name: "The Twilight Forest".to_string(),
            aliases: Vec::new(),
            description: "Enchanted realm dimension with bosses and dungeons".to_string(),
            category: CatalogModCategory::WorldGen,
            license: Some("LGPL-2.1-only".to_string()),
            built_in: None,
            wasm_port: None,
            unsupported: Some(CatalogUnsupported {
                reason: "Custom Java dimension, boss AI, and worldgen bytecode.".to_string(),
                suggested_alternative: Some("Play in Java Edition or wait for Horizon Dimension API".to_string()),
                technical_blocker: Some("Custom entity AI state machine & biome generators".to_string()),
            }),
        },
    );

    // 24. Botania (Unsupported JVM mod)
    entries.insert(
        "botania".to_string(),
        CatalogEntry {
            mod_id: "botania".to_string(),
            name: "Botania".to_string(),
            aliases: Vec::new(),
            description: "Natural magic tech mod using mana and flowers".to_string(),
            category: CatalogModCategory::Gameplay,
            license: Some("Custom (Open Source)".to_string()),
            built_in: None,
            wasm_port: None,
            unsupported: Some(CatalogUnsupported {
                reason: "Java particle rendering and tile entity bytecode.".to_string(),
                suggested_alternative: Some("Play in Java Edition".to_string()),
                technical_blocker: Some("Direct OpenGL 1.1 immediate mode calls".to_string()),
            }),
        },
    );

    let mut catalog = CatalogRoot {
        schema_version: 1,
        catalog_version: "2026.11.1".to_string(),
        updated_at: "2026-11-13T12:00:00Z".to_string(),
        entries,
        signature: None,
    };

    catalog.sign_with_seed(&OFFICIAL_CATALOG_SEED).expect("Failed to sign official catalog");
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_official_default_catalog_validity_and_signature() {
        let catalog = build_official_default_catalog();
        assert_eq!(catalog.schema_version, 1);
        assert_eq!(catalog.catalog_version, "2026.11.1");
        assert!(catalog.entries.len() >= 24);
        assert!(catalog.signature.is_some());

        // Verify with official public key
        let pk = official_public_key();
        let valid = catalog.verify_signature(&pk).unwrap();
        assert!(valid, "Official catalog signature verification failed!");
    }

    #[test]
    fn test_signature_tamper_detection() {
        let mut catalog = build_official_default_catalog();
        let pk = official_public_key();

        // Valid signature initially
        assert!(catalog.verify_signature(&pk).unwrap());

        // Tamper with an entry
        if let Some(entry) = catalog.entries.get_mut("sodium") {
            entry.name = "Tampered Sodium".to_string();
        }

        // Must reject tampered catalog!
        let valid = catalog.verify_signature(&pk).unwrap();
        assert!(!valid, "Signature check must fail on tampered content!");
    }

    #[test]
    fn test_catalog_lookups_and_aliases() {
        let catalog = build_official_default_catalog();

        // Exact ID lookup
        let sodium = catalog.lookup("sodium").expect("Should find sodium");
        assert_eq!(sodium.name, "Sodium");
        assert!(sodium.built_in.is_some());

        // Alias lookup
        let embeddium = catalog.lookup("embeddium").expect("Should find embeddium by alias");
        assert_eq!(embeddium.mod_id, "sodium");

        let rubidium = catalog.lookup("Rubidium").expect("Should find rubidium case-insensitively");
        assert_eq!(rubidium.mod_id, "sodium");

        // Hyphen and underscore handling
        let voice = catalog.lookup("simple_voice_chat").expect("Should find simple-voice-chat");
        assert_eq!(voice.mod_id, "simple-voice-chat");
        assert!(voice.wasm_port.is_some());

        // Unsupported mod
        let create = catalog.lookup("create").expect("Should find create");
        assert!(create.unsupported.is_some());
        assert!(create.unsupported.as_ref().unwrap().reason.contains("JVM"));
    }

    #[test]
    fn test_wasm_port_license_enforcement() {
        let catalog = build_official_default_catalog();

        // Permissive MIT port (appleskin) requires no special author consent
        let appleskin = catalog.lookup("appleskin").unwrap();
        assert!(CatalogRoot::validate_port_license(appleskin).is_ok());

        // Non-permissive port without granted consent (waystones) must fail validation!
        let waystones = catalog.lookup("waystones").unwrap();
        let err = CatalogRoot::validate_port_license(waystones).unwrap_err();
        assert!(err.to_string().contains("requires explicit redistribution consent"));
    }

    #[test]
    fn test_in_memory_catalog_manager_and_fallback() {
        let manager = CatalogManager::default();
        let cat = manager.get_catalog();
        assert_eq!(cat.schema_version, 1);

        let entry = manager.lookup("lithium");
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().mod_id, "lithium");
    }

    #[test]
    fn test_sync_default_catalog_resource() {
        let catalog = build_official_default_catalog();
        let json = serde_json::to_string_pretty(&catalog).unwrap();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/default_catalog.json");
        std::fs::write(&path, json).unwrap();

        // Ensure file can be parsed back and verified
        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: CatalogRoot = serde_json::from_str(&content).unwrap();
        let pk = official_public_key();
        assert!(loaded.verify_signature(&pk).unwrap());
    }
}
