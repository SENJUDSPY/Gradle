//! Resource preparation engine for Horizon
//!
//! Rebuilds Horizon-compatible game resources locally from the user's own Minecraft 26.3 copy:
//! verifies jar checksums, extracts client textures, models, blockstates, fonts, languages,
//! and copies sound events from the user's asset store.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::releases::horizon_data_dir;
use crate::requirements::{
    MINECRAFT_26_3_CLIENT_SHA1, check_minecraft_26_3,
    find_default_minecraft_dirs,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::time::Instant;
use zip::ZipArchive;

pub const MANIFEST_NAME: &str = "horizon-resources.json";
pub const SCHEMA_VERSION: i64 = 7;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreparationProgress {
    pub stage: String,
    pub details: String,
    pub percent: f32,
    pub finished: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesManifest {
    pub schema: i64,
    pub minecraft: String,
    pub client_sha1: String,
    pub resource_files: usize,
    pub has_sounds: bool,
    pub prepared_at: String,
}

pub fn prepared_resources_dir() -> PathBuf {
    horizon_data_dir().join("resources")
}

pub fn is_prepared() -> bool {
    prepared_resources_dir().join(MANIFEST_NAME).is_file()
}

pub fn has_sounds() -> bool {
    let dir = prepared_resources_dir();
    dir.join("assets/minecraft/sounds.json").is_file()
}

fn compute_file_sha1(path: &Path) -> Result<String> {
    let file = File::open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn run_preparation<F>(
    custom_client_jar: Option<PathBuf>,
    custom_minecraft_dir: Option<PathBuf>,
    progress: F,
) -> Result<PathBuf>
where
    F: Fn(PreparationProgress) + Send + Sync + 'static,
{
    let notify = |stage: &str,
                  details: &str,
                  percent: f32,
                  finished: bool,
                  error: Option<String>| {
        progress(PreparationProgress {
            stage: stage.to_string(),
            details: details.to_string(),
            percent,
            finished,
            error,
        });
    };

    let start_time = Instant::now();
    notify(
        "Verifying",
        "Locating Minecraft 26.3 client jar...",
        0.05,
        false,
        None,
    );

    // 1. Locate client jar
    let client_jar_path = if let Some(path) = custom_client_jar {
        if !path.is_file() {
            let err = format!(
                "Specified Minecraft jar not found at {}",
                path.display()
            );
            notify("Error", &err, 0.05, false, Some(err.clone()));
            bail!(err);
        }
        path
    } else {
        let check = check_minecraft_26_3(None);
        if let Some(p) = check.path {
            PathBuf::from(p)
        } else {
            let err = "Could not locate Minecraft 26.3 client jar. Please specify the path manually.".to_string();
            notify("Error", &err, 0.05, false, Some(err.clone()));
            bail!(err);
        }
    };

    // 2. Validate jar SHA-1
    notify(
        "Verifying",
        &format!(
            "Validating checksum for {}...",
            client_jar_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
        ),
        0.10,
        false,
        None,
    );

    let jar_sha1 = compute_file_sha1(&client_jar_path)?;
    if jar_sha1 != MINECRAFT_26_3_CLIENT_SHA1 {
        let err = format!(
            "Jar SHA-1 ({jar_sha1}) does not match genuine Minecraft 26.3 ({MINECRAFT_26_3_CLIENT_SHA1})."
        );
        notify("Error", &err, 0.10, false, Some(err.clone()));
        bail!(err);
    }

    // 3. Prepare target output directory
    notify(
        "Workspace",
        "Preparing output resources workspace...",
        0.15,
        false,
        None,
    );

    let output_dir = prepared_resources_dir();
    fs::create_dir_all(&output_dir).with_context(|| {
        format!("creating directory {}", output_dir.display())
    })?;

    // 4. Extract assets and data from client jar
    notify(
        "Extracting",
        "Extracting textures, models, blockstates, and data from 26.3 client jar...",
        0.20,
        false,
        None,
    );

    let jar_file = File::open(&client_jar_path)?;
    let mut archive = ZipArchive::new(BufReader::new(jar_file))
        .context("Reading Minecraft jar archive")?;

    let total_entries = archive.len();
    let mut extracted_count = 0usize;

    for i in 0..total_entries {
        let mut file = archive.by_index(i)?;
        let name = file.name().to_string();

        if file.is_dir() {
            continue;
        }

        let is_resource = name.starts_with("assets/")
            || name.starts_with("data/")
            || name == "pack.png"
            || name == "pack.mcmeta"
            || name == "version.json";

        if !is_resource {
            continue;
        }

        let target_path = output_dir.join(&name);
        if let Some(parent) = target_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        let mut out_file = File::create(&target_path)
            .with_context(|| format!("creating {}", target_path.display()))?;
        std::io::copy(&mut file, &mut out_file)?;
        extracted_count += 1;

        if extracted_count % 300 == 0 {
            let pct = 0.20 + (i as f32 / total_entries as f32) * 0.45;
            notify(
                "Extracting",
                &format!("Extracted {extracted_count} game resources..."),
                pct,
                false,
                None,
            );
        }
    }

    notify(
        "Audio & Languages",
        "Scanning user asset store for sound events and language indexes...",
        0.70,
        false,
        None,
    );

    // 5. Look for .minecraft assets (sounds, languages, fonts)
    let mut found_sounds = false;
    let mut candidate_dirs = Vec::new();
    if let Some(ref custom_dir) = custom_minecraft_dir {
        candidate_dirs.push(custom_dir.clone());
    }
    candidate_dirs.extend(find_default_minecraft_dirs());

    for mc_dir in candidate_dirs {
        let assets_dir = mc_dir.join("assets");
        let indexes_dir = assets_dir.join("indexes");
        let objects_dir = assets_dir.join("objects");

        if !indexes_dir.is_dir() || !objects_dir.is_dir() {
            continue;
        }

        // Try indexes (34.json or 26.3.json or newest json)
        let index_candidates = ["34.json", "26.3.json"];
        let mut chosen_index: Option<PathBuf> = None;

        for name in index_candidates {
            let p = indexes_dir.join(name);
            if p.is_file() {
                chosen_index = Some(p);
                break;
            }
        }

        if chosen_index.is_none() {
            if let Ok(entries) = fs::read_dir(&indexes_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map(|e| e == "json").unwrap_or(false) {
                        chosen_index = Some(p);
                        break;
                    }
                }
            }
        }

        if let Some(index_path) = chosen_index {
            notify(
                "Audio & Languages",
                &format!(
                    "Reading asset index {}...",
                    index_path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                ),
                0.78,
                false,
                None,
            );

            if let Ok(content) = fs::read_to_string(&index_path) {
                if let Ok(index_json) =
                    serde_json::from_str::<serde_json::Value>(&content)
                {
                    if let Some(objects) =
                        index_json.get("objects").and_then(|o| o.as_object())
                    {
                        let mut sounds_copied = 0usize;

                        for (asset_name, meta) in objects {
                            let is_sound = asset_name
                                == "minecraft/sounds.json"
                                || asset_name.starts_with("minecraft/sounds/");
                            let is_extra_lang =
                                asset_name.starts_with("minecraft/lang/");

                            if !is_sound && !is_extra_lang {
                                continue;
                            }

                            let Some(hash) =
                                meta.get("hash").and_then(|h| h.as_str())
                            else {
                                continue;
                            };

                            if hash.len() < 2 {
                                continue;
                            }

                            let obj_path =
                                objects_dir.join(&hash[..2]).join(hash);
                            if !obj_path.is_file() {
                                continue;
                            }

                            // Determine target location in resources/assets
                            let dest =
                                output_dir.join("assets").join(asset_name);
                            if let Some(p) = dest.parent() {
                                let _ = fs::create_dir_all(p);
                            }

                            if fs::copy(&obj_path, &dest).is_ok() {
                                extracted_count += 1;
                                if is_sound {
                                    sounds_copied += 1;
                                }
                            }
                        }

                        if sounds_copied > 0 {
                            found_sounds = true;
                        }
                    }
                }
            }

            if found_sounds {
                break;
            }
        }
    }

    // 6. Write horizon-resources.json manifest
    notify(
        "Manifest",
        "Writing horizon-resources.json manifest...",
        0.95,
        false,
        None,
    );

    let manifest = ResourcesManifest {
        schema: SCHEMA_VERSION,
        minecraft: "26.3".to_string(),
        client_sha1: jar_sha1,
        resource_files: extracted_count,
        has_sounds: found_sounds,
        prepared_at: chrono::Utc::now().to_rfc3339(),
    };

    let manifest_bytes = serde_json::to_string_pretty(&manifest)
        .context("serializing horizon-resources.json")?;
    fs::write(output_dir.join(MANIFEST_NAME), manifest_bytes)
        .context("writing horizon-resources.json")?;

    let elapsed = start_time.elapsed();
    notify(
        "Complete",
        &format!(
            "Prepared {extracted_count} game resources successfully in {:.1}s!",
            elapsed.as_secs_f32()
        ),
        1.0,
        true,
        None,
    );

    Ok(output_dir)
}
