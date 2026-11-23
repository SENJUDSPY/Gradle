//! Mod JAR inspector, metadata parser, and compatibility classifier for Horizon
//! (Minecraft 26.3 rewritten in native Rust).
//!
//! Evaluates Fabric, Quilt, Forge, NeoForge, and Horizon WASM mod archives (.jar, .wasm, .hmod)
//! to classify them into:
//! - RESOURCE_ONLY: 0 JVM bytecode classes, pure assets/datapacks/shaders
//! - BUILT_IN_EQUIVALENT: Feature implemented natively in Horizon's Rust/wgpu engine
//! - WASM_AVAILABLE: Native WebAssembly module (.wasm, .hmod, or horizon-mod.toml)
//! - UNSUPPORTED: Requires Java Virtual Machine (JVM) with bytecode/Mixins/entrypoints
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek};
use std::path::Path;
use zip::ZipArchive;

/// Mod loader detected from jar manifest and entrypoint metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetectedLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
    HorizonWasm,
    Unknown,
}

impl DetectedLoader {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Fabric => "Fabric",
            Self::Quilt => "Quilt",
            Self::Forge => "Forge",
            Self::NeoForge => "NeoForge",
            Self::HorizonWasm => "Horizon (WASM)",
            Self::Unknown => "Unknown / Legacy",
        }
    }
}

/// The 4 strict compatibility buckets defined for Horizon
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[derive(Default)]
pub enum CompatibilityBucket {
    #[serde(rename = "RESOURCE_ONLY")]
    ResourceOnly,
    #[serde(rename = "CODE_WITH_RECOVERABLE_ASSETS")]
    CodeWithRecoverableAssets,
    #[serde(rename = "NATIVE_PORT_AVAILABLE")]
    NativePortAvailable,
    #[serde(rename = "UNSUPPORTED")]
    #[default]
    Unsupported,
}


impl CompatibilityBucket {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ResourceOnly => "RESOURCE_ONLY",
            Self::CodeWithRecoverableAssets => "CODE_WITH_RECOVERABLE_ASSETS",
            Self::NativePortAvailable => "NATIVE_PORT_AVAILABLE",
            Self::Unsupported => "UNSUPPORTED",
        }
    }
}

/// Detailed classification category for each inspected mod
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "details")]
pub enum ModClassification {
    #[serde(rename = "RESOURCE_ONLY")]
    ResourceOnly {
        reason: String,
        assets_found: Vec<String>,
    },
    #[serde(rename = "CODE_WITH_RECOVERABLE_ASSETS")]
    CodeWithRecoverableAssets {
        reason: String,
        assets_found: Vec<String>,
        class_count: usize,
        recoverable_summary: String,
    },
    #[serde(rename = "NATIVE_PORT_AVAILABLE")]
    NativePortAvailable {
        wasm_id: String,
        reason: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        port_url: Option<String>,
        #[serde(default = "default_version")]
        version: String,
    },
    #[serde(rename = "BUILT_IN_EQUIVALENT")]
    BuiltInEquivalent {
        feature: String,
        reason: String,
    },
    #[serde(rename = "WASM_AVAILABLE")]
    WasmAvailable {
        wasm_id: String,
        reason: String,
    },
    #[serde(rename = "UNSUPPORTED")]
    Unsupported {
        reason: String,
        class_count: usize,
        mixin_count: usize,
    },
}

fn default_version() -> String {
    "1.0.0".to_string()
}

impl ModClassification {
    pub fn bucket(&self) -> CompatibilityBucket {
        match self {
            Self::ResourceOnly { .. } => CompatibilityBucket::ResourceOnly,
            Self::CodeWithRecoverableAssets { .. } => CompatibilityBucket::CodeWithRecoverableAssets,
            Self::NativePortAvailable { .. }
            | Self::BuiltInEquivalent { .. }
            | Self::WasmAvailable { .. } => CompatibilityBucket::NativePortAvailable,
            Self::Unsupported { .. } => CompatibilityBucket::Unsupported,
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(
            self,
            Self::ResourceOnly { .. }
                | Self::BuiltInEquivalent { .. }
                | Self::WasmAvailable { .. }
                | Self::NativePortAvailable { .. }
        )
    }

    pub fn is_partial(&self) -> bool {
        matches!(self, Self::CodeWithRecoverableAssets { .. })
    }

    pub fn category_label(&self) -> &'static str {
        match self {
            Self::ResourceOnly { .. } => "Resource Only",
            Self::CodeWithRecoverableAssets { .. } => "Recoverable Assets (Code Skipped)",
            Self::NativePortAvailable { .. } | Self::WasmAvailable { .. } => "WASM Native Port",
            Self::BuiltInEquivalent { .. } => "Built-in Equivalent",
            Self::Unsupported { .. } => "Needs Java (JVM)",
        }
    }
}

/// Metadata and compatibility analysis for a single mod archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModJarAnalysis {
    pub file_name: String,
    pub mod_id: String,
    pub name: String,
    pub version: String,
    pub loader: DetectedLoader,
    pub minecraft_version: Option<String>,
    pub entrypoints: Vec<String>,
    pub mixins: Vec<String>,
    pub dependencies: HashMap<String, String>,
    pub class_count: usize,
    pub is_resource_only: bool,
    pub classification: ModClassification,
    #[serde(default)]
    pub bucket: CompatibilityBucket,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_entry: Option<crate::catalog::CatalogEntry>,
}

/// Compatibility report across all mods in an instance or modpack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceModCompatibilityReport {
    pub total_mods: usize,
    pub supported_count: usize,
    pub skipped_count: usize,
    #[serde(default)]
    pub resource_only_count: usize,
    #[serde(default)]
    pub native_port_count: usize,
    #[serde(default)]
    pub recoverable_assets_count: usize,
    #[serde(default)]
    pub unsupported_count: usize,
    pub summary: String,
    #[serde(default)]
    pub breakdown_summary: String,
    pub can_launch_native: bool,
    pub mods: Vec<ModJarAnalysis>,
}

/// Analyzes an archive from an arbitrary reader and returns structured metadata + classification
pub fn analyze_jar_reader<R: Read + Seek>(
    file_name: &str,
    mut archive: ZipArchive<R>,
    _file_size: Option<u64>,
) -> Result<ModJarAnalysis> {
    // Inspect archive contents (scan for bytecode, assets, and shaders)
    let mut class_count = 0usize;
    let mut assets_found = Vec::new();
    let mut has_pack_mcmeta = false;
    let mut has_fabric_json = false;
    let mut has_quilt_json = false;
    let mut has_forge_toml = false;
    let mut has_neoforge_toml = false;
    let mut has_horizon_toml = false;
    let mut has_manifest_mf = false;
    let mut has_mcmod_info = false;

    for i in 0..archive.len() {
        if let Ok(entry) = archive.by_index(i) {
            let entry_name = entry.name().to_string();
            let lower_entry = entry_name.to_lowercase();

            if lower_entry.ends_with(".class") {
                class_count += 1;
            } else if lower_entry == "pack.mcmeta" {
                has_pack_mcmeta = true;
            } else if lower_entry.starts_with("assets/")
                && !assets_found.contains(&"assets/".to_string())
            {
                assets_found.push("assets/".to_string());
            } else if lower_entry.starts_with("data/")
                && !assets_found.contains(&"data/".to_string())
            {
                assets_found.push("data/".to_string());
            } else if lower_entry.starts_with("shaders/")
                && !assets_found.contains(&"shaders/".to_string())
            {
                assets_found.push("shaders/".to_string());
            }

            match entry_name.as_str() {
                "fabric.mod.json" => has_fabric_json = true,
                "quilt.mod.json" => has_quilt_json = true,
                "META-INF/mods.toml" => has_forge_toml = true,
                "META-INF/neoforge.mods.toml" => has_neoforge_toml = true,
                "horizon-mod.toml" => has_horizon_toml = true,
                "META-INF/MANIFEST.MF" => has_manifest_mf = true,
                "mcmod.info" => has_mcmod_info = true,
                _ => {}
            }
        }
    }

    if has_pack_mcmeta && !assets_found.contains(&"pack.mcmeta".to_string()) {
        assets_found.push("pack.mcmeta".to_string());
    }

    let is_resource_only = class_count == 0 && !assets_found.is_empty();

    // Extract manifest metadata based on detected loader
    let mut mod_id = String::new();
    let mut mod_name = String::new();
    let mut mod_version = String::new();
    let mut loader = DetectedLoader::Unknown;
    let mut minecraft_version = None;
    let mut entrypoints = Vec::new();
    let mut mixins = Vec::new();
    let mut dependencies = HashMap::new();

    if has_horizon_toml {
        loader = DetectedLoader::HorizonWasm;
        if let Some(text) = read_entry_string(&mut archive, "horizon-mod.toml")
            && let Ok(toml_val) = toml::from_str::<toml::Value>(&text) {
                if let Some(id) = toml_val.get("id").and_then(|v| v.as_str()) {
                    mod_id = id.to_string();
                }
                if let Some(name) = toml_val.get("name").and_then(|v| v.as_str()) {
                    mod_name = name.to_string();
                }
                if let Some(ver) = toml_val.get("version").and_then(|v| v.as_str()) {
                    mod_version = ver.to_string();
                }
            }
    } else if has_fabric_json {
        loader = DetectedLoader::Fabric;
        if let Some(text) = read_entry_string(&mut archive, "fabric.mod.json")
            && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                parse_fabric_json(
                    &json_val,
                    &mut mod_id,
                    &mut mod_name,
                    &mut mod_version,
                    &mut minecraft_version,
                    &mut entrypoints,
                    &mut mixins,
                    &mut dependencies,
                );
            }
    } else if has_quilt_json {
        loader = DetectedLoader::Quilt;
        if let Some(text) = read_entry_string(&mut archive, "quilt.mod.json")
            && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                parse_quilt_json(
                    &json_val,
                    &mut mod_id,
                    &mut mod_name,
                    &mut mod_version,
                    &mut minecraft_version,
                    &mut entrypoints,
                    &mut mixins,
                    &mut dependencies,
                );
            }
    } else if has_neoforge_toml || has_forge_toml {
        loader = if has_neoforge_toml {
            DetectedLoader::NeoForge
        } else {
            DetectedLoader::Forge
        };
        let path = if has_neoforge_toml {
            "META-INF/neoforge.mods.toml"
        } else {
            "META-INF/mods.toml"
        };
        if let Some(text) = read_entry_string(&mut archive, path) {
            match toml::from_str::<toml::Value>(&text) {
                Ok(toml_val) => {
                    parse_forge_toml(
                        &toml_val,
                        &mut mod_id,
                        &mut mod_name,
                        &mut mod_version,
                        &mut minecraft_version,
                        &mut entrypoints,
                        &mut mixins,
                        &mut dependencies,
                    );
                }
                Err(err) => {
                    tracing::warn!("Failed to parse {path}: {err}");
                }
            }
        }
    } else if has_mcmod_info {
        loader = DetectedLoader::Forge;
        if let Some(text) = read_entry_string(&mut archive, "mcmod.info")
            && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                parse_mcmod_info(
                    &json_val,
                    &mut mod_id,
                    &mut mod_name,
                    &mut mod_version,
                    &mut minecraft_version,
                    &mut dependencies,
                );
            }
    }

    // Check MANIFEST.MF for extra Mixin configurations if not found yet
    if has_manifest_mf && mixins.is_empty()
        && let Some(manifest_text) = read_entry_string(&mut archive, "META-INF/MANIFEST.MF") {
            for line in manifest_text.lines() {
                if let Some(mixins_val) = line.strip_prefix("MixinConfigs:") {
                    for m in mixins_val.split(',') {
                        let trimmed = m.trim();
                        if !trimmed.is_empty() && !mixins.contains(&trimmed.to_string()) {
                            mixins.push(trimmed.to_string());
                        }
                    }
                }
            }
        }

    // Fallbacks if mod id / name / version were absent
    if mod_id.is_empty() {
        mod_id = infer_mod_id_from_filename(file_name);
    }
    if mod_name.is_empty() {
        mod_name = mod_id.clone();
    }
    if mod_version.is_empty() {
        mod_version = "1.0.0".to_string();
    }

    // Lookup in compatibility catalog
    let catalog_entry = lookup_in_catalog(&mod_id, &mod_name, file_name);

    // Classify the mod
    let classification = classify_mod(
        &mod_id,
        &mod_name,
        file_name,
        loader,
        is_resource_only,
        &assets_found,
        class_count,
        &entrypoints,
        &mixins,
        catalog_entry.as_ref(),
    );

    let bucket = classification.bucket();

    Ok(ModJarAnalysis {
        file_name: file_name.to_string(),
        mod_id,
        name: mod_name,
        version: mod_version,
        loader,
        minecraft_version,
        entrypoints,
        mixins,
        dependencies,
        class_count,
        is_resource_only,
        classification,
        bucket,
        catalog_entry,
    })
}

/// Cached classification results for rapid repeat scans
static SCAN_CACHE: std::sync::OnceLock<parking_lot::RwLock<HashMap<String, ModJarAnalysis>>> = std::sync::OnceLock::new();

fn get_scan_cache() -> &'static parking_lot::RwLock<HashMap<String, ModJarAnalysis>> {
    SCAN_CACHE.get_or_init(|| {
        let cache_file = crate::releases::horizon_data_dir().join("compatibility_scan_cache.json");
        let mut map = HashMap::new();
        if cache_file.is_file()
            && let Ok(data) = fs::read_to_string(&cache_file)
                && let Ok(deserialized) = serde_json::from_str::<HashMap<String, ModJarAnalysis>>(&data) {
                    map = deserialized;
                }
        parking_lot::RwLock::new(map)
    })
}

pub fn get_cached_analysis(cache_key: &str) -> Option<ModJarAnalysis> {
    get_scan_cache().read().get(cache_key).cloned()
}

pub fn put_cached_analysis(cache_key: String, analysis: ModJarAnalysis) {
    let mut cache = get_scan_cache().write();
    cache.insert(cache_key, analysis);
    let cache_file = crate::releases::horizon_data_dir().join("compatibility_scan_cache.json");
    if let Some(parent) = cache_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string(&*cache) {
        let _ = fs::write(cache_file, data);
    }
}

pub fn clear_compatibility_scan_cache() {
    let mut cache = get_scan_cache().write();
    cache.clear();
    let cache_file = crate::releases::horizon_data_dir().join("compatibility_scan_cache.json");
    let _ = fs::remove_file(cache_file);
}

/// Analyzes a mod jar or wasm file on disk
pub fn analyze_jar_file(path: &Path) -> Result<ModJarAnalysis> {
    if !path.is_file() {
        bail!("Mod file does not exist at {}", path.display());
    }

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown.jar");

    let lower_name = file_name.to_lowercase();
    if lower_name.ends_with(".wasm") || lower_name.ends_with(".hmod") {
        let mod_id = file_name
            .trim_end_matches(".wasm")
            .trim_end_matches(".hmod")
            .to_string();
        let catalog_entry = lookup_in_catalog(&mod_id, &mod_id, file_name);
        let classification = ModClassification::WasmAvailable {
            wasm_id: mod_id.clone(),
            reason: "Native WebAssembly mod executes directly on Horizon engine".to_string(),
        };
        let bucket = classification.bucket();
        return Ok(ModJarAnalysis {
            file_name: file_name.to_string(),
            mod_id: mod_id.clone(),
            name: mod_id,
            version: "1.0.0".to_string(),
            loader: DetectedLoader::HorizonWasm,
            minecraft_version: Some("26.3".to_string()),
            entrypoints: vec!["wasm_main".to_string()],
            mixins: Vec::new(),
            dependencies: HashMap::new(),
            class_count: 0,
            is_resource_only: false,
            classification,
            bucket,
            catalog_entry,
        });
    }

    let file = File::open(path)
        .with_context(|| format!("Opening mod file {}", path.display()))?;
    let metadata = file.metadata().ok();
    let file_size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
    let mtime = metadata
        .as_ref()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let cache_key = format!("{}:{}:{}", path.display(), file_size, mtime);
    if let Some(cached) = get_cached_analysis(&cache_key) {
        return Ok(cached);
    }

    let archive = ZipArchive::new(file)
        .with_context(|| format!("Reading zip archive from {}", path.display()))?;

    let analysis = analyze_jar_reader(file_name, archive, Some(file_size))?;
    put_cached_analysis(cache_key, analysis.clone());
    Ok(analysis)
}

/// Classifies all active mods located within an instance's `mods` folder
pub fn classify_instance_mods_dir(instance_mods_dir: &Path) -> Result<InstanceModCompatibilityReport> {
    if !instance_mods_dir.is_dir() {
        return Ok(InstanceModCompatibilityReport {
            total_mods: 0,
            supported_count: 0,
            skipped_count: 0,
            resource_only_count: 0,
            native_port_count: 0,
            recoverable_assets_count: 0,
            unsupported_count: 0,
            summary: "0 of 0 mods supported, 0 skipped".to_string(),
            breakdown_summary: "0 resource-only, 0 natively ported, 0 unsupported".to_string(),
            can_launch_native: true,
            mods: Vec::new(),
        });
    }

    let mut results = Vec::new();
    let entries = fs::read_dir(instance_mods_dir)
        .with_context(|| format!("Reading mods dir {}", instance_mods_dir.display()))?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().to_string();
        let lower = file_name.to_lowercase();

        // Skip disabled mods (.disabled) or non-mod files
        if lower.ends_with(".disabled") {
            continue;
        }
        if !lower.ends_with(".jar") && !lower.ends_with(".wasm") && !lower.ends_with(".hmod") {
            continue;
        }

        match analyze_jar_file(&path) {
            Ok(analysis) => results.push(analysis),
            Err(e) => {
                tracing::warn!("Failed to analyze mod {}: {:#}", path.display(), e);
                // Create an unsupported fallback record
                let mod_id = infer_mod_id_from_filename(&file_name);
                let classification = ModClassification::Unsupported {
                    reason: format!("Failed to read archive: {e}"),
                    class_count: 1,
                    mixin_count: 0,
                };
                let bucket = classification.bucket();
                results.push(ModJarAnalysis {
                    file_name: file_name.clone(),
                    mod_id: mod_id.clone(),
                    name: mod_id,
                    version: "unknown".to_string(),
                    loader: DetectedLoader::Unknown,
                    minecraft_version: None,
                    entrypoints: Vec::new(),
                    mixins: Vec::new(),
                    dependencies: HashMap::new(),
                    class_count: 1,
                    is_resource_only: false,
                    classification,
                    bucket,
                    catalog_entry: None,
                });
            }
        }
    }

    Ok(generate_compatibility_report(results))
}

/// Aggregates individual mod analysis results into an instance compatibility report
pub fn generate_compatibility_report(mut mods: Vec<ModJarAnalysis>) -> InstanceModCompatibilityReport {
    // Sort mods: supported first, then recoverable assets, then unsupported, then by name
    mods.sort_by(|a, b| {
        let a_rank = if a.classification.is_supported() {
            0
        } else if a.classification.is_partial() {
            1
        } else {
            2
        };
        let b_rank = if b.classification.is_supported() {
            0
        } else if b.classification.is_partial() {
            1
        } else {
            2
        };
        a_rank.cmp(&b_rank).then_with(|| a.name.cmp(&b.name))
    });

    let total_mods = mods.len();
    let supported_count = mods.iter().filter(|m| m.classification.is_supported()).count();
    let skipped_count = total_mods.saturating_sub(supported_count);

    let mut resource_only_count = 0usize;
    let mut native_port_count = 0usize;
    let mut recoverable_assets_count = 0usize;
    let mut unsupported_count = 0usize;

    for m in &mods {
        match m.bucket {
            CompatibilityBucket::ResourceOnly => resource_only_count += 1,
            CompatibilityBucket::NativePortAvailable => native_port_count += 1,
            CompatibilityBucket::CodeWithRecoverableAssets => recoverable_assets_count += 1,
            CompatibilityBucket::Unsupported => unsupported_count += 1,
        }
    }

    let summary = format!("{supported_count} of {total_mods} mods supported, {skipped_count} skipped");
    let breakdown_summary = if recoverable_assets_count > 0 {
        format!("{resource_only_count} resource-only, {native_port_count} natively ported, {recoverable_assets_count} with recoverable assets, {unsupported_count} unsupported (listed)")
    } else {
        format!("{resource_only_count} resource-only, {native_port_count} natively ported, {unsupported_count} unsupported (listed)")
    };

    let can_launch_native = skipped_count == 0;

    InstanceModCompatibilityReport {
        total_mods,
        supported_count,
        skipped_count,
        resource_only_count,
        native_port_count,
        recoverable_assets_count,
        unsupported_count,
        summary,
        breakdown_summary,
        can_launch_native,
        mods,
    }
}

/// Generates a modpack-level compatibility report before install or launch
pub fn generate_modpack_compatibility_report(
    modpack_name: &str,
    mods: Vec<ModJarAnalysis>,
) -> InstanceModCompatibilityReport {
    let mut report = generate_compatibility_report(mods);
    if report.recoverable_assets_count > 0 {
        report.breakdown_summary = format!("{modpack_name}: {} resource-only, {} natively ported, {} with recoverable assets, {} unsupported (listed)",
            report.resource_only_count, report.native_port_count, report.recoverable_assets_count, report.unsupported_count);
    } else {
        report.breakdown_summary = format!("{modpack_name}: {} resource-only, {} natively ported, {} unsupported (listed)",
            report.resource_only_count, report.native_port_count, report.unsupported_count);
    }
    report
}

// ---------------------------------------------------------------------------
// Classification logic & Catalog Lookup
// ---------------------------------------------------------------------------

/// Consults the active compatibility catalog for an entry matching the mod
pub fn lookup_in_catalog(mod_id: &str, name: &str, file_name: &str) -> Option<crate::catalog::CatalogEntry> {
    static CATALOG: std::sync::OnceLock<crate::catalog::CatalogRoot> = std::sync::OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(crate::catalog::BUNDLED_CATALOG_JSON)
            .unwrap_or_else(|_| crate::catalog::build_official_default_catalog())
    });

    catalog.lookup(mod_id)
        .or_else(|| catalog.lookup(name))
        .or_else(|| {
            let stem = file_name
                .strip_suffix(".jar")
                .or_else(|| file_name.strip_suffix(".wasm"))
                .or_else(|| file_name.strip_suffix(".hmod"))
                .unwrap_or(file_name);
            catalog.lookup(stem)
        })
        .cloned()
}

#[allow(clippy::too_many_arguments)]
fn classify_mod(
    mod_id: &str,
    name: &str,
    file_name: &str,
    loader: DetectedLoader,
    is_resource_only: bool,
    assets_found: &[String],
    class_count: usize,
    entrypoints: &[String],
    mixins: &[String],
    catalog_entry: Option<&crate::catalog::CatalogEntry>,
) -> ModClassification {
    // 1. WASM native mod
    if loader == DetectedLoader::HorizonWasm
        || file_name.to_lowercase().ends_with(".wasm")
        || file_name.to_lowercase().ends_with(".hmod")
    {
        return ModClassification::WasmAvailable {
            wasm_id: mod_id.to_string(),
            reason: "Executes natively via Horizon WebAssembly runtime".to_string(),
        };
    }

    // 2. Pure resource / data pack archive (zero JVM bytecode classes)
    if is_resource_only {
        return ModClassification::ResourceOnly {
            reason: format!(
                "Asset & Data mod containing 0 Java classes. Resources ({}) run natively in Horizon.",
                assets_found.join(", ")
            ),
            assets_found: assets_found.to_vec(),
        };
    }

    // 3. Compatibility catalog matching
    if let Some(entry) = catalog_entry {
        if let Some(built_in) = &entry.built_in {
            return ModClassification::BuiltInEquivalent {
                feature: built_in.feature.clone(),
                reason: built_in.description.clone(),
            };
        }
        if let Some(wasm_port) = &entry.wasm_port {
            let consent = if wasm_port.author_permission_needed && !wasm_port.author_permission_granted {
                " [Requires author redistribution consent]"
            } else {
                ""
            };
            return ModClassification::NativePortAvailable {
                wasm_id: wasm_port.wasm_id.clone(),
                reason: format!(
                    "Native WebAssembly port available: {} v{} (License: {}){}",
                    wasm_port.name, wasm_port.version, wasm_port.license, consent
                ),
                port_url: Some(wasm_port.url.clone()),
                version: wasm_port.version.clone(),
            };
        }
        if let Some(unsupported) = &entry.unsupported {
            if !assets_found.is_empty() {
                return ModClassification::CodeWithRecoverableAssets {
                    reason: format!(
                        "Java mod with {class_count} classes (catalog note: {}). Assets ({}) imported via Phase 1; bytecode logic not supported.",
                        unsupported.reason,
                        assets_found.join(", ")
                    ),
                    assets_found: assets_found.to_vec(),
                    class_count,
                    recoverable_summary: format!("{class_count} Java classes skipped, assets recovered"),
                };
            }
            let alt = match &unsupported.suggested_alternative {
                Some(a) => format!(" Suggested alternative: {a}."),
                None => String::new(),
            };
            return ModClassification::Unsupported {
                reason: format!("{}.{alt}", unsupported.reason),
                class_count,
                mixin_count: mixins.len(),
            };
        }
    }

    // 4. Built-in feature equivalent fallback table
    if let Some((feature, reason)) = lookup_built_in_equivalent(mod_id, name, file_name) {
        return ModClassification::BuiltInEquivalent { feature, reason };
    }

    // 5. Code with recoverable assets (has .class files but also non-code assets/data)
    if !assets_found.is_empty() {
        return ModClassification::CodeWithRecoverableAssets {
            reason: format!(
                "Contains {class_count} compiled Java classes (logic not supported on native engine), but non-code assets ({}) can be recovered via Phase 1.",
                assets_found.join(", ")
            ),
            assets_found: assets_found.to_vec(),
            class_count,
            recoverable_summary: format!("{class_count} Java classes skipped, assets recovered"),
        };
    }

    // 6. Unsupported Java bytecode mod (pure logic, no assets/data, no native port)
    let mixin_count = mixins.len();
    let entrypoint_count = entrypoints.len();

    let mut details = format!(
        "Requires Java Virtual Machine (JVM). Pure logic mod with {class_count} compiled Java classes"
    );

    if mixin_count > 0 {
        details.push_str(&format!(", {mixin_count} Mixin configurations ({})", mixins.join(", ")));
    }
    if entrypoint_count > 0 {
        details.push_str(&format!(", {entrypoint_count} entrypoints"));
    }
    details.push_str(" and 0 recoverable assets. Horizon executes native Rust without a JVM.");

    ModClassification::Unsupported {
        reason: details,
        class_count,
        mixin_count,
    }
}

/// Catalog of mod IDs and aliases that have built-in native implementations in Horizon
fn lookup_built_in_equivalent(
    mod_id: &str,
    name: &str,
    file_name: &str,
) -> Option<(String, String)> {
    let clean_id = mod_id.to_lowercase().replace(['-', '_'], "");
    let clean_name = name.to_lowercase().replace(['-', '_', ' '], "");
    let clean_file = file_name.to_lowercase().replace(['-', '_'], "");

    let matches = |key: &str| {
        clean_id == key
            || clean_name == key
            || clean_id.contains(key)
            || clean_name.contains(key)
            || clean_file.starts_with(key)
    };

    // Rendering & Optimization
    if matches("sodium") || matches("embeddium") || matches("rubidium") {
        return Some((
            "Vulkan/wgpu chunk mesher & frustum/occlusion culling".to_string(),
            "Horizon includes a native Rust multi-threaded chunk mesher with frustum & occlusion culling built-in."
                .to_string(),
        ));
    }
    if matches("lithium") || matches("radium") || matches("canary") {
        return Some((
            "Multi-threaded physics ECS & tick optimizations".to_string(),
            "Horizon is built from scratch with an optimized lock-free physics ECS in Rust.".to_string(),
        ));
    }
    if matches("ferritecore") || matches("memoryleakfix") || matches("krypton") {
        return Some((
            "Zero-heap native memory & async Tokio networking".to_string(),
            "Horizon runs without JVM heap overhead and uses Tokio async event-driven I/O.".to_string(),
        ));
    }
    if matches("entityculling") {
        return Some((
            "Depth-buffer entity occlusion culling".to_string(),
            "Occlusion culling of hidden entities is executed directly by Horizon's render pipeline."
                .to_string(),
        ));
    }
    if matches("immediatelyfast") || matches("exordium") {
        return Some((
            "GPU drawcall batching & UI acceleration".to_string(),
            "Horizon batches HUD and UI draw calls into persistent wgpu vertex buffers.".to_string(),
        ));
    }
    if matches("fastquit") {
        return Some((
            "Non-blocking background world save".to_string(),
            "Horizon saves chunks asynchronously on background worker threads on disconnect.".to_string(),
        ));
    }
    if matches("c2me") || matches("noisium") {
        return Some((
            "Multi-threaded SIMD world generation".to_string(),
            "Horizon's world generator runs asynchronously across all CPU cores with SIMD-accelerated noise."
                .to_string(),
        ));
    }
    if matches("modernfix") {
        return Some((
            "Native blockstate baking & texture atlas stitching".to_string(),
            "Horizon's asset pipeline bakes block models directly into GPU buffers.".to_string(),
        ));
    }

    // Shaders & Visuals
    if matches("iris") || matches("oculus") {
        return Some((
            "Native WGSL/SPIR-V shaderpack pipeline".to_string(),
            "Horizon provides native hardware deferred rendering and shadow mapping for shaderpacks."
                .to_string(),
        ));
    }
    if matches("continuity") || matches("connectedtextures") || matches("ctm") {
        return Some((
            "Connected texture model baking".to_string(),
            "Horizon natively compiles connected textures into geometry arrays.".to_string(),
        ));
    }
    if matches("lambdynamiclights") || matches("dynamiclights") {
        return Some((
            "Real-time dynamic lighting".to_string(),
            "Horizon computes real-time dynamic light sources in its compute shader lighting grid."
                .to_string(),
        ));
    }

    // Quality of Life & Controls
    if matches("zoomify") || matches("okzoomer") || matches("logicalzoom") || matches("wizoom") {
        return Some((
            "Smooth camera zoom".to_string(),
            "Horizon includes a built-in smooth camera zoom bound to 'C' (configurable in Settings)."
                .to_string(),
        ));
    }
    if matches("borderlessmining") {
        return Some((
            "Borderless fullscreen windowing".to_string(),
            "Horizon supports true borderless fullscreen natively via winit/SDL windowing.".to_string(),
        ));
    }
    if matches("controllable") || matches("midnightcontrols") || matches("controlify") {
        return Some((
            "Gamepad & DualSense controller support".to_string(),
            "Horizon natively detects gamepads with analog trigger curves and haptic rumble.".to_string(),
        ));
    }
    if matches("appleskin") {
        return Some((
            "Food saturation & exhaustion HUD".to_string(),
            "Horizon displays saturation and food exhaustion values directly in the native HUD.".to_string(),
        ));
    }
    if matches("clothconfig") {
        return Some((
            "In-launcher configuration UI".to_string(),
            "Horizon exposes all graphic, audio, and controller settings natively in Gradle.".to_string(),
        ));
    }
    if matches("yungsapi") || matches("yungsextras") {
        return Some((
            "Jigsaw structure template engine".to_string(),
            "Horizon supports standard jigsaw structure templates and pool generation natively."
                .to_string(),
        ));
    }
    if matches("chunky") {
        return Some((
            "Native pre-generation in Horizon engine".to_string(),
            "Horizon provides command-line background chunk generation in Rust.".to_string(),
        ));
    }
    if matches("reesessodiumoptions") {
        return Some((
            "Horizon native video settings menu".to_string(),
            "Horizon's video settings UI provides granular control over all rendering parameters."
                .to_string(),
        ));
    }

    None
}

// ---------------------------------------------------------------------------
// Manifest parsing helpers
// ---------------------------------------------------------------------------

fn read_entry_string<R: Read + Seek>(archive: &mut ZipArchive<R>, name: &str) -> Option<String> {
    let mut entry = archive.by_name(name).ok()?;
    let mut string = String::new();
    entry.read_to_string(&mut string).ok()?;
    Some(string)
}

#[allow(clippy::too_many_arguments)]
fn parse_fabric_json(
    json: &serde_json::Value,
    mod_id: &mut String,
    mod_name: &mut String,
    mod_version: &mut String,
    minecraft_version: &mut Option<String>,
    entrypoints: &mut Vec<String>,
    mixins: &mut Vec<String>,
    dependencies: &mut HashMap<String, String>,
) {
    if let Some(id) = json.get("id").and_then(|v| v.as_str()) {
        *mod_id = id.to_string();
    }
    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
        *mod_name = name.to_string();
    }
    if let Some(version) = json.get("version").and_then(json_to_string) {
        *mod_version = version;
    }

    // Depends
    if let Some(depends) = json.get("depends").and_then(|v| v.as_object()) {
        for (dep_id, dep_val) in depends {
            let ver = json_to_string(dep_val).unwrap_or_else(|| "*".to_string());
            if dep_id == "minecraft" {
                *minecraft_version = Some(ver.clone());
            }
            dependencies.insert(dep_id.clone(), ver);
        }
    }

    // Mixins
    if let Some(mixins_arr) = json.get("mixins").and_then(|v| v.as_array()) {
        for m in mixins_arr {
            if let Some(m_str) = m.as_str() {
                mixins.push(m_str.to_string());
            } else if let Some(config) = m.get("config").and_then(|v| v.as_str()) {
                mixins.push(config.to_string());
            }
        }
    }

    // Entrypoints
    if let Some(ep) = json.get("entrypoints")
        && let Some(ep_obj) = ep.as_object() {
            for (key, val) in ep_obj {
                if let Some(arr) = val.as_array() {
                    for item in arr {
                        if let Some(s) = item.as_str() {
                            entrypoints.push(format!("{key}: {s}"));
                        } else if let Some(val_s) = item.get("value").and_then(|v| v.as_str()) {
                            entrypoints.push(format!("{key}: {val_s}"));
                        }
                    }
                } else if let Some(s) = val.as_str() {
                    entrypoints.push(format!("{key}: {s}"));
                }
            }
        }
}

#[allow(clippy::too_many_arguments)]
fn parse_quilt_json(
    json: &serde_json::Value,
    mod_id: &mut String,
    mod_name: &mut String,
    mod_version: &mut String,
    minecraft_version: &mut Option<String>,
    entrypoints: &mut Vec<String>,
    mixins: &mut Vec<String>,
    dependencies: &mut HashMap<String, String>,
) {
    let loader = json.get("quilt_loader").unwrap_or(json);

    if let Some(id) = loader.get("id").and_then(|v| v.as_str()) {
        *mod_id = id.to_string();
    }
    if let Some(meta) = loader.get("metadata")
        && let Some(name) = meta.get("name").and_then(|v| v.as_str()) {
            *mod_name = name.to_string();
        }
    if let Some(version) = loader.get("version").and_then(json_to_string) {
        *mod_version = version;
    }

    // Depends
    if let Some(depends) = loader.get("depends").and_then(|v| v.as_array()) {
        for item in depends {
            if let Some(dep_obj) = item.as_object() {
                if let Some(id) = dep_obj.get("id").and_then(|v| v.as_str()) {
                    let versions = dep_obj
                        .get("versions")
                        .and_then(json_to_string)
                        .unwrap_or_else(|| "*".to_string());
                    if id == "minecraft" {
                        *minecraft_version = Some(versions.clone());
                    }
                    dependencies.insert(id.to_string(), versions);
                }
            } else if let Some(id) = item.as_str() {
                dependencies.insert(id.to_string(), "*".to_string());
            }
        }
    }

    // Mixins
    for key in ["mixin", "mixins"] {
        if let Some(arr) = json.get(key).and_then(|v| v.as_array()) {
            for m in arr {
                if let Some(s) = m.as_str() {
                    mixins.push(s.to_string());
                } else if let Some(config) = m.get("config").and_then(|v| v.as_str()) {
                    mixins.push(config.to_string());
                }
            }
        }
    }

    // Entrypoints
    if let Some(ep) = loader.get("entrypoints").and_then(|v| v.as_object()) {
        for (key, val) in ep {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    if let Some(s) = item.as_str() {
                        entrypoints.push(format!("{key}: {s}"));
                    }
                }
            } else if let Some(s) = val.as_str() {
                entrypoints.push(format!("{key}: {s}"));
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn parse_forge_toml(
    toml_val: &toml::Value,
    mod_id: &mut String,
    mod_name: &mut String,
    mod_version: &mut String,
    minecraft_version: &mut Option<String>,
    _entrypoints: &mut Vec<String>,
    mixins: &mut Vec<String>,
    dependencies: &mut HashMap<String, String>,
) {
    if let Some(mods_arr) = toml_val.get("mods").and_then(|v| v.as_array())
        && let Some(first_mod) = mods_arr.first().and_then(|v| v.as_table()) {
            if let Some(id) = first_mod.get("modId").and_then(|v| v.as_str()) {
                *mod_id = id.to_string();
            }
            if let Some(name) = first_mod.get("displayName").and_then(|v| v.as_str()) {
                *mod_name = name.to_string();
            }
            if let Some(ver) = first_mod.get("version").and_then(|v| v.as_str()) {
                *mod_version = ver.to_string();
            }
        }

    // Dependencies
    if let Some(deps_table) = toml_val.get("dependencies").and_then(|v| v.as_table()) {
        for (_owner_mod, dep_list) in deps_table {
            if let Some(arr) = dep_list.as_array() {
                for dep in arr {
                    if let Some(dep_tbl) = dep.as_table()
                        && let Some(dep_id) = dep_tbl.get("modId").and_then(|v| v.as_str()) {
                            let range = dep_tbl
                                .get("versionRange")
                                .and_then(|v| v.as_str())
                                .unwrap_or("*")
                                .to_string();
                            if dep_id == "minecraft" {
                                *minecraft_version = Some(range.clone());
                            }
                            dependencies.insert(dep_id.to_string(), range);
                        }
                }
            }
        }
    }

    // Mixins
    if let Some(mixins_arr) = toml_val.get("mixins").and_then(|v| v.as_array()) {
        for m in mixins_arr {
            if let Some(tbl) = m.as_table() {
                if let Some(cfg) = tbl.get("config").and_then(|v| v.as_str()) {
                    mixins.push(cfg.to_string());
                }
            } else if let Some(s) = m.as_str() {
                mixins.push(s.to_string());
            }
        }
    }
}

fn parse_mcmod_info(
    json: &serde_json::Value,
    mod_id: &mut String,
    mod_name: &mut String,
    mod_version: &mut String,
    minecraft_version: &mut Option<String>,
    dependencies: &mut HashMap<String, String>,
) {
    let list = if let Some(arr) = json.as_array() {
        arr
    } else if let Some(arr) = json.get("modList").and_then(|v| v.as_array()) {
        arr
    } else {
        return;
    };

    if let Some(first) = list.first() {
        if let Some(id) = first.get("modid").and_then(|v| v.as_str()) {
            *mod_id = id.to_string();
        }
        if let Some(name) = first.get("name").and_then(|v| v.as_str()) {
            *mod_name = name.to_string();
        }
        if let Some(version) = first.get("version").and_then(json_to_string) {
            *mod_version = version;
        }
        if let Some(mcversion) = first.get("mcversion").and_then(json_to_string) {
            *minecraft_version = Some(mcversion);
        }
        if let Some(req) = first.get("requiredMods").and_then(|v| v.as_array()) {
            for dep in req {
                if let Some(d) = dep.as_str() {
                    dependencies.insert(d.to_string(), "*".to_string());
                }
            }
        }
        if let Some(dep_arr) = first.get("dependencies").and_then(|v| v.as_array()) {
            for dep in dep_arr {
                if let Some(d) = dep.as_str() {
                    dependencies.insert(d.to_string(), "*".to_string());
                }
            }
        }
    }
}

fn json_to_string(val: &serde_json::Value) -> Option<String> {
    match val {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Array(arr) => {
            let items: Vec<String> = arr.iter().filter_map(json_to_string).collect();
            Some(items.join(", "))
        }
        _ => None,
    }
}

fn infer_mod_id_from_filename(filename: &str) -> String {
    let base = filename
        .trim_end_matches(".jar")
        .trim_end_matches(".wasm")
        .trim_end_matches(".hmod")
        .trim_end_matches(".zip");

    // Remove trailing version like -1.2.3 or _v1.0
    if let Some(idx) = base.find(['-', '+']) {
        base[..idx].to_string()
    } else {
        base.to_string()
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_fabric_mod_classification_and_mixins() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("fabric.mod.json", opts).unwrap();
            let fabric_json = r#"{
                "schemaVersion": 1,
                "id": "test_fabric_mod",
                "name": "Test Fabric Mod",
                "version": "1.2.3",
                "entrypoints": {
                    "client": ["net.example.client.ClientInit"],
                    "main": ["net.example.CommonInit"]
                },
                "mixins": ["test_fabric.mixins.json"],
                "depends": {
                    "minecraft": ">=1.20.4",
                    "fabricloader": ">=0.15.0"
                }
            }"#;
            writer.write_all(fabric_json.as_bytes()).unwrap();

            // Add dummy class bytecode
            writer.start_file("net/example/CommonInit.class", opts).unwrap();
            writer.write_all(b"\xCA\xFE\xBA\xBE").unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("test_fabric_mod-1.2.3.jar", archive, Some(1024)).unwrap();

        assert_eq!(analysis.mod_id, "test_fabric_mod");
        assert_eq!(analysis.name, "Test Fabric Mod");
        assert_eq!(analysis.version, "1.2.3");
        assert_eq!(analysis.loader, DetectedLoader::Fabric);
        assert_eq!(analysis.minecraft_version.as_deref(), Some(">=1.20.4"));
        assert_eq!(analysis.class_count, 1);
        assert!(!analysis.is_resource_only);
        assert_eq!(analysis.mixins, vec!["test_fabric.mixins.json"]);
        assert!(analysis.entrypoints.iter().any(|e| e.contains("ClientInit")));

        // Should be classified as UNSUPPORTED because it contains bytecode classes
        match &analysis.classification {
            ModClassification::Unsupported { reason, class_count, mixin_count } => {
                assert_eq!(*class_count, 1);
                assert_eq!(*mixin_count, 1);
                assert!(reason.contains("Requires Java Virtual Machine"));
            }
            _ => panic!("Expected Unsupported classification"),
        }
    }

    #[test]
    fn test_resource_only_jar_detection() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("pack.mcmeta", opts).unwrap();
            writer.write_all(b"{\"pack\":{\"description\":\"Fresh Animations\",\"pack_format\":15}}").unwrap();

            writer.start_file("assets/minecraft/optifine/cem/pig.jem", opts).unwrap();
            writer.write_all(b"models").unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("FreshAnimations_v1.9.jar", archive, Some(2048)).unwrap();

        assert_eq!(analysis.class_count, 0);
        assert!(analysis.is_resource_only);
        match &analysis.classification {
            ModClassification::ResourceOnly { reason, assets_found } => {
                assert!(reason.contains("Asset & Data mod"));
                assert!(assets_found.contains(&"pack.mcmeta".to_string()));
            }
            _ => panic!("Expected ResourceOnly classification"),
        }
        assert!(analysis.classification.is_supported());
    }

    #[test]
    fn test_builtin_equivalent_detection() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("fabric.mod.json", opts).unwrap();
            let fabric_json = r#"{
                "schemaVersion": 1,
                "id": "sodium",
                "name": "Sodium",
                "version": "0.5.8",
                "mixins": ["sodium.mixins.json"]
            }"#;
            writer.write_all(fabric_json.as_bytes()).unwrap();

            // Has bytecode
            writer.start_file("me/jellysquid/mods/sodium/Sodium.class", opts).unwrap();
            writer.write_all(b"\xCA\xFE\xBA\xBE").unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("sodium-fabric-0.5.8.jar", archive, Some(4096)).unwrap();

        assert_eq!(analysis.mod_id, "sodium");
        match &analysis.classification {
            ModClassification::BuiltInEquivalent { feature, reason } => {
                assert!(feature.contains("chunk mesher"));
                assert!(reason.contains("Horizon includes a native Rust multi-threaded chunk mesher"));
            }
            _ => panic!("Expected BuiltInEquivalent classification"),
        }
        assert!(analysis.classification.is_supported());
    }

    #[test]
    fn test_quilt_mod_classification() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("quilt.mod.json", opts).unwrap();
            let quilt_json = r#"{
                "schema_version": 1,
                "quilt_loader": {
                    "group": "org.quiltmc",
                    "id": "quilt_sample",
                    "version": "2.0.0",
                    "metadata": {
                        "name": "Quilt Sample Mod"
                    },
                    "entrypoints": {
                        "init": ["org.quiltmc.sample.SampleMod"]
                    },
                    "depends": [
                        { "id": "minecraft", "versions": ">=1.20.2" }
                    ]
                },
                "mixin": ["sample.mixins.json"]
            }"#;
            writer.write_all(quilt_json.as_bytes()).unwrap();

            writer.start_file("org/quiltmc/sample/SampleMod.class", opts).unwrap();
            writer.write_all(b"\xCA\xFE\xBA\xBE").unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("quilt_sample-2.0.0.jar", archive, Some(1024)).unwrap();

        assert_eq!(analysis.mod_id, "quilt_sample");
        assert_eq!(analysis.name, "Quilt Sample Mod");
        assert_eq!(analysis.loader, DetectedLoader::Quilt);
        assert_eq!(analysis.minecraft_version.as_deref(), Some(">=1.20.2"));
        assert_eq!(analysis.class_count, 1);
        assert_eq!(analysis.mixins, vec!["sample.mixins.json"]);
        assert!(analysis.entrypoints.iter().any(|e| e.contains("SampleMod")));
    }

    #[test]
    fn test_forge_mod_classification() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("META-INF/mods.toml", opts).unwrap();
            let forge_toml = r#"
modLoader="javafml"
loaderVersion="[47,)"

[[mods]]
modId="create"
displayName="Create"
version="0.5.1"

[[dependencies.create]]
modId="minecraft"
versionRange="[1.20.1,1.21)"

[[mixins]]
config="create.mixins.json"
"#;
            writer.write_all(forge_toml.as_bytes()).unwrap();

            writer.start_file("com/simibubi/create/Create.class", opts).unwrap();
            writer.write_all(b"\xCA\xFE\xBA\xBE").unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("create-1.20.1-0.5.1.jar", archive, Some(4096)).unwrap();

        assert_eq!(analysis.mod_id, "create");
        assert_eq!(analysis.name, "Create");
        assert_eq!(analysis.version, "0.5.1");
        assert_eq!(analysis.loader, DetectedLoader::Forge);
        assert_eq!(analysis.minecraft_version.as_deref(), Some("[1.20.1,1.21)"));
        assert_eq!(analysis.mixins, vec!["create.mixins.json"]);
    }

    #[test]
    fn test_horizon_mod_archive_classification() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("horizon-mod.toml", opts).unwrap();
            let manifest = r#"
id = "native_minimap"
name = "Native Minimap"
version = "1.0.0"
api = "0.1"
"#;
            writer.write_all(manifest.as_bytes()).unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("minimap.hmod", archive, Some(1024)).unwrap();

        assert_eq!(analysis.mod_id, "native_minimap");
        assert_eq!(analysis.name, "Native Minimap");
        assert_eq!(analysis.loader, DetectedLoader::HorizonWasm);
        assert!(analysis.classification.is_supported());
    }

    #[test]
    fn test_wasm_mod_classification() {
        let dir = tempdir().unwrap();
        let wasm_path = dir.path().join("horizon_minimap.wasm");
        fs::write(&wasm_path, b"\x00asm\x01\x00\x00\x00").unwrap();

        let analysis = analyze_jar_file(&wasm_path).unwrap();

        assert_eq!(analysis.loader, DetectedLoader::HorizonWasm);
        match &analysis.classification {
            ModClassification::WasmAvailable { wasm_id, .. } => {
                assert_eq!(wasm_id, "horizon_minimap");
            }
            _ => panic!("Expected WasmAvailable classification"),
        }
        assert!(analysis.classification.is_supported());
    }

    #[test]
    fn test_compatibility_report_math_and_summary() {
        // Build mock list: 4 supported (2 built-in, 1 resource-only, 1 wasm), 45 unsupported
        let mut mods = Vec::new();

        // 1 Wasm
        let wasm_cls = ModClassification::WasmAvailable {
            wasm_id: "minimap".to_string(),
            reason: "Native".to_string(),
        };
        mods.push(ModJarAnalysis {
            file_name: "minimap.wasm".to_string(),
            mod_id: "minimap".to_string(),
            name: "Minimap".to_string(),
            version: "1.0.0".to_string(),
            loader: DetectedLoader::HorizonWasm,
            minecraft_version: None,
            entrypoints: Vec::new(),
            mixins: Vec::new(),
            dependencies: HashMap::new(),
            class_count: 0,
            is_resource_only: false,
            bucket: wasm_cls.bucket(),
            classification: wasm_cls,
            catalog_entry: None,
        });

        // 1 Resource-only
        let res_cls = ModClassification::ResourceOnly {
            reason: "Pure resources".to_string(),
            assets_found: vec!["assets/".to_string()],
        };
        mods.push(ModJarAnalysis {
            file_name: "fresh_animations.jar".to_string(),
            mod_id: "fresh_animations".to_string(),
            name: "Fresh Animations".to_string(),
            version: "1.0.0".to_string(),
            loader: DetectedLoader::Unknown,
            minecraft_version: None,
            entrypoints: Vec::new(),
            mixins: Vec::new(),
            dependencies: HashMap::new(),
            class_count: 0,
            is_resource_only: true,
            bucket: res_cls.bucket(),
            classification: res_cls,
            catalog_entry: None,
        });

        // 2 Built-in equivalents (Sodium, Lithium)
        let sod_cls = ModClassification::BuiltInEquivalent {
            feature: "Chunk mesher".to_string(),
            reason: "Native Rust".to_string(),
        };
        mods.push(ModJarAnalysis {
            file_name: "sodium.jar".to_string(),
            mod_id: "sodium".to_string(),
            name: "Sodium".to_string(),
            version: "0.5.8".to_string(),
            loader: DetectedLoader::Fabric,
            minecraft_version: None,
            entrypoints: Vec::new(),
            mixins: Vec::new(),
            dependencies: HashMap::new(),
            class_count: 120,
            is_resource_only: false,
            bucket: sod_cls.bucket(),
            classification: sod_cls,
            catalog_entry: None,
        });
        let lith_cls = ModClassification::BuiltInEquivalent {
            feature: "Physics ECS".to_string(),
            reason: "Native Rust".to_string(),
        };
        mods.push(ModJarAnalysis {
            file_name: "lithium.jar".to_string(),
            mod_id: "lithium".to_string(),
            name: "Lithium".to_string(),
            version: "0.12.0".to_string(),
            loader: DetectedLoader::Fabric,
            minecraft_version: None,
            entrypoints: Vec::new(),
            mixins: Vec::new(),
            dependencies: HashMap::new(),
            class_count: 80,
            is_resource_only: false,
            bucket: lith_cls.bucket(),
            classification: lith_cls,
            catalog_entry: None,
        });

        // 45 Unsupported mods
        for i in 1..=45 {
            let unsupp_cls = ModClassification::Unsupported {
                reason: "Requires JVM".to_string(),
                class_count: 50,
                mixin_count: 1,
            };
            mods.push(ModJarAnalysis {
                file_name: format!("mod_{i}.jar"),
                mod_id: format!("mod_{i}"),
                name: format!("Java Mod {i}"),
                version: "1.0.0".to_string(),
                loader: DetectedLoader::Fabric,
                minecraft_version: Some("1.20.4".to_string()),
                entrypoints: vec!["client: ClientMod".to_string()],
                mixins: vec!["mod.mixins.json".to_string()],
                dependencies: HashMap::new(),
                class_count: 50,
                is_resource_only: false,
                bucket: unsupp_cls.bucket(),
                classification: unsupp_cls,
                catalog_entry: None,
            });
        }

        let report = generate_compatibility_report(mods);
        assert_eq!(report.total_mods, 49);
        assert_eq!(report.supported_count, 4);
        assert_eq!(report.skipped_count, 45);
        assert_eq!(report.summary, "4 of 49 mods supported, 45 skipped");
        assert!(!report.can_launch_native);

        // Supported mods should be sorted first
        assert!(report.mods[0].classification.is_supported());
        assert!(report.mods[1].classification.is_supported());
        assert!(report.mods[2].classification.is_supported());
        assert!(report.mods[3].classification.is_supported());
        assert!(!report.mods[4].classification.is_supported());
    }

    #[test]
    fn test_classify_instance_mods_dir_on_disk() {
        let dir = tempdir().unwrap();
        let mods_dir = dir.path().join("mods");
        fs::create_dir_all(&mods_dir).unwrap();

        // 1. Create a sodium jar (BuiltInEquivalent)
        {
            let jar_path = mods_dir.join("sodium-0.5.8.jar");
            let file = File::create(&jar_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let opts = SimpleFileOptions::default();
            writer.start_file("fabric.mod.json", opts).unwrap();
            writer.write_all(b"{\"id\":\"sodium\",\"name\":\"Sodium\",\"version\":\"0.5.8\"}").unwrap();
            writer.start_file("Sodium.class", opts).unwrap();
            writer.write_all(b"\xCA\xFE\xBA\xBE").unwrap();
            writer.finish().unwrap();
        }

        // 2. Create a disabled mod (should be ignored)
        {
            let jar_path = mods_dir.join("disabled_mod.jar.disabled");
            fs::write(jar_path, b"not a mod").unwrap();
        }

        let report = classify_instance_mods_dir(&mods_dir).unwrap();
        assert_eq!(report.total_mods, 1);
        assert_eq!(report.supported_count, 1);
        assert_eq!(report.skipped_count, 0);
        assert_eq!(report.summary, "1 of 1 mods supported, 0 skipped");
        assert!(report.can_launch_native);
    }

    #[test]
    fn test_mcmod_info_legacy_forge_parsing() {
        let mut buffer = Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let opts = SimpleFileOptions::default();

            writer.start_file("mcmod.info", opts).unwrap();
            let mcmod_json = r#"[
                {
                    "modid": "ironchest",
                    "name": "Iron Chest",
                    "version": "1.12.2-7.0.72.847",
                    "mcversion": "1.12.2",
                    "requiredMods": ["forge@[14.23.5.2847,)"]
                }
            ]"#;
            writer.write_all(mcmod_json.as_bytes()).unwrap();

            // Bytecode class
            writer.start_file("cpw/mods/ironchest/IronChest.class", opts).unwrap();
            writer.write_all(b"\xCA\xFE\xBA\xBE").unwrap();

            writer.finish().unwrap();
        }

        buffer.set_position(0);
        let archive = ZipArchive::new(buffer).unwrap();
        let analysis = analyze_jar_reader("ironchest-1.12.2.jar", archive, Some(4096)).unwrap();

        assert_eq!(analysis.mod_id, "ironchest");
        assert_eq!(analysis.name, "Iron Chest");
        assert_eq!(analysis.version, "1.12.2-7.0.72.847");
        assert_eq!(analysis.loader, DetectedLoader::Forge);
        assert_eq!(analysis.minecraft_version.as_deref(), Some("1.12.2"));
        assert!(analysis.dependencies.contains_key("forge@[14.23.5.2847,)"));
        assert_eq!(analysis.bucket, CompatibilityBucket::Unsupported);
    }

    #[test]
    fn test_four_bucket_classification() {
        // 1. RESOURCE_ONLY
        let mut b1 = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut b1);
            let opts = SimpleFileOptions::default();
            w.start_file("assets/example/textures/block.png", opts).unwrap();
            w.write_all(b"\x89PNG\r\n\x1a\n").unwrap();
            w.finish().unwrap();
        }
        b1.set_position(0);
        let a1 = analyze_jar_reader("textures.jar", ZipArchive::new(b1).unwrap(), Some(10)).unwrap();
        assert_eq!(a1.bucket, CompatibilityBucket::ResourceOnly);

        // 2. CODE_WITH_RECOVERABLE_ASSETS
        let mut b2 = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut b2);
            let opts = SimpleFileOptions::default();
            w.start_file("assets/example/textures/block.png", opts).unwrap();
            w.write_all(b"\x89PNG\r\n\x1a\n").unwrap();
            w.start_file("net/example/Code.class", opts).unwrap();
            w.write_all(b"\xca\xfe\xba\xbe").unwrap();
            w.finish().unwrap();
        }
        b2.set_position(0);
        let a2 = analyze_jar_reader("hybrid.jar", ZipArchive::new(b2).unwrap(), Some(20)).unwrap();
        assert_eq!(a2.bucket, CompatibilityBucket::CodeWithRecoverableAssets);

        // 3. NATIVE_PORT_AVAILABLE (Sodium has built-in equivalent, maps to native port available)
        let mut b3 = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut b3);
            let opts = SimpleFileOptions::default();
            w.start_file("fabric.mod.json", opts).unwrap();
            w.write_all(b"{\"id\":\"sodium\",\"name\":\"Sodium\"}").unwrap();
            w.finish().unwrap();
        }
        b3.set_position(0);
        let a3 = analyze_jar_reader("sodium.jar", ZipArchive::new(b3).unwrap(), Some(10)).unwrap();
        assert_eq!(a3.bucket, CompatibilityBucket::NativePortAvailable);

        // 4. UNSUPPORTED (Pure logic mod with 0 assets and no native port)
        let mut b4 = Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut b4);
            let opts = SimpleFileOptions::default();
            w.start_file("net/example/Logic.class", opts).unwrap();
            w.write_all(b"\xca\xfe\xba\xbe").unwrap();
            w.finish().unwrap();
        }
        b4.set_position(0);
        let a4 = analyze_jar_reader("pure_logic.jar", ZipArchive::new(b4).unwrap(), Some(10)).unwrap();
        assert_eq!(a4.bucket, CompatibilityBucket::Unsupported);
    }

    #[test]
    fn test_modpack_breakdown_summary() {
        let mut mods = Vec::new();

        // 6 Resource-only
        for i in 1..=6 {
            let res = ModClassification::ResourceOnly {
                reason: "Assets only".to_string(),
                assets_found: vec!["assets/".to_string()],
            };
            mods.push(ModJarAnalysis {
                file_name: format!("res_{i}.jar"),
                mod_id: format!("res_{i}"),
                name: format!("Resource Pack {i}"),
                version: "1.0".to_string(),
                loader: DetectedLoader::Unknown,
                minecraft_version: None,
                entrypoints: Vec::new(),
                mixins: Vec::new(),
                dependencies: HashMap::new(),
                class_count: 0,
                is_resource_only: true,
                bucket: res.bucket(),
                classification: res,
                catalog_entry: None,
            });
        }

        // 3 Natively ported
        for i in 1..=3 {
            let port = ModClassification::NativePortAvailable {
                wasm_id: format!("port_{i}"),
                reason: "WASM available".to_string(),
                port_url: Some(format!("https://example.com/port_{i}.wasm")),
                version: "1.0".to_string(),
            };
            mods.push(ModJarAnalysis {
                file_name: format!("port_{i}.wasm"),
                mod_id: format!("port_{i}"),
                name: format!("Ported Mod {i}"),
                version: "1.0".to_string(),
                loader: DetectedLoader::HorizonWasm,
                minecraft_version: Some("26.3".to_string()),
                entrypoints: vec!["main".to_string()],
                mixins: Vec::new(),
                dependencies: HashMap::new(),
                class_count: 0,
                is_resource_only: false,
                bucket: port.bucket(),
                classification: port,
                catalog_entry: None,
            });
        }

        // 40 Unsupported
        for i in 1..=40 {
            let unsupp = ModClassification::Unsupported {
                reason: "Pure Java logic".to_string(),
                class_count: 25,
                mixin_count: 1,
            };
            mods.push(ModJarAnalysis {
                file_name: format!("logic_{i}.jar"),
                mod_id: format!("logic_{i}"),
                name: format!("Logic Mod {i}"),
                version: "1.0".to_string(),
                loader: DetectedLoader::Fabric,
                minecraft_version: Some("1.20.4".to_string()),
                entrypoints: vec!["init".to_string()],
                mixins: vec!["mixins.json".to_string()],
                dependencies: HashMap::new(),
                class_count: 25,
                is_resource_only: false,
                bucket: unsupp.bucket(),
                classification: unsupp,
                catalog_entry: None,
            });
        }

        let report = generate_modpack_compatibility_report("Fabulously Optimized", mods);
        assert_eq!(report.total_mods, 49);
        assert_eq!(report.resource_only_count, 6);
        assert_eq!(report.native_port_count, 3);
        assert_eq!(report.unsupported_count, 40);
        assert_eq!(
            report.breakdown_summary,
            "Fabulously Optimized: 6 resource-only, 3 natively ported, 40 unsupported (listed)"
        );
    }

    #[test]
    fn test_instant_repeat_scan_caching() {
        let temp = tempdir().unwrap();
        let jar_path = temp.path().join("cached_test.jar");
        {
            let file = File::create(&jar_path).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            let opts = SimpleFileOptions::default();
            writer.start_file("fabric.mod.json", opts).unwrap();
            writer.write_all(b"{\"id\":\"test_cache\",\"name\":\"Test Cache\"}").unwrap();
            writer.finish().unwrap();
        }

        clear_compatibility_scan_cache();

        // First scan (uncached)
        let t0 = std::time::Instant::now();
        let a1 = analyze_jar_file(&jar_path).unwrap();
        let d1 = t0.elapsed();

        assert_eq!(a1.mod_id, "test_cache");

        // Second scan (cached hit)
        let t1 = std::time::Instant::now();
        let a2 = analyze_jar_file(&jar_path).unwrap();
        let d2 = t1.elapsed();

        assert_eq!(a2.mod_id, "test_cache");
        // Cached read is much faster
        assert!(d2 <= d1 || d2.as_micros() < 500);
    }
}
