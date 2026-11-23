//! Resource preparation engine for Horizon
//!
//! Rebuilds Horizon-compatible game resources locally from the user's own Minecraft 26.3 copy:
//! verifies jar checksums, extracts client textures, models, blockstates, fonts, languages,
//! and copies sound events from the user's asset store.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::releases::horizon_data_dir;
use crate::requirements::{
    MINECRAFT_26_3_CLIENT_SHA1, MINECRAFT_26_3_VERSION, check_minecraft_26_3,
    find_default_minecraft_dirs,
};
use anyhow::{Context, Result, bail};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use sha2::Sha256;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use zip::ZipArchive;

pub const MANIFEST_NAME: &str = "horizon-resources.json";
pub const SCHEMA_VERSION: i64 = 7;

static CURRENT_CANCEL_FLAG: Mutex<Option<Arc<AtomicBool>>> = Mutex::new(None);

/// Signals cancellation to any ongoing resource preparation job.
pub fn cancel_preparation() {
    let mut guard = CURRENT_CANCEL_FLAG.lock();
    if let Some(flag) = guard.take() {
        flag.store(true, Ordering::Relaxed);
    }
}

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_sha256: Option<String>,
    pub resource_files: usize,
    pub has_sounds: bool,
    pub prepared_at: String,
}

pub fn prepared_resources_dir() -> PathBuf {
    horizon_data_dir().join("resources")
}

pub fn get_resources_manifest() -> Option<ResourcesManifest> {
    let manifest_path = prepared_resources_dir().join(MANIFEST_NAME);
    let content = fs::read_to_string(manifest_path).ok()?;
    serde_json::from_str::<ResourcesManifest>(&content).ok()
}

pub fn is_prepared_for_version(version: &str) -> bool {
    let manifest_path = prepared_resources_dir().join(MANIFEST_NAME);
    if !manifest_path.is_file() {
        return false;
    }

    let Ok(content) = fs::read_to_string(&manifest_path) else {
        return false;
    };

    let Ok(manifest) = serde_json::from_str::<ResourcesManifest>(&content) else {
        return false;
    };

    if manifest.schema < SCHEMA_VERSION || manifest.minecraft != version {
        return false;
    }

    let res_dir = prepared_resources_dir();
    res_dir.join("assets/minecraft").is_dir()
}

pub fn is_prepared() -> bool {
    is_prepared_for_version(MINECRAFT_26_3_VERSION)
}

pub fn has_sounds() -> bool {
    let dir = prepared_resources_dir();
    dir.join("assets/minecraft/sounds.json").is_file()
}

pub fn compute_file_checksums(path: &Path) -> Result<(String, String)> {
    let file = File::open(path)
        .with_context(|| format!("opening {}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut sha1_hasher = Sha1::new();
    let mut sha256_hasher = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        sha1_hasher.update(&buf[..n]);
        sha256_hasher.update(&buf[..n]);
    }
    let sha1 = format!("{:x}", sha1_hasher.finalize());
    let sha256 = format!("{:x}", sha256_hasher.finalize());
    Ok((sha1, sha256))
}

pub fn run_preparation<F>(
    custom_client_jar: Option<PathBuf>,
    custom_minecraft_dir: Option<PathBuf>,
    force: bool,
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

    // 0. Cache check: if valid resources exist and not forced, return cached directory
    if !force && is_prepared_for_version(MINECRAFT_26_3_VERSION)
        && let Some(manifest) = get_resources_manifest() {
            notify(
                "Cached",
                &format!(
                    "Using cached game resources for Minecraft {} (prepared at {}).",
                    manifest.minecraft, manifest.prepared_at
                ),
                1.0,
                true,
                None,
            );
            return Ok(prepared_resources_dir());
        }

    // Set up cancellation token
    let cancel_flag = Arc::new(AtomicBool::new(false));
    {
        let mut guard = CURRENT_CANCEL_FLAG.lock();
        *guard = Some(cancel_flag.clone());
    }

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

    if cancel_flag.load(Ordering::Relaxed) {
        notify("Cancelled", "Resource preparation was cancelled by user.", 0.0, false, Some("Cancelled by user".to_string()));
        bail!("Resource preparation was cancelled by user");
    }

    // 2. Validate jar SHA-1 and compute SHA-256
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

    let (jar_sha1, jar_sha256) = compute_file_checksums(&client_jar_path)?;
    if jar_sha1 != MINECRAFT_26_3_CLIENT_SHA1 {
        let err = format!(
            "Jar SHA-1 ({jar_sha1}) does not match genuine Minecraft 26.3 ({MINECRAFT_26_3_CLIENT_SHA1})."
        );
        notify("Error", &err, 0.10, false, Some(err.clone()));
        bail!(err);
    }

    if cancel_flag.load(Ordering::Relaxed) {
        notify("Cancelled", "Resource preparation was cancelled by user.", 0.0, false, Some("Cancelled by user".to_string()));
        bail!("Resource preparation was cancelled by user");
    }

    // 3. Prepare staging output directory for atomic extraction
    let staging_dir = horizon_data_dir().join("resources_staging");
    if staging_dir.exists() {
        let _ = fs::remove_dir_all(&staging_dir);
    }
    fs::create_dir_all(&staging_dir).with_context(|| {
        format!("creating staging directory {}", staging_dir.display())
    })?;

    // 4. Extract assets and data from client jar into staging directory
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
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = fs::remove_dir_all(&staging_dir);
            notify("Cancelled", "Resource preparation was cancelled by user.", 0.0, false, Some("Cancelled by user".to_string()));
            bail!("Resource preparation was cancelled by user");
        }

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

        let target_path = staging_dir.join(&name);
        if let Some(parent) = target_path.parent()
            && !parent.exists() {
                fs::create_dir_all(parent)?;
            }

        let mut out_file = File::create(&target_path)
            .with_context(|| format!("creating {}", target_path.display()))?;
        std::io::copy(&mut file, &mut out_file)?;
        extracted_count += 1;

        if extracted_count.is_multiple_of(300) {
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
        if cancel_flag.load(Ordering::Relaxed) {
            let _ = fs::remove_dir_all(&staging_dir);
            notify("Cancelled", "Resource preparation was cancelled by user.", 0.0, false, Some("Cancelled by user".to_string()));
            bail!("Resource preparation was cancelled by user");
        }

        let assets_dir = mc_dir.join("assets");
        let indexes_dir = assets_dir.join("indexes");
        let objects_dir = assets_dir.join("objects");

        if !indexes_dir.is_dir() || !objects_dir.is_dir() {
            continue;
        }

        let index_candidates = ["34.json", "26.3.json"];
        let mut chosen_index: Option<PathBuf> = None;

        for name in index_candidates {
            let p = indexes_dir.join(name);
            if p.is_file() {
                chosen_index = Some(p);
                break;
            }
        }

        if chosen_index.is_none()
            && let Ok(entries) = fs::read_dir(&indexes_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.extension().map(|e| e == "json").unwrap_or(false) {
                        chosen_index = Some(p);
                        break;
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

            if let Ok(content) = fs::read_to_string(&index_path)
                && let Ok(index_json) =
                    serde_json::from_str::<serde_json::Value>(&content)
                    && let Some(objects) =
                        index_json.get("objects").and_then(|o| o.as_object())
                    {
                        let mut sounds_copied = 0usize;

                        for (asset_name, meta) in objects {
                            if cancel_flag.load(Ordering::Relaxed) {
                                let _ = fs::remove_dir_all(&staging_dir);
                                notify("Cancelled", "Resource preparation was cancelled by user.", 0.0, false, Some("Cancelled by user".to_string()));
                                bail!("Resource preparation was cancelled by user");
                            }

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

                            let dest =
                                staging_dir.join("assets").join(asset_name);
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

            if found_sounds {
                break;
            }
        }
    }

    // 6. Write horizon-resources.json manifest in staging
    notify(
        "Manifest",
        "Writing horizon-resources.json manifest...",
        0.95,
        false,
        None,
    );

    let manifest = ResourcesManifest {
        schema: SCHEMA_VERSION,
        minecraft: MINECRAFT_26_3_VERSION.to_string(),
        client_sha1: jar_sha1,
        client_sha256: Some(jar_sha256),
        resource_files: extracted_count,
        has_sounds: found_sounds,
        prepared_at: chrono::Utc::now().to_rfc3339(),
    };

    let manifest_bytes = serde_json::to_string_pretty(&manifest)
        .context("serializing horizon-resources.json")?;
    fs::write(staging_dir.join(MANIFEST_NAME), manifest_bytes)
        .context("writing horizon-resources.json")?;

    // 7. Atomic promotion of staging to final resources dir
    let final_dir = prepared_resources_dir();
    if final_dir.exists() {
        let backup_dir = horizon_data_dir().join("resources_old");
        let _ = fs::remove_dir_all(&backup_dir);
        let _ = fs::rename(&final_dir, &backup_dir);
        if let Err(e) = fs::rename(&staging_dir, &final_dir) {
            let _ = fs::rename(&backup_dir, &final_dir);
            return Err(e).context("promoting prepared resources");
        }
        let _ = fs::remove_dir_all(&backup_dir);
    } else {
        if let Some(parent) = final_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        fs::rename(&staging_dir, &final_dir)
            .context("promoting staging resources directory")?;
    }

    // Clear cancellation token
    {
        let mut guard = CURRENT_CANCEL_FLAG.lock();
        *guard = None;
    }

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

    Ok(final_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manifest_serialization() {
        let manifest = ResourcesManifest {
            schema: SCHEMA_VERSION,
            minecraft: "26.3".to_string(),
            client_sha1: "test_sha1".to_string(),
            client_sha256: Some("test_sha256".to_string()),
            resource_files: 42,
            has_sounds: true,
            prepared_at: "2026-01-01T00:00:00Z".to_string(),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: ResourcesManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.schema, SCHEMA_VERSION);
        assert_eq!(parsed.minecraft, "26.3");
        assert_eq!(parsed.client_sha256, Some("test_sha256".to_string()));
    }

    #[test]
    fn test_cancellation_flag() {
        cancel_preparation(); // Calling cancel when nothing is running should not panic
    }

    #[test]
    fn test_compute_file_checksums_and_verification() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        let test_payload = b"Gradle Minecraft Rust Edition - Verified Checksum Payload";
        tmp.write_all(test_payload).unwrap();
        tmp.flush().unwrap();

        let (sha1, sha256) = compute_file_checksums(tmp.path()).unwrap();
        assert_eq!(sha1.len(), 40);
        assert_eq!(sha256.len(), 64);

        // Verify SHA-256 matches independent sha2 computation
        use sha2::{Digest, Sha256};
        let expected_sha256 = format!("{:x}", Sha256::digest(test_payload));
        assert_eq!(sha256, expected_sha256);

        // Verify SHA-1 matches independent sha1 computation
        use sha1::Sha1;
        let expected_sha1 = format!("{:x}", Sha1::digest(test_payload));
        assert_eq!(sha1, expected_sha1);

        // Test with different payload
        let mut tmp2 = tempfile::NamedTempFile::new().unwrap();
        tmp2.write_all(b"Altered Payload").unwrap();
        tmp2.flush().unwrap();
        let (_sha1_2, sha256_2) = compute_file_checksums(tmp2.path()).unwrap();
        assert_ne!(sha256, sha256_2);
    }
}
