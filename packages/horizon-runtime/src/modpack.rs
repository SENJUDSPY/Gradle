//! Modpack inspector and compatibility analyzer for Horizon (Minecraft 26.3 native Rust)
//!
//! Analyzes Modrinth (.mrpack) and CurseForge (.zip) modpacks to identify which components
//! can be carried over to Horizon (data packs, resource packs, shaders, configs, asset mods)
//! and which Java bytecode mods (.class jars) cannot run natively on Rust.
//!
//! Copyright (c) 2026 AstralRinth contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::path::Path;
use zip::ZipArchive;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModpackItemCategory {
    ResourcePack,
    DataPack,
    ShaderPack,
    Configuration,
    AssetMod,
    JavaBytecodeMod,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackAnalyzedItem {
    pub name: String,
    pub path: String,
    pub category: ModpackItemCategory,
    pub is_compatible: bool,
    pub file_size: Option<u64>,
    pub details: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModpackAnalysis {
    pub pack_format: String,
    pub pack_name: String,
    pub pack_version: String,
    pub author: Option<String>,
    pub summary: Option<String>,
    pub original_game_version: String,
    pub original_loader: String,
    pub total_items: usize,
    pub compatible_count: usize,
    pub incompatible_count: usize,
    pub items: Vec<ModpackAnalyzedItem>,
}

#[derive(Deserialize)]
struct ModrinthIndex {
    name: Option<String>,
    #[serde(rename = "versionId")]
    version_id: Option<String>,
    summary: Option<String>,
    files: Option<Vec<ModrinthFileEntry>>,
    dependencies: Option<HashMap<String, String>>,
}

#[derive(Deserialize)]
struct ModrinthFileEntry {
    path: String,
    #[serde(rename = "fileSize")]
    file_size: Option<u64>,
}

#[derive(Deserialize)]
struct CurseforgeManifest {
    name: Option<String>,
    version: Option<String>,
    author: Option<String>,
    minecraft: Option<CurseforgeMinecraft>,
    files: Option<Vec<CurseforgeFileEntry>>,
    #[allow(dead_code)]
    overrides: Option<String>,
}

#[derive(Deserialize)]
struct CurseforgeMinecraft {
    version: Option<String>,
    #[serde(rename = "modLoaders")]
    mod_loaders: Option<Vec<CurseforgeModLoader>>,
}

#[derive(Deserialize)]
struct CurseforgeModLoader {
    id: Option<String>,
}

#[derive(Deserialize)]
struct CurseforgeFileEntry {
    #[serde(rename = "projectID")]
    project_id: Option<u64>,
    #[allow(dead_code)]
    #[serde(rename = "fileID")]
    file_id: Option<u64>,
}

pub fn analyze_modpack_file(path: &Path) -> Result<ModpackAnalysis> {
    if !path.is_file() {
        bail!("Modpack file not found at {}", path.display());
    }

    let file = File::open(path).context("opening modpack archive")?;
    let mut zip = ZipArchive::new(file).context("reading zip archive")?;

    // Check if this is a Modrinth pack (modrinth.index.json exists)
    let modrinth_content = {
        if let Ok(mut index_entry) = zip.by_name("modrinth.index.json") {
            let mut content = String::new();
            index_entry.read_to_string(&mut content)?;
            Some(content)
        } else {
            None
        }
    };

    if let Some(content) = modrinth_content {
        let index: ModrinthIndex = serde_json::from_str(&content)
            .context("parsing modrinth.index.json")?;
        return analyze_modrinth_pack(index, &mut zip);
    }

    // Check if this is a CurseForge pack (manifest.json exists)
    let curseforge_content = {
        if let Ok(mut manifest_entry) = zip.by_name("manifest.json") {
            let mut content = String::new();
            manifest_entry.read_to_string(&mut content)?;
            Some(content)
        } else {
            None
        }
    };

    if let Some(content) = curseforge_content {
        let manifest: CurseforgeManifest = serde_json::from_str(&content)
            .context("parsing CurseForge manifest.json")?;
        return analyze_curseforge_pack(manifest, &mut zip);
    }

    bail!(
        "Unrecognized modpack format. Neither 'modrinth.index.json' nor 'manifest.json' found in the archive."
    );
}

fn analyze_modrinth_pack(
    index: ModrinthIndex,
    zip: &mut ZipArchive<File>,
) -> Result<ModpackAnalysis> {
    let pack_name =
        index.name.unwrap_or_else(|| "Modrinth Modpack".to_string());
    let pack_version = index.version_id.unwrap_or_else(|| "1.0.0".to_string());
    let summary = index.summary;

    let original_game_version = index
        .dependencies
        .as_ref()
        .and_then(|d| d.get("minecraft").cloned())
        .unwrap_or_else(|| "Unknown".to_string());

    let original_loader = index
        .dependencies
        .as_ref()
        .and_then(|d| {
            if d.contains_key("fabric-loader") {
                Some("Fabric".to_string())
            } else if d.contains_key("forge") {
                Some("Forge".to_string())
            } else if d.contains_key("neoforge") {
                Some("NeoForge".to_string())
            } else if d.contains_key("quilt-loader") {
                Some("Quilt".to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "Vanilla / None".to_string());

    let mut items = Vec::new();

    if let Some(files) = index.files {
        for f in files {
            let p = f.path.replace('\\', "/");
            let name = p.split('/').last().unwrap_or(&p).to_string();

            let (category, is_compatible, details) =
                classify_path(&p, f.file_size);
            items.push(ModpackAnalyzedItem {
                name,
                path: p,
                category,
                is_compatible,
                file_size: f.file_size,
                details,
            });
        }
    }

    // Also scan files inside overrides/
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            let name = entry.name().to_string();
            if entry.is_dir() {
                continue;
            }
            if let Some(rel) = name.strip_prefix("overrides/") {
                let rel_clean = rel.replace('\\', "/");
                if !items.iter().any(|item| item.path == rel_clean) {
                    let file_name = rel_clean
                        .split('/')
                        .last()
                        .unwrap_or(&rel_clean)
                        .to_string();
                    let (category, is_compatible, details) =
                        classify_path(&rel_clean, Some(entry.size()));
                    items.push(ModpackAnalyzedItem {
                        name: file_name,
                        path: rel_clean,
                        category,
                        is_compatible,
                        file_size: Some(entry.size()),
                        details,
                    });
                }
            }
        }
    }

    let total_items = items.len();
    let compatible_count = items.iter().filter(|i| i.is_compatible).count();
    let incompatible_count = total_items.saturating_sub(compatible_count);

    Ok(ModpackAnalysis {
        pack_format: "Modrinth (.mrpack)".to_string(),
        pack_name,
        pack_version,
        author: None,
        summary,
        original_game_version,
        original_loader,
        total_items,
        compatible_count,
        incompatible_count,
        items,
    })
}

fn analyze_curseforge_pack(
    manifest: CurseforgeManifest,
    zip: &mut ZipArchive<File>,
) -> Result<ModpackAnalysis> {
    let pack_name = manifest
        .name
        .unwrap_or_else(|| "CurseForge Modpack".to_string());
    let pack_version = manifest.version.unwrap_or_else(|| "1.0.0".to_string());
    let author = manifest.author;

    let original_game_version = manifest
        .minecraft
        .as_ref()
        .and_then(|m| m.version.clone())
        .unwrap_or_else(|| "Unknown".to_string());

    let original_loader = manifest
        .minecraft
        .as_ref()
        .and_then(|m| m.mod_loaders.as_ref())
        .and_then(|loaders| loaders.first())
        .and_then(|l| l.id.clone())
        .unwrap_or_else(|| "Forge / Fabric".to_string());

    let mut items = Vec::new();

    // Manifest listed mods (CurseForge projects)
    if let Some(files) = manifest.files {
        for f in files {
            let name = format!("CurseForge Mod #{}", f.project_id.unwrap_or(0));
            items.push(ModpackAnalyzedItem {
				name,
				path: format!("mods/curseforge_{}.jar", f.project_id.unwrap_or(0)),
				category: ModpackItemCategory::JavaBytecodeMod,
				is_compatible: false,
				file_size: None,
				details: "Java bytecode mod - skipped (Horizon is native Rust, no JVM)".to_string(),
			});
        }
    }

    // Overrides (bundled resource packs, data packs, configs, shaders)
    for i in 0..zip.len() {
        if let Ok(entry) = zip.by_index(i) {
            let name = entry.name().to_string();
            if entry.is_dir() {
                continue;
            }
            if let Some(rel) = name.strip_prefix("overrides/") {
                let rel_clean = rel.replace('\\', "/");
                let file_name = rel_clean
                    .split('/')
                    .last()
                    .unwrap_or(&rel_clean)
                    .to_string();
                let (category, is_compatible, details) =
                    classify_path(&rel_clean, Some(entry.size()));
                items.push(ModpackAnalyzedItem {
                    name: file_name,
                    path: rel_clean,
                    category,
                    is_compatible,
                    file_size: Some(entry.size()),
                    details,
                });
            }
        }
    }

    let total_items = items.len();
    let compatible_count = items.iter().filter(|i| i.is_compatible).count();
    let incompatible_count = total_items.saturating_sub(compatible_count);

    Ok(ModpackAnalysis {
        pack_format: "CurseForge (.zip)".to_string(),
        pack_name,
        pack_version,
        author,
        summary: None,
        original_game_version,
        original_loader,
        total_items,
        compatible_count,
        incompatible_count,
        items,
    })
}

fn classify_path(
    path: &str,
    _file_size: Option<u64>,
) -> (ModpackItemCategory, bool, String) {
    let lower = path.to_lowercase();

    if lower.starts_with("resourcepacks/") || lower.contains("/resourcepacks/")
    {
        (
			ModpackItemCategory::ResourcePack,
			true,
			"Resource Pack · Custom textures, sounds, and models (100% compatible)".to_string(),
		)
    } else if lower.starts_with("datapacks/") || lower.contains("/datapacks/") {
        (
			ModpackItemCategory::DataPack,
			true,
			"Data Pack · Custom recipes, loot, advancements, and worldgen (100% compatible)".to_string(),
		)
    } else if lower.starts_with("shaderpacks/")
        || lower.contains("/shaderpacks/")
    {
        (
            ModpackItemCategory::ShaderPack,
            true,
            "Shader Pack · Compatible with Horizon wgpu graphics pipeline"
                .to_string(),
        )
    } else if lower.starts_with("config/")
        || lower.starts_with("defaultconfigs/")
    {
        (
            ModpackItemCategory::Configuration,
            true,
            "Configuration file · Game and feature presets (carried over)"
                .to_string(),
        )
    } else if lower.starts_with("mods/") {
        let is_asset_mod = lower.contains("resource")
            || lower.contains("texture")
            || lower.contains("sound")
            || lower.contains("lang")
            || lower.contains("asset")
            || lower.contains("music")
            || lower.contains("animation")
            || lower.contains("datapack")
            || lower.contains("fabulous");

        if is_asset_mod {
            (
				ModpackItemCategory::AssetMod,
				true,
				"Asset & Data Mod · Pure resources/datapack jar (carried over to Horizon)".to_string(),
			)
        } else {
            (
				ModpackItemCategory::JavaBytecodeMod,
				false,
				"Java bytecode jar · Skipped (Requires JVM; Horizon runs native Rust)".to_string(),
			)
        }
    } else {
        (
            ModpackItemCategory::Other,
            true,
            "Auxiliary game asset or data file".to_string(),
        )
    }
}

pub fn extract_compatible_assets_to_dir(
    pack_path: &Path,
    target_dir: &Path,
) -> Result<usize> {
    let file = File::open(pack_path).context("opening modpack archive")?;
    let mut zip = ZipArchive::new(file).context("reading zip archive")?;
    let mut extracted_count = 0;

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();

        let rel_path = if let Some(stripped) = name.strip_prefix("overrides/") {
            stripped
        } else if let Some(stripped) = name.strip_prefix("client-overrides/") {
            stripped
        } else if name.starts_with("resourcepacks/")
            || name.starts_with("datapacks/")
            || name.starts_with("config/")
            || name.starts_with("shaderpacks/")
        {
            &name
        } else {
            continue;
        };

        if entry.is_dir() || rel_path.is_empty() {
            continue;
        }

        let is_compatible_target = rel_path.starts_with("resourcepacks/")
            || rel_path.starts_with("datapacks/")
            || rel_path.starts_with("config/")
            || rel_path.starts_with("shaderpacks/");

        if !is_compatible_target {
            continue;
        }

        let out_path = target_dir.join(rel_path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut out_file = File::create(&out_path)?;
        std::io::copy(&mut entry, &mut out_file)?;
        extracted_count += 1;
    }

    // Ensure destination folders exist
    let _ = fs::create_dir_all(target_dir.join("resourcepacks"));
    let _ = fs::create_dir_all(target_dir.join("datapacks"));
    let _ = fs::create_dir_all(target_dir.join("config"));

    Ok(extracted_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;

    #[test]
    fn test_classify_modpack_items() {
        let (cat, comp, _) = classify_path("resourcepacks/Faithful.zip", None);
        assert_eq!(cat, ModpackItemCategory::ResourcePack);
        assert!(comp);

        let (cat, comp, _) = classify_path("datapacks/Terralith.zip", None);
        assert_eq!(cat, ModpackItemCategory::DataPack);
        assert!(comp);

        let (cat, comp, _) = classify_path("config/sodium-options.json", None);
        assert_eq!(cat, ModpackItemCategory::Configuration);
        assert!(comp);

        let (cat, comp, _) =
            classify_path("mods/FreshAnimations_v1.9.jar", None);
        assert_eq!(cat, ModpackItemCategory::AssetMod);
        assert!(comp);

        let (cat, comp, _) =
            classify_path("mods/sodium-fabric-0.5.8.jar", None);
        assert_eq!(cat, ModpackItemCategory::JavaBytecodeMod);
        assert!(!comp);
    }

    #[test]
    fn test_analyze_mock_mrpack() {
        let dir = tempdir().unwrap();
        let pack_path = dir.path().join("test.mrpack");

        // Create mock .mrpack
        {
            let file = File::create(&pack_path).unwrap();
            let mut zip = zip::ZipWriter::new(file);
            let opts = SimpleFileOptions::default();

            zip.start_file("modrinth.index.json", opts).unwrap();
            let index_json = r#"{
				"formatVersion": 1,
				"game": "minecraft",
				"versionId": "1.0.0",
				"name": "Test Pack",
				"summary": "A test pack",
				"files": [
					{ "path": "resourcepacks/BareBones.zip", "fileSize": 1024 },
					{ "path": "datapacks/VanillaTweaks.zip", "fileSize": 2048 },
					{ "path": "mods/sodium-fabric.jar", "fileSize": 4096 }
				],
				"dependencies": {
					"minecraft": "1.20.4",
					"fabric-loader": "0.15.7"
				}
			}"#;
            zip.write_all(index_json.as_bytes()).unwrap();

            zip.start_file("overrides/config/options.txt", opts)
                .unwrap();
            zip.write_all(b"gamma:1.0").unwrap();

            zip.finish().unwrap();
        }

        let analysis = analyze_modpack_file(&pack_path).unwrap();
        assert_eq!(analysis.pack_name, "Test Pack");
        assert_eq!(analysis.pack_format, "Modrinth (.mrpack)");
        assert_eq!(analysis.total_items, 4);
        assert_eq!(analysis.compatible_count, 3); // resourcepack, datapack, config
        assert_eq!(analysis.incompatible_count, 1); // sodium-fabric.jar

        // Test extraction to target
        let target = dir.path().join("instance");
        let extracted =
            extract_compatible_assets_to_dir(&pack_path, &target).unwrap();
        assert_eq!(extracted, 1); // config/options.txt extracted from overrides
        assert!(target.join("config/options.txt").is_file());
    }
}
