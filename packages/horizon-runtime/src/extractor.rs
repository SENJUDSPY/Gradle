//! Multi-loader mod JAR content extractor, schema validator, and asset merger for Horizon
//! (Minecraft 26.3 rewritten in native Rust).
//!
//! Extracts non-code content (`assets/` and `data/`) from Fabric, Quilt, Forge, and NeoForge
//! mod JARs, completely ignoring JVM `.class` bytecode and signatures.
//! Validates extracted models, blockstates, and recipes against Horizon schemas,
//! and applies vanilla resource pack override rules with configurable load ordering.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Category of extracted asset/datapack item
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetCategory {
    Texture,
    Model,
    Blockstate,
    Sound,
    Lang,
    Particle,
    Font,
    Shader,
    Recipe,
    LootTable,
    Advancement,
    Tag,
    Structure,
    Worldgen,
    OtherAsset,
    OtherData,
}

impl AssetCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Texture => "texture",
            Self::Model => "model",
            Self::Blockstate => "blockstate",
            Self::Sound => "sound",
            Self::Lang => "language file",
            Self::Particle => "particle",
            Self::Font => "font",
            Self::Shader => "shader",
            Self::Recipe => "recipe",
            Self::LootTable => "loot table",
            Self::Advancement => "advancement",
            Self::Tag => "tag",
            Self::Structure => "structure",
            Self::Worldgen => "worldgen",
            Self::OtherAsset => "asset",
            Self::OtherData => "data",
        }
    }
}

/// Validation issue detected during schema checking
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetValidationIssue {
    pub path: String,
    pub category: AssetCategory,
    pub reason: String,
}

/// Summary of an extraction operation on a mod JAR
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionSummary {
    pub mod_id: String,
    pub mod_name: String,
    pub jar_file: String,
    pub total_entries: usize,
    pub skipped_classes_count: usize,
    pub textures_recovered: usize,
    pub total_textures_in_jar: usize,
    pub models_recovered: usize,
    pub total_models_in_jar: usize,
    pub blockstates_recovered: usize,
    pub total_blockstates_in_jar: usize,
    pub sounds_recovered: usize,
    pub total_sounds_in_jar: usize,
    pub lang_files_recovered: usize,
    pub recipes_recovered: usize,
    pub total_recipes_in_jar: usize,
    pub loot_tables_recovered: usize,
    pub total_loot_tables_in_jar: usize,
    pub tags_recovered: usize,
    pub other_assets_recovered: usize,
    pub other_data_recovered: usize,
    pub total_recovered: usize,
    pub total_skipped: usize,
    pub validation_warnings: Vec<AssetValidationIssue>,
    pub formatted_report: String,
    pub extracted_at: String,
}

/// Entry in the instance's extracted mod load order
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoadOrderItem {
    pub mod_id: String,
    pub mod_name: String,
    pub priority: usize,
    pub enabled: bool,
    pub relative_dir: String,
}

/// Record of an asset overridden by a higher-priority mod
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AssetOverrideRecord {
    pub relative_path: String,
    pub winning_mod_id: String,
    pub overridden_mod_id: String,
}

/// State of the merged extracted assets for an instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergedAssetsState {
    pub total_unique_assets: usize,
    pub total_unique_data: usize,
    pub active_mods: Vec<String>,
    pub override_conflicts: Vec<AssetOverrideRecord>,
    pub last_merged_at: String,
}

/// Manifest saved in each mod's extracted directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedModManifest {
    pub schema_version: u32,
    pub summary: ExtractionSummary,
}

/// Validates raw content against Horizon/Minecraft schema expectations
pub fn validate_asset_content(
    relative_path: &str,
    category: AssetCategory,
    bytes: &[u8],
) -> Result<(), String> {
    if bytes.is_empty() {
        return Err("File is empty (0 bytes)".to_string());
    }

    match category {
        AssetCategory::Texture => {
            // Validate PNG header (89 50 4E 47 0D 0A 1A 0A)
            if bytes.len() < 8 {
                return Err("File is too small to be a valid PNG image".to_string());
            }
            const PNG_HEADER: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
            if bytes[0..8] != PNG_HEADER {
                return Err("Invalid PNG magic header; file may be corrupt or misnamed".to_string());
            }
            Ok(())
        }
        AssetCategory::Model => {
            // Must be valid JSON object with optional parent, textures, or elements
            let Ok(text) = std::str::from_utf8(bytes) else {
                return Err("Model JSON is not valid UTF-8".to_string());
            };
            let json: serde_json::Value = serde_json::from_str(text)
                .map_err(|e| format!("Invalid model JSON syntax: {e}"))?;

            let Some(obj) = json.as_object() else {
                return Err("Model root must be a JSON object".to_string());
            };

            let has_parent = obj.get("parent").and_then(|v| v.as_str()).is_some();
            let has_elements = obj.get("elements").and_then(|v| v.as_array()).is_some();
            let has_textures = obj.get("textures").and_then(|v| v.as_object()).is_some();

            if !has_parent && !has_elements && !has_textures {
                return Err("Model schema invalid: missing 'parent', 'elements', or 'textures' definition".to_string());
            }

            // If elements are present, validate each element has 'from' and 'to'
            if let Some(elements) = obj.get("elements").and_then(|v| v.as_array()) {
                for (idx, elem) in elements.iter().enumerate() {
                    let Some(e_obj) = elem.as_object() else {
                        return Err(format!("Model element #{idx} must be a JSON object"));
                    };
                    let from_valid = e_obj.get("from").and_then(|v| v.as_array()).is_some_and(|a| a.len() == 3);
                    let to_valid = e_obj.get("to").and_then(|v| v.as_array()).is_some_and(|a| a.len() == 3);
                    if !from_valid || !to_valid {
                        return Err(format!("Model element #{idx} must contain 3-coordinate 'from' and 'to' arrays"));
                    }
                }
            }

            Ok(())
        }
        AssetCategory::Blockstate => {
            let Ok(text) = std::str::from_utf8(bytes) else {
                return Err("Blockstate JSON is not valid UTF-8".to_string());
            };
            let json: serde_json::Value = serde_json::from_str(text)
                .map_err(|e| format!("Invalid blockstate JSON syntax: {e}"))?;

            let Some(obj) = json.as_object() else {
                return Err("Blockstate root must be a JSON object".to_string());
            };

            let has_variants = obj.get("variants").and_then(|v| v.as_object()).is_some();
            let has_multipart = obj.get("multipart").and_then(|v| v.as_array()).is_some();

            if !has_variants && !has_multipart {
                return Err("Blockstate schema invalid: must contain either 'variants' or 'multipart'".to_string());
            }

            Ok(())
        }
        AssetCategory::Recipe => {
            let Ok(text) = std::str::from_utf8(bytes) else {
                return Err("Recipe JSON is not valid UTF-8".to_string());
            };
            let json: serde_json::Value = serde_json::from_str(text)
                .map_err(|e| format!("Invalid recipe JSON syntax: {e}"))?;

            let Some(obj) = json.as_object() else {
                return Err("Recipe root must be a JSON object".to_string());
            };

            if obj.get("type").and_then(|v| v.as_str()).is_none() {
                return Err("Recipe schema invalid: missing required 'type' field".to_string());
            }

            Ok(())
        }
        AssetCategory::Sound | AssetCategory::Lang | AssetCategory::Tag | AssetCategory::LootTable | AssetCategory::Advancement | AssetCategory::Worldgen => {
            if relative_path.ends_with(".json") {
                let Ok(text) = std::str::from_utf8(bytes) else {
                    return Err("JSON content is not valid UTF-8".to_string());
                };
                serde_json::from_str::<serde_json::Value>(text)
                    .map_err(|e| format!("Invalid JSON syntax: {e}"))?;
            }
            Ok(())
        }
        AssetCategory::Structure => {
            // NBT gzip magic number: 0x1f 0x8b or uncompressed NBT 0x0a (compound tag)
            if bytes.len() < 2 {
                return Err("Structure file is too small".to_string());
            }
            let is_gzip = bytes[0] == 0x1f && bytes[1] == 0x8b;
            let is_raw_nbt = bytes[0] == 0x0a;
            if !is_gzip && !is_raw_nbt {
                return Err("Structure file does not appear to be valid gzip or raw NBT".to_string());
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// Identifies the category of an entry inside a JAR
pub fn categorize_entry_path(path: &str) -> Option<AssetCategory> {
    let lower = path.to_lowercase();

    if lower.starts_with("assets/") {
        if lower.contains("/textures/") && lower.ends_with(".png") {
            return Some(AssetCategory::Texture);
        }
        if lower.contains("/models/") && lower.ends_with(".json") {
            return Some(AssetCategory::Model);
        }
        if lower.contains("/blockstates/") && lower.ends_with(".json") {
            return Some(AssetCategory::Blockstate);
        }
        if lower.ends_with("sounds.json") || (lower.contains("/sounds/") && (lower.ends_with(".ogg") || lower.ends_with(".wav"))) {
            return Some(AssetCategory::Sound);
        }
        if lower.contains("/lang/") && lower.ends_with(".json") {
            return Some(AssetCategory::Lang);
        }
        if lower.contains("/particles/") && lower.ends_with(".json") {
            return Some(AssetCategory::Particle);
        }
        if lower.contains("/font/") {
            return Some(AssetCategory::Font);
        }
        if lower.contains("/shaders/") {
            return Some(AssetCategory::Shader);
        }
        return Some(AssetCategory::OtherAsset);
    }

    if lower.starts_with("data/") {
        if lower.contains("/recipes/") && lower.ends_with(".json") {
            return Some(AssetCategory::Recipe);
        }
        if lower.contains("/loot_tables/") && lower.ends_with(".json") {
            return Some(AssetCategory::LootTable);
        }
        if lower.contains("/advancements/") && lower.ends_with(".json") {
            return Some(AssetCategory::Advancement);
        }
        if lower.contains("/tags/") && lower.ends_with(".json") {
            return Some(AssetCategory::Tag);
        }
        if lower.contains("/structures/") && (lower.ends_with(".nbt") || lower.ends_with(".snbt")) {
            return Some(AssetCategory::Structure);
        }
        if lower.contains("/worldgen/") && lower.ends_with(".json") {
            return Some(AssetCategory::Worldgen);
        }
        return Some(AssetCategory::OtherData);
    }

    None
}

/// Extracts assets and data from a mod JAR reader into an instance target directory
pub fn extract_jar_content<R: Read + Seek>(
    mut archive: ZipArchive<R>,
    jar_file_name: &str,
    target_instance_dir: &Path,
) -> Result<ExtractionSummary> {
    let mut total_entries = 0usize;
    let mut skipped_classes_count = 0usize;

    let mut total_textures = 0usize;
    let mut total_models = 0usize;
    let mut total_blockstates = 0usize;
    let mut total_sounds = 0usize;
    let mut total_recipes = 0usize;
    let mut total_loot_tables = 0usize;

    let mut detected_mod_id = None;
    let mut detected_mod_name = None;

    // Scan for metadata manifests first
    for i in 0..archive.len() {
        if let Ok(mut entry) = archive.by_index(i) {
            let name = entry.name().to_string();
            match name.as_str() {
                "fabric.mod.json" => {
                    let mut s = String::new();
                    if entry.read_to_string(&mut s).is_ok()
                        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&s) {
                            if detected_mod_id.is_none() {
                                detected_mod_id = v.get("id").and_then(|x| x.as_str()).map(ToString::to_string);
                            }
                            if detected_mod_name.is_none() {
                                detected_mod_name = v.get("name").and_then(|x| x.as_str()).map(ToString::to_string);
                            }
                        }
                }
                "quilt.mod.json" => {
                    let mut s = String::new();
                    if entry.read_to_string(&mut s).is_ok()
                        && let Ok(v) = serde_json::from_str::<serde_json::Value>(&s)
                            && let Some(loader) = v.get("quilt_loader") {
                                if detected_mod_id.is_none() {
                                    detected_mod_id = loader.get("id").and_then(|x| x.as_str()).map(ToString::to_string);
                                }
                                if detected_mod_name.is_none() {
                                    detected_mod_name = loader.get("metadata").and_then(|m| m.get("name")).and_then(|x| x.as_str()).map(ToString::to_string);
                                }
                            }
                }
                "META-INF/mods.toml" | "META-INF/neoforge.mods.toml" => {
                    let mut s = String::new();
                    if entry.read_to_string(&mut s).is_ok()
                        && let Ok(v) = toml::from_str::<toml::Value>(&s)
                            && let Some(mods) = v.get("mods").and_then(|m| m.as_array()).and_then(|a| a.first()) {
                                if detected_mod_id.is_none() {
                                    detected_mod_id = mods.get("modId").and_then(|x| x.as_str()).map(ToString::to_string);
                                }
                                if detected_mod_name.is_none() {
                                    detected_mod_name = mods.get("displayName").and_then(|x| x.as_str()).map(ToString::to_string);
                                }
                            }
                }
                _ => {}
            }
        }
    }

    let raw_stem = Path::new(jar_file_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown_mod");

    let mod_id = detected_mod_id.unwrap_or_else(|| {
        raw_stem
            .chars()
            .map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' })
            .collect::<String>()
            .to_lowercase()
    });

    let mod_name = detected_mod_name.unwrap_or_else(|| raw_stem.replace(['-', '_'], " "));

    let extracted_base = target_instance_dir
        .join("horizon_extracted")
        .join(&mod_id);

    fs::create_dir_all(&extracted_base)
        .with_context(|| format!("creating extracted directory for mod {mod_id}"))?;

    let mut textures_recovered = 0usize;
    let mut models_recovered = 0usize;
    let mut blockstates_recovered = 0usize;
    let mut sounds_recovered = 0usize;
    let mut lang_files_recovered = 0usize;
    let mut recipes_recovered = 0usize;
    let mut loot_tables_recovered = 0usize;
    let mut tags_recovered = 0usize;
    let mut other_assets_recovered = 0usize;
    let mut other_data_recovered = 0usize;

    let mut validation_warnings = Vec::new();

    // Iterate over archive and extract valid assets/data
    for i in 0..archive.len() {
        total_entries += 1;
        let Ok(mut entry) = archive.by_index(i) else {
            continue;
        };

        let entry_name = entry.name().to_string();
        let lower_name = entry_name.to_lowercase();

        // 1. Explicitly ignore all .class files, JVM manifests, and signatures
        if lower_name.ends_with(".class") {
            skipped_classes_count += 1;
            continue;
        }
        if lower_name.starts_with("meta-inf/") && (lower_name.ends_with(".sf") || lower_name.ends_with(".dsa") || lower_name.ends_with(".rsa")) {
            continue;
        }

        // 2. Identify if entry is non-code asset or data
        let Some(category) = categorize_entry_path(&entry_name) else {
            continue;
        };

        if entry.is_dir() {
            continue;
        }

        // Count totals
        match category {
            AssetCategory::Texture => total_textures += 1,
            AssetCategory::Model => total_models += 1,
            AssetCategory::Blockstate => total_blockstates += 1,
            AssetCategory::Sound => total_sounds += 1,
            AssetCategory::Recipe => total_recipes += 1,
            AssetCategory::LootTable => total_loot_tables += 1,
            _ => {}
        }

        let mut content = Vec::new();
        if entry.read_to_end(&mut content).is_err() {
            validation_warnings.push(AssetValidationIssue {
                path: entry_name.clone(),
                category,
                reason: "Could not read entry bytes from archive".to_string(),
            });
            continue;
        }

        // 3. Schema validation
        if let Err(reason) = validate_asset_content(&entry_name, category, &content) {
            validation_warnings.push(AssetValidationIssue {
                path: entry_name.clone(),
                category,
                reason,
            });
            continue;
        }

        // 4. Write valid asset into extracted directory (with Zip-Slip path traversal protection)
        let rel_entry = Path::new(&entry_name);
        let has_traversal = rel_entry.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        });
        let out_path = extracted_base.join(rel_entry);
        if has_traversal || !out_path.starts_with(&extracted_base) {
            validation_warnings.push(AssetValidationIssue {
                path: entry_name.clone(),
                category,
                reason: "Path traversal / Zip-Slip detected in archive entry".to_string(),
            });
            continue;
        }

        if let Some(parent) = out_path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        if let Ok(mut out_file) = File::create(&out_path)
            && out_file.write_all(&content).is_ok() {
                match category {
                    AssetCategory::Texture => textures_recovered += 1,
                    AssetCategory::Model => models_recovered += 1,
                    AssetCategory::Blockstate => blockstates_recovered += 1,
                    AssetCategory::Sound => sounds_recovered += 1,
                    AssetCategory::Lang => lang_files_recovered += 1,
                    AssetCategory::Recipe => recipes_recovered += 1,
                    AssetCategory::LootTable => loot_tables_recovered += 1,
                    AssetCategory::Tag => tags_recovered += 1,
                    AssetCategory::OtherAsset | AssetCategory::Particle | AssetCategory::Font | AssetCategory::Shader => {
                        other_assets_recovered += 1;
                    }
                    AssetCategory::OtherData | AssetCategory::Advancement | AssetCategory::Structure | AssetCategory::Worldgen => {
                        other_data_recovered += 1;
                    }
                }
            }
    }

    let total_recovered = textures_recovered
        + models_recovered
        + blockstates_recovered
        + sounds_recovered
        + lang_files_recovered
        + recipes_recovered
        + loot_tables_recovered
        + tags_recovered
        + other_assets_recovered
        + other_data_recovered;

    let total_skipped = skipped_classes_count + validation_warnings.len();

    // Format human-readable report string matching user requirements
    // e.g. "12 of 40 textures, 8 of 15 recipes imported — code logic not supported"
    let mut parts = Vec::new();
    if total_textures > 0 {
        parts.push(format!("{textures_recovered} of {total_textures} textures"));
    }
    if total_models > 0 {
        parts.push(format!("{models_recovered} of {total_models} models"));
    }
    if total_recipes > 0 {
        parts.push(format!("{recipes_recovered} of {total_recipes} recipes"));
    }
    if total_sounds > 0 {
        parts.push(format!("{sounds_recovered} of {total_sounds} sounds"));
    }

    let formatted_report = if parts.is_empty() {
        if total_recovered > 0 {
            format!("{total_recovered} assets/data imported — code logic not supported")
        } else {
            "0 assets recovered — pure code mod (unsupported)".to_string()
        }
    } else {
        let joined = parts.join(", ");
        format!("{joined} imported — code logic not supported")
    };

    let summary = ExtractionSummary {
        mod_id: mod_id.clone(),
        mod_name: mod_name.clone(),
        jar_file: jar_file_name.to_string(),
        total_entries,
        skipped_classes_count,
        textures_recovered,
        total_textures_in_jar: total_textures,
        models_recovered,
        total_models_in_jar: total_models,
        blockstates_recovered,
        total_blockstates_in_jar: total_blockstates,
        sounds_recovered,
        total_sounds_in_jar: total_sounds,
        lang_files_recovered,
        recipes_recovered,
        total_recipes_in_jar: total_recipes,
        loot_tables_recovered,
        total_loot_tables_in_jar: total_loot_tables,
        tags_recovered,
        other_assets_recovered,
        other_data_recovered,
        total_recovered,
        total_skipped,
        validation_warnings,
        formatted_report,
        extracted_at: chrono::Utc::now().to_rfc3339(),
    };

    // Write pack.mcmeta for native Minecraft resource/datapack format compatibility
    let mcmeta = serde_json::json!({
        "pack": {
            "pack_format": 34,
            "description": format!("Extracted assets from {}", mod_name)
        }
    });
    let _ = fs::write(
        extracted_base.join("pack.mcmeta"),
        serde_json::to_string_pretty(&mcmeta).unwrap_or_default(),
    );

    // Save mod manifest
    let manifest = ExtractedModManifest {
        schema_version: 1,
        summary: summary.clone(),
    };
    let _ = fs::write(
        extracted_base.join("extraction_manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap_or_default(),
    );

    // Update instance load order and re-merge
    let _ = register_in_load_order(target_instance_dir, &mod_id, &mod_name);
    let _ = remerge_extracted_content(target_instance_dir);

    Ok(summary)
}

/// Helper to extract from a jar on filesystem
pub fn extract_jar_from_file(
    jar_path: &Path,
    target_instance_dir: &Path,
) -> Result<ExtractionSummary> {
    let file = File::open(jar_path)
        .with_context(|| format!("opening jar file at {}", jar_path.display()))?;
    let archive = ZipArchive::new(file)
        .with_context(|| format!("reading zip archive at {}", jar_path.display()))?;
    let file_name = jar_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("mod.jar");

    extract_jar_content(archive, file_name, target_instance_dir)
}

/// Registers a mod in the load order file (or preserves existing order)
pub fn register_in_load_order(
    instance_dir: &Path,
    mod_id: &str,
    mod_name: &str,
) -> Result<Vec<LoadOrderItem>> {
    let load_order_path = instance_dir.join("horizon_extracted").join("load_order.json");
    let mut order = load_load_order(instance_dir).unwrap_or_default();

    if let Some(existing) = order.iter_mut().find(|i| i.mod_id == mod_id) {
        existing.mod_name = mod_name.to_string();
    } else {
        let next_priority = order.len();
        order.push(LoadOrderItem {
            mod_id: mod_id.to_string(),
            mod_name: mod_name.to_string(),
            priority: next_priority,
            enabled: true,
            relative_dir: format!("horizon_extracted/{mod_id}"),
        });
    }

    if let Some(parent) = load_order_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::write(
        &load_order_path,
        serde_json::to_string_pretty(&order)?,
    )?;

    Ok(order)
}

/// Reads current load order for an instance
pub fn load_load_order(instance_dir: &Path) -> Result<Vec<LoadOrderItem>> {
    let load_order_path = instance_dir.join("horizon_extracted").join("load_order.json");
    if !load_order_path.is_file() {
        return Ok(Vec::new());
    }
    let data = fs::read_to_string(load_order_path)?;
    let order: Vec<LoadOrderItem> = serde_json::from_str(&data)?;
    Ok(order)
}

/// Updates load order and priority
pub fn update_load_order(
    instance_dir: &Path,
    ordered_mod_ids: &[String],
) -> Result<Vec<LoadOrderItem>> {
    let mut current = load_load_order(instance_dir)?;
    let mut new_order = Vec::new();

    for (new_pri, id) in ordered_mod_ids.iter().enumerate() {
        if let Some(mut item) = current.iter().find(|i| &i.mod_id == id).cloned() {
            item.priority = new_pri;
            new_order.push(item);
        }
    }

    // Append any items that were omitted
    for item in current.iter_mut() {
        if !ordered_mod_ids.contains(&item.mod_id) {
            item.priority = new_order.len();
            new_order.push(item.clone());
        }
    }

    let load_order_path = instance_dir.join("horizon_extracted").join("load_order.json");
    fs::write(
        &load_order_path,
        serde_json::to_string_pretty(&new_order)?,
    )?;

    remerge_extracted_content(instance_dir)?;
    Ok(new_order)
}

/// Enables or disables an extracted mod and remerges
pub fn toggle_extracted_mod(
    instance_dir: &Path,
    mod_id: &str,
    enabled: bool,
) -> Result<()> {
    let mut order = load_load_order(instance_dir)?;
    if let Some(item) = order.iter_mut().find(|i| i.mod_id == mod_id) {
        item.enabled = enabled;
    } else {
        bail!("Mod '{mod_id}' not found in load order");
    }

    let load_order_path = instance_dir.join("horizon_extracted").join("load_order.json");
    fs::write(
        &load_order_path,
        serde_json::to_string_pretty(&order)?,
    )?;

    remerge_extracted_content(instance_dir)?;
    Ok(())
}

/// Merges extracted content using vanilla resource pack override rules:
/// Later-loaded (higher priority index) packs override earlier ones per namespace+path.
pub fn remerge_extracted_content(instance_dir: &Path) -> Result<MergedAssetsState> {
    let load_order = load_load_order(instance_dir)?;
    let merged_base = instance_dir.join("horizon_extracted").join("merged");

    // Clean previous merged directory
    if merged_base.is_dir() {
        let _ = fs::remove_dir_all(&merged_base);
    }
    fs::create_dir_all(&merged_base)?;

    let mut active_mods = Vec::new();
    let mut override_conflicts = Vec::new();
    let mut file_owner_map: HashMap<String, String> = HashMap::new();
    let mut unique_assets = 0usize;
    let mut unique_data = 0usize;

    // Filter enabled mods and sort by priority ascending (lower priority applied first)
    let mut enabled_items: Vec<_> = load_order.iter().filter(|i| i.enabled).collect();
    enabled_items.sort_by_key(|i| i.priority);

    for item in enabled_items {
        active_mods.push(item.mod_id.clone());
        let mod_dir = instance_dir.join("horizon_extracted").join(&item.mod_id);
        if !mod_dir.is_dir() {
            continue;
        }

        // Walk all files in assets/ and data/
        for root_prefix in ["assets", "data"] {
            let prefix_dir = mod_dir.join(root_prefix);
            if !prefix_dir.is_dir() {
                continue;
            }

            for entry in walk_dir_recursive(&prefix_dir)? {
                if let Ok(rel) = entry.strip_prefix(&mod_dir) {
                    let rel_str = rel.to_string_lossy().replace('\\', "/");

                    // Check for override conflict
                    if let Some(prev_mod) = file_owner_map.insert(rel_str.clone(), item.mod_id.clone()) {
                        override_conflicts.push(AssetOverrideRecord {
                            relative_path: rel_str.clone(),
                            winning_mod_id: item.mod_id.clone(),
                            overridden_mod_id: prev_mod,
                        });
                    } else if rel_str.starts_with("assets/") {
                        unique_assets += 1;
                    } else if rel_str.starts_with("data/") {
                        unique_data += 1;
                    }

                    // Copy file to merged location
                    let dest_path = merged_base.join(rel);
                    if let Some(parent) = dest_path.parent() {
                        let _ = fs::create_dir_all(parent);
                    }
                    let _ = fs::copy(&entry, &dest_path);
                }
            }
        }
    }

    // Write pack.mcmeta for merged output
    let merged_mcmeta = serde_json::json!({
        "pack": {
            "pack_format": 34,
            "description": "Horizon Merged Mod Resources & Datapacks"
        }
    });
    let _ = fs::write(
        merged_base.join("pack.mcmeta"),
        serde_json::to_string_pretty(&merged_mcmeta).unwrap_or_default(),
    );

    let state = MergedAssetsState {
        total_unique_assets: unique_assets,
        total_unique_data: unique_data,
        active_mods,
        override_conflicts,
        last_merged_at: chrono::Utc::now().to_rfc3339(),
    };

    let _ = fs::write(
        merged_base.join("merged_state.json"),
        serde_json::to_string_pretty(&state).unwrap_or_default(),
    );

    Ok(state)
}

/// Helper to recursively collect files in a directory
fn walk_dir_recursive(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir_recursive(&path)?);
            } else if path.is_file() {
                files.push(path);
            }
        }
    }
    Ok(files)
}

/// Scans all extracted mods in an instance directory and returns their manifests
pub fn get_instance_extracted_summaries(instance_dir: &Path) -> Result<Vec<ExtractionSummary>> {
    let extracted_root = instance_dir.join("horizon_extracted");
    if !extracted_root.is_dir() {
        return Ok(Vec::new());
    }

    let mut summaries = Vec::new();
    for entry in fs::read_dir(extracted_root)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && path.file_name().and_then(|n| n.to_str()) != Some("merged") {
            let manifest_path = path.join("extraction_manifest.json");
            if manifest_path.is_file()
                && let Ok(data) = fs::read_to_string(&manifest_path)
                    && let Ok(manifest) = serde_json::from_str::<ExtractedModManifest>(&data) {
                        summaries.push(manifest.summary);
                    }
        }
    }

    // Sort by mod_id
    summaries.sort_by(|a, b| a.mod_id.cmp(&b.mod_id));
    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use tempfile::tempdir;
    use zip::write::{SimpleFileOptions, ZipWriter};

    fn create_test_mod_jar(
        mod_id: &str,
        with_classes: bool,
        valid_model: bool,
        valid_recipe: bool,
    ) -> Vec<u8> {
        let mut buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut buf));
            let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

            // fabric.mod.json
            zip.start_file("fabric.mod.json", options).unwrap();
            let manifest = serde_json::json!({
                "schemaVersion": 1,
                "id": mod_id,
                "name": format!("Test Mod {mod_id}"),
                "version": "1.0.0"
            });
            zip.write_all(manifest.to_string().as_bytes()).unwrap();

            // textures
            zip.start_file(format!("assets/{mod_id}/textures/item/ruby.png"), options).unwrap();
            let png_bytes = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00];
            zip.write_all(&png_bytes).unwrap();

            // models
            zip.start_file(format!("assets/{mod_id}/models/item/ruby.json"), options).unwrap();
            if valid_model {
                let model = serde_json::json!({
                    "parent": "minecraft:item/generated",
                    "textures": {
                        "layer0": format!("{mod_id}:item/ruby")
                    }
                });
                zip.write_all(model.to_string().as_bytes()).unwrap();
            } else {
                zip.write_all(b"not a valid json model").unwrap();
            }

            // recipes
            zip.start_file(format!("data/{mod_id}/recipes/ruby_block.json"), options).unwrap();
            if valid_recipe {
                let recipe = serde_json::json!({
                    "type": "minecraft:crafting_shaped",
                    "pattern": ["###", "###", "###"],
                    "key": { "#": { "item": format!("{mod_id}:ruby") } },
                    "result": { "item": format!("{mod_id}:ruby_block"), "count": 1 }
                });
                zip.write_all(recipe.to_string().as_bytes()).unwrap();
            } else {
                let bad_recipe = serde_json::json!({ "missing_type": true });
                zip.write_all(bad_recipe.to_string().as_bytes()).unwrap();
            }

            // JVM classes if requested
            if with_classes {
                zip.start_file(format!("net/example/{mod_id}/ModEntry.class"), options).unwrap();
                zip.write_all(&[0xca, 0xfe, 0xba, 0xbe, 0x00, 0x00, 0x00, 0x3d]).unwrap();
            }

            zip.finish().unwrap();
        }
        buf
    }

    #[test]
    fn test_extract_jar_content_and_schema_validation() {
        let temp = tempfile::tempdir().unwrap();
        let jar_bytes = create_test_mod_jar("ruby_mod", true, true, true);
        let archive = ZipArchive::new(Cursor::new(jar_bytes)).unwrap();

        let summary = extract_jar_content(archive, "ruby_mod-1.0.jar", temp.path()).unwrap();

        assert_eq!(summary.mod_id, "ruby_mod");
        assert_eq!(summary.skipped_classes_count, 1);
        assert_eq!(summary.textures_recovered, 1);
        assert_eq!(summary.models_recovered, 1);
        assert_eq!(summary.recipes_recovered, 1);
        assert_eq!(summary.validation_warnings.len(), 0);
        assert!(summary.formatted_report.contains("1 of 1 textures"));
        assert!(summary.formatted_report.contains("1 of 1 models"));
        assert!(summary.formatted_report.contains("1 of 1 recipes"));
        assert!(summary.formatted_report.contains("code logic not supported"));

        // Confirm extracted files exist
        let extracted_item = temp.path().join("horizon_extracted/ruby_mod/assets/ruby_mod/textures/item/ruby.png");
        assert!(extracted_item.is_file());
    }

    #[test]
    fn test_extract_invalid_schema_reports_warnings() {
        let temp = tempfile::tempdir().unwrap();
        let jar_bytes = create_test_mod_jar("corrupt_mod", false, false, false);
        let archive = ZipArchive::new(Cursor::new(jar_bytes)).unwrap();

        let summary = extract_jar_content(archive, "corrupt_mod.jar", temp.path()).unwrap();

        assert_eq!(summary.mod_id, "corrupt_mod");
        assert_eq!(summary.textures_recovered, 1);
        assert_eq!(summary.models_recovered, 0);
        assert_eq!(summary.recipes_recovered, 0);
        assert_eq!(summary.validation_warnings.len(), 2);
    }

    #[test]
    fn test_load_order_and_override_merger() {
        let temp = tempfile::tempdir().unwrap();

        // Mod 1: provides a texture
        let jar1 = create_test_mod_jar("base_mod", false, true, true);
        extract_jar_content(ZipArchive::new(Cursor::new(jar1)).unwrap(), "base_mod.jar", temp.path()).unwrap();

        // Mod 2: overrides the exact same texture namespace/path
        let mut jar2_buf = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut jar2_buf));
            let opt = SimpleFileOptions::default();
            zip.start_file("fabric.mod.json", opt).unwrap();
            zip.write_all(b"{\"id\": \"override_mod\", \"name\": \"Override Mod\"}").unwrap();
            // Same texture path
            zip.start_file("assets/base_mod/textures/item/ruby.png", opt).unwrap();
            // Distinct bytes for override
            zip.write_all(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0xFF, 0xFF]).unwrap();
            zip.finish().unwrap();
        }
        extract_jar_content(ZipArchive::new(Cursor::new(jar2_buf)).unwrap(), "override_mod.jar", temp.path()).unwrap();

        let state = remerge_extracted_content(temp.path()).unwrap();
        assert_eq!(state.active_mods.len(), 2);
        assert_eq!(state.override_conflicts.len(), 1);
        assert_eq!(state.override_conflicts[0].overridden_mod_id, "base_mod");
        assert_eq!(state.override_conflicts[0].winning_mod_id, "override_mod");

        // Verify merged file has winning bytes
        let merged_file = temp.path().join("horizon_extracted/merged/assets/base_mod/textures/item/ruby.png");
        assert!(merged_file.is_file());
        let merged_bytes = fs::read(merged_file).unwrap();
        assert_eq!(merged_bytes[8..10], [0xFF, 0xFF]);

        // Test toggling mod off
        toggle_extracted_mod(temp.path(), "override_mod", false).unwrap();
        let state2 = remerge_extracted_content(temp.path()).unwrap();
        assert_eq!(state2.active_mods.len(), 1);
        assert_eq!(state2.override_conflicts.len(), 0);

        // Verify merged file restored to base_mod bytes
        let merged_bytes2 = fs::read(temp.path().join("horizon_extracted/merged/assets/base_mod/textures/item/ruby.png")).unwrap();
        assert_eq!(merged_bytes2[8..10], [0x00, 0x00]);
    }

    #[test]
    fn test_demo_resource_heavy_modpack_extraction() {
        let temp = tempfile::tempdir().unwrap();

        // 1. Create realistic multi-loader mod JAR: "chisel_and_bits" (Forge)
        // 40 textures, 15 models, 8 recipes, 120 JVM classes
        let mut chisel_jar = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut chisel_jar));
            let opt = SimpleFileOptions::default();

            // META-INF/mods.toml
            zip.start_file("META-INF/mods.toml", opt).unwrap();
            let toml = r#"
[[mods]]
modId = "chisel_and_bits"
displayName = "Chisels & Bits (Java)"
version = "1.4.0"
"#;
            zip.write_all(toml.as_bytes()).unwrap();

            // 40 textures
            for i in 1..=40 {
                zip.start_file(format!("assets/chisel_and_bits/textures/block/chiseled_{i}.png"), opt).unwrap();
                let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, i as u8, 0x00];
                zip.write_all(&png).unwrap();
            }

            // 15 models
            for i in 1..=15 {
                zip.start_file(format!("assets/chisel_and_bits/models/block/chiseled_{i}.json"), opt).unwrap();
                let model = serde_json::json!({
                    "parent": "block/cube_all",
                    "textures": { "all": format!("chisel_and_bits:block/chiseled_{i}") }
                });
                zip.write_all(model.to_string().as_bytes()).unwrap();
            }

            // 8 recipes (1 intentionally malformed schema to verify error reporting)
            for i in 1..=7 {
                zip.start_file(format!("data/chisel_and_bits/recipes/recipe_{i}.json"), opt).unwrap();
                let recipe = serde_json::json!({
                    "type": "minecraft:crafting_shaped",
                    "pattern": [" # ", "# #", " # "],
                    "key": { "#": { "item": "minecraft:iron_ingot" } },
                    "result": { "item": format!("chisel_and_bits:bit_{i}"), "count": 4 }
                });
                zip.write_all(recipe.to_string().as_bytes()).unwrap();
            }
            // 8th recipe has missing type
            zip.start_file("data/chisel_and_bits/recipes/recipe_8_malformed.json", opt).unwrap();
            zip.write_all(b"{\"result\": {\"item\": \"chisel_and_bits:bad\"}}").unwrap();

            // 120 compiled Java bytecode classes
            for i in 1..=120 {
                zip.start_file(format!("mod/chisels/core/Class_{i}.class"), opt).unwrap();
                zip.write_all(&[0xca, 0xfe, 0xba, 0xbe, 0x00, 0x00, 0x00, 0x3d]).unwrap();
            }

            zip.finish().unwrap();
        }

        // Run extraction
        let archive = ZipArchive::new(Cursor::new(chisel_jar)).unwrap();
        let summary = extract_jar_content(archive, "chisel_and_bits-1.4.0.jar", temp.path()).unwrap();

        assert_eq!(summary.mod_id, "chisel_and_bits");
        assert_eq!(summary.mod_name, "Chisels & Bits (Java)");
        assert_eq!(summary.skipped_classes_count, 120);
        assert_eq!(summary.textures_recovered, 40);
        assert_eq!(summary.total_textures_in_jar, 40);
        assert_eq!(summary.models_recovered, 15);
        assert_eq!(summary.total_models_in_jar, 15);
        assert_eq!(summary.recipes_recovered, 7);
        assert_eq!(summary.total_recipes_in_jar, 8);
        assert_eq!(summary.validation_warnings.len(), 1);
        assert_eq!(summary.validation_warnings[0].category, AssetCategory::Recipe);
        assert!(summary.validation_warnings[0].reason.contains("missing required 'type' field"));

        // Confirm human-readable report matches specification
        assert!(summary.formatted_report.contains("40 of 40 textures"));
        assert!(summary.formatted_report.contains("15 of 15 models"));
        assert!(summary.formatted_report.contains("7 of 8 recipes"));
        assert!(summary.formatted_report.contains("code logic not supported"));

        // Verify summary retrieval API
        let summaries = get_instance_extracted_summaries(temp.path()).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].mod_id, "chisel_and_bits");

        // Verify load order
        let load_order = load_load_order(temp.path()).unwrap();
        assert_eq!(load_order.len(), 1);
        assert_eq!(load_order[0].mod_id, "chisel_and_bits");
        assert!(load_order[0].enabled);

        // Verify merged assets exist and contain the 40 textures
        for i in 1..=40 {
            let p = temp.path().join(format!("horizon_extracted/merged/assets/chisel_and_bits/textures/block/chiseled_{i}.png"));
            assert!(p.is_file(), "Merged texture #{i} missing");
        }
    }

    #[test]
    fn test_reject_zip_slip_traversal_entry() {
        let temp = tempdir().unwrap();
        let mut jar_data = Vec::new();
        {
            let mut zip = ZipWriter::new(Cursor::new(&mut jar_data));
            let opt = SimpleFileOptions::default();

            // Mod manifest
            zip.start_file("fabric.mod.json", opt).unwrap();
            zip.write_all(br#"{"id": "evil_mod", "name": "Evil Mod", "version": "1.0.0"}"#).unwrap();

            // Malicious Zip-Slip path entry
            zip.start_file("assets/evil_mod/textures/../../../../evil.txt", opt).unwrap();
            let png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x01, 0x00];
            zip.write_all(&png).unwrap();

            zip.finish().unwrap();
        }

        let archive = ZipArchive::new(Cursor::new(jar_data)).unwrap();
        let summary = extract_jar_content(archive, "evil_mod-1.0.jar", temp.path()).unwrap();

        // Ensure evil file was NOT written outside
        let evil_target = temp.path().join("evil.txt");
        assert!(!evil_target.exists(), "Zip-Slip path traversal vulnerability: file written outside target!");

        // Ensure warning was recorded
        assert!(
            summary.validation_warnings.iter().any(|w| w.reason.contains("Path traversal / Zip-Slip detected")),
            "Expected Zip-Slip validation warning"
        );
    }
}
