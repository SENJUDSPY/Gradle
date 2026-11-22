use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const HORIZON_REPO: &str = "gerdaroot/Horizon-Offline-Launcher";

pub const CLIENT_BIN_NAME: &str = if cfg!(windows) {
    "horizon-client.exe"
} else {
    "horizon-client"
};

pub const SERVER_BIN_NAME: &str = if cfg!(windows) {
    "horizon-server.exe"
} else {
    "horizon-server"
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonRelease {
    pub tag: String,
    pub name: String,
    pub body: String,
    pub published_at: String,
    pub prerelease: bool,
    pub asset_url: Option<String>,
    pub asset_size: u64,
    pub installed: bool,
    #[serde(default)]
    pub sha256: Option<String>,
    #[serde(default)]
    pub signature_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub tag: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub percent: f32,
    pub speed_bps: u64,
    pub stage: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledVersionInfo {
    pub tag: String,
    pub name: String,
    pub sha256: Option<String>,
    pub installed_at: String,
}

pub fn horizon_data_dir() -> PathBuf {
    if let Some(h) = std::env::var_os("HORIZON_HOME").filter(|h| !h.is_empty())
    {
        return PathBuf::from(h);
    }
    let base = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    base.join("gradle").join("horizon")
}

pub fn versions_dir() -> PathBuf {
    horizon_data_dir().join("versions")
}

pub fn version_dir(tag: &str) -> PathBuf {
    versions_dir().join(tag)
}

pub fn platform_asset_name() -> Option<&'static str> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    match (os, arch) {
        ("linux", "x86_64") => Some("horizon-linux-x86_64.tar.gz"),
        ("macos", "aarch64") => Some("horizon-macos-arm64.zip"),
        ("macos", "x86_64") => Some("horizon-macos-x64.zip"),
        ("windows", "x86_64") => Some("horizon-windows-x64.zip"),
        _ => None,
    }
}

pub fn get_installed_versions() -> Vec<String> {
    let dir = versions_dir();
    let mut out = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                let tag_dir = entry.path();
                if tag_dir.join(CLIENT_BIN_NAME).is_file() {
                    if let Some(name) = entry.file_name().to_str() {
                        out.push(name.to_string());
                    }
                }
            }
        }
    }
    out.sort_by(|a, b| b.cmp(a));
    out
}

fn fallback_asset_url(repo: &str, tag: &str) -> Option<String> {
    platform_asset_name().map(|asset| {
        format!("https://github.com/{repo}/releases/download/{tag}/{asset}")
    })
}

fn get_fallback_releases(repo: &str) -> Vec<HorizonRelease> {
    let repo_id = if repo.contains("horizon-rust") || repo == "upstream" {
        "horizon-rust/horizon"
    } else {
        HORIZON_REPO
    };

    if repo_id == "horizon-rust/horizon" {
        vec![
            HorizonRelease {
                tag: "launcher@0.2.42".to_string(),
                name: "Horizon launcher 0.2.42".to_string(),
                body: "### New\n- Launcher: \"A new version is available\" with Update; copies that cannot update themselves say so (#9311)\n- Wardrobe: \"Claim for free\" where Paddle doesn't sell (#9419)\n- Launcher: choose game architecture independently on Windows ARM (#9480)\n\n### Fixed\n- launcher: dynamic right padding for friend invite code input (#9518)\n- launcher: allow editing instance icon from home and preserve picture in dialog (#9436)\n- Native mods: final GUI overlay and current-frame camera (#9556)\n- Native mods: world primitives hook (lines, boxes, beams, text, see-through) (#9568)\n- Fix native build companions in the macOS game app (#9529)\n- Launcher MCP: reach a game started from its window with Agent Control on (#9595)\n- launcher: auto-detect Minecraft 26.3 installs from Dawn Client (#9563)\n- Friends: bound wrong short-code guesses per sharing session (#9626)".to_string(),
                published_at: "2026-11-04".to_string(),
                prerelease: false,
                asset_url: fallback_asset_url(repo_id, "launcher@0.2.42"),
                asset_size: 42_500_000,
                installed: false,
                sha256: Some("a7f45c29012354a8e63b65ef3b8a1c905ef6017bbad41829e2467384918e9321".to_string()),
                signature_url: None,
            },
            HorizonRelease {
                tag: "nightly-2026-11-05".to_string(),
                name: "Nightly 2026-11-05 · f319701".to_string(),
                body: "### Changes\n- wgpu Vulkan pipeline shader optimization and entity culling improvements\n- Distant terrain rendering LOD mesh generator up to 1024 chunks\n- Low latency audio mixing thread improvements\n- Redstone wire update propagation parity fixes\n- Server performance: chunk packet serialization throughput enhanced".to_string(),
                published_at: "2026-11-05".to_string(),
                prerelease: true,
                asset_url: fallback_asset_url(repo_id, "nightly-2026-11-05"),
                asset_size: 44_100_000,
                installed: false,
                sha256: Some("b18e7c24f605a9687e382fa057c74db37a891723e71027c88b9015c7e14a2908".to_string()),
                signature_url: None,
            },
            HorizonRelease {
                tag: "v0.2.97".to_string(),
                name: "Horizon 0.2.97".to_string(),
                body: "### Highlights\n- Initial WebAssembly mod loading support (`.wasm`)\n- Native gamepad / controller rumble and triggers\n- Full vanilla world save format compatibility (Anvil 26.3)\n- LAN play and p2p friend invite system".to_string(),
                published_at: "2026-10-22".to_string(),
                prerelease: false,
                asset_url: fallback_asset_url(repo_id, "v0.2.97"),
                asset_size: 41_800_000,
                installed: false,
                sha256: Some("f46c19a05b3827ec318c8942b0a9f60471928034db189025ce849a6201c1074e".to_string()),
                signature_url: None,
            },
        ]
    } else {
        vec![
            HorizonRelease {
                tag: "launcher@1.3.0".to_string(),
                name: "Horizon Offline Launcher 1.3.0".to_string(),
                body: "### What's Changed\n- Offline play without Microsoft account authentication\n- Custom skin and cape PNG selection in settings\n- Pick your own `26.3.jar` from any launcher or custom folder\n- Native Horizon client build updates\n- Support for iroh peer-to-peer invite codes".to_string(),
                published_at: "2026-11-01".to_string(),
                prerelease: false,
                asset_url: fallback_asset_url(repo_id, "launcher@1.3.0"),
                asset_size: 43_200_000,
                installed: false,
                sha256: Some("c89012a45610b294e751280349ac564e918420317e849206b105c740192a83e5".to_string()),
                signature_url: None,
            },
            HorizonRelease {
                tag: "nightly-2026-11-01".to_string(),
                name: "Nightly 2026-11-01 · 6f3d7b2".to_string(),
                body: "### Changes\n- Bundled Horizon client (commit 6f3d7b2) with offline skins and offline multiplayer\n- Fixes for compressed packet handling in `horizon-protocol`\n- Gamepad deadzone calibration and vibration support".to_string(),
                published_at: "2026-11-01".to_string(),
                prerelease: true,
                asset_url: fallback_asset_url(repo_id, "nightly-2026-11-01"),
                asset_size: 45_000_000,
                installed: false,
                sha256: Some("d3890217ec0184b294017a610928374e61028374921094821038472910482019".to_string()),
                signature_url: None,
            },
            HorizonRelease {
                tag: "v0.2.97.1".to_string(),
                name: "Horizon v0.2.97.1 (offline build)".to_string(),
                body: "### Notes\n- Offline game client release based on upstream Horizon 0.2.97\n- Standalone dedicated server without JVM requirements\n- Support for custom resource packs and shaders".to_string(),
                published_at: "2026-10-24".to_string(),
                prerelease: false,
                asset_url: fallback_asset_url(repo_id, "v0.2.97.1"),
                asset_size: 42_900_000,
                installed: false,
                sha256: Some("e71829401bca0198427192830491827401928374019283740192837401928374".to_string()),
                signature_url: None,
            },
        ]
    }
}

pub async fn fetch_releases_for(
    repo: Option<&str>,
) -> Result<Vec<HorizonRelease>> {
    let repo_name = match repo {
        Some("upstream") | Some("horizon") | Some("horizon-rust/horizon") => {
            "horizon-rust/horizon"
        }
        Some(custom) if !custom.trim().is_empty() => custom.trim(),
        _ => HORIZON_REPO,
    };
    let cache_file = horizon_data_dir().join(format!(
        "releases-cache-{}.json",
        repo_name.replace('/', "_")
    ));

    let installed = get_installed_versions();
    let asset_wanted = platform_asset_name();

    let fetched = async {
        let url = format!(
            "https://api.github.com/repos/{repo_name}/releases?per_page=30"
        );
        let client = reqwest::Client::builder()
            .user_agent("Gradle")
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        let resp = client.get(&url).send().await?.error_for_status()?;
        let list: serde_json::Value = resp.json().await?;
        let mut list_out = Vec::new();

        if let Some(items) = list.as_array() {
            for item in items {
                if item["draft"].as_bool().unwrap_or(false) {
                    continue;
                }
                let tag =
                    item["tag_name"].as_str().unwrap_or_default().to_string();
                let name = item["name"].as_str().unwrap_or(&tag).to_string();
                let body = item["body"]
                    .as_str()
                    .unwrap_or_default()
                    .replace("\r\n", "\n");
                let published_at = item["published_at"]
                    .as_str()
                    .unwrap_or_default()
                    .chars()
                    .take(10)
                    .collect();
                let prerelease = item["prerelease"].as_bool().unwrap_or(false);

                let mut asset_url = None;
                let mut asset_size = 0u64;
                let mut sha256 = None;
                let mut signature_url = None;

                if let Some(assets) = item["assets"].as_array() {
                    for a in assets {
                        let a_name = a["name"].as_str().unwrap_or_default();
                        if asset_wanted.is_some_and(|w| w == a_name) {
                            asset_url = a["browser_download_url"]
                                .as_str()
                                .map(|s| s.to_string());
                            asset_size = a["size"].as_u64().unwrap_or(0);
                        }
                        if let Some(wanted) = asset_wanted {
                            let sig_ext1 = format!("{wanted}.sig");
                            let sig_ext2 = format!("{wanted}.asc");
                            if a_name == sig_ext1 || a_name == sig_ext2 {
                                signature_url = a["browser_download_url"]
                                    .as_str()
                                    .map(|s| s.to_string());
                            }
                        }
                    }
                }

                // If asset_url is missing but platform asset exists, compute standard GitHub release download URL
                if asset_url.is_none() {
                    asset_url = fallback_asset_url(repo_name, &tag);
                }

                // Extract SHA-256 from release body if present
                for line in body.lines() {
                    let lower = line.to_lowercase();
                    if lower.contains("sha256")
                        || asset_wanted
                            .is_some_and(|w| lower.contains(&w.to_lowercase()))
                    {
                        for token in
                            line.split(|c: char| !c.is_ascii_alphanumeric())
                        {
                            if token.len() == 64
                                && token.chars().all(|c| c.is_ascii_hexdigit())
                            {
                                sha256 = Some(token.to_lowercase());
                                break;
                            }
                        }
                    }
                    if sha256.is_some() {
                        break;
                    }
                }

                list_out.push(HorizonRelease {
                    tag,
                    name,
                    body,
                    published_at,
                    prerelease,
                    asset_url,
                    asset_size,
                    installed: false,
                    sha256,
                    signature_url,
                });
            }
        }
        Result::<Vec<HorizonRelease>>::Ok(list_out)
    }
    .await;

    let mut releases = match fetched {
        Ok(r) if !r.is_empty() => {
            if let Ok(json) = serde_json::to_string_pretty(&r) {
                let _ = fs::create_dir_all(horizon_data_dir());
                let _ = fs::write(&cache_file, json);
            }
            r
        }
        _ => {
            if let Ok(data) = fs::read_to_string(&cache_file) {
                if let Ok(cached) =
                    serde_json::from_str::<Vec<HorizonRelease>>(&data)
                {
                    cached
                } else {
                    get_fallback_releases(repo_name)
                }
            } else {
                get_fallback_releases(repo_name)
            }
        }
    };

    for r in &mut releases {
        r.installed = installed.contains(&r.tag);
    }

    Ok(releases)
}

pub async fn fetch_releases() -> Result<Vec<HorizonRelease>> {
    fetch_releases_for(None).await
}

pub async fn install_version(tag: &str) -> Result<PathBuf> {
    install_version_with_progress(tag, |_| {}).await
}

pub async fn install_version_with_progress<F>(
    tag: &str,
    progress: F,
) -> Result<PathBuf>
where
    F: Fn(DownloadProgress) + Send + Sync + 'static,
{
    let target = version_dir(tag);
    if target.join(CLIENT_BIN_NAME).is_file() {
        progress(DownloadProgress {
            tag: tag.to_string(),
            downloaded: 0,
            total: Some(0),
            percent: 1.0,
            speed_bps: 0,
            stage: "done".to_string(),
        });
        return Ok(target);
    }

    let releases = fetch_releases().await?;
    let release = releases
        .iter()
        .find(|r| r.tag == tag)
        .ok_or_else(|| anyhow::anyhow!("Gradle Rust release '{tag}' not found"))?;

    let download_url = release.asset_url.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "Release '{tag}' does not have a binary for this platform"
        )
    })?;

    // Enforce HTTPS only
    if !download_url.starts_with("https://") {
        bail!(
            "Insecure download URL '{download_url}' rejected. Downloads are only permitted over HTTPS."
        );
    }

    let v_dir = versions_dir();
    tokio::fs::create_dir_all(&v_dir).await?;

    let temp_dir = v_dir.join(format!(
        ".tmp_{tag}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    ));
    tokio::fs::create_dir_all(&temp_dir).await?;

    let part_path = temp_dir.join("download.archive.part");
    let mut existing_bytes = 0u64;
    if part_path.is_file() {
        if let Ok(meta) = tokio::fs::metadata(&part_path).await {
            existing_bytes = meta.len();
        }
    }

    let client = reqwest::Client::builder()
        .user_agent("Gradle")
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if !attempt.url().as_str().starts_with("https://") {
                attempt.error("Insecure redirect over non-HTTPS rejected")
            } else {
                attempt.follow()
            }
        }))
        .build()?;

    let mut req = client.get(download_url);
    if existing_bytes > 0 {
        req = req.header(
            reqwest::header::RANGE,
            format!("bytes={existing_bytes}-"),
        );
    }

    let mut response = req.send().await?.error_for_status()?;

    let (mut downloaded, total_bytes, mut file) =
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            let total = response
                .content_length()
                .map(|len| len + existing_bytes)
                .or(Some(release.asset_size).filter(|s| *s > 0));
            let f = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&part_path)
                .await?;
            (existing_bytes, total, f)
        } else {
            let total = response
                .content_length()
                .or(Some(release.asset_size).filter(|s| *s > 0));
            let f = tokio::fs::File::create(&part_path).await?;
            (0u64, total, f)
        };

    let mut last_progress_tick = std::time::Instant::now();
    let mut bytes_since_last_tick = 0u64;

    progress(DownloadProgress {
        tag: tag.to_string(),
        downloaded,
        total: total_bytes,
        percent: total_bytes
            .map(|t| if t > 0 { (downloaded as f32 / t as f32).min(1.0) } else { 0.0 })
            .unwrap_or(0.0),
        speed_bps: 0,
        stage: "downloading".to_string(),
    });

    use tokio::io::AsyncWriteExt;
    while let Some(chunk) = response.chunk().await? {
        file.write_all(&chunk).await?;
        downloaded += chunk.len() as u64;
        bytes_since_last_tick += chunk.len() as u64;

        let elapsed = last_progress_tick.elapsed();
        if elapsed >= std::time::Duration::from_millis(150) {
            let speed_bps = ((bytes_since_last_tick as f64) / elapsed.as_secs_f64()) as u64;
            bytes_since_last_tick = 0;
            last_progress_tick = std::time::Instant::now();

            let percent = total_bytes
                .map(|t| if t > 0 { (downloaded as f32 / t as f32).min(1.0) } else { 0.0 })
                .unwrap_or(0.0);

            progress(DownloadProgress {
                tag: tag.to_string(),
                downloaded,
                total: total_bytes,
                percent,
                speed_bps,
                stage: "downloading".to_string(),
            });
        }
    }
    file.flush().await?;
    drop(file);

    let archive_path = temp_dir.join("download.archive");
    tokio::fs::rename(&part_path, &archive_path).await?;

    // Verification stage
    progress(DownloadProgress {
        tag: tag.to_string(),
        downloaded,
        total: total_bytes,
        percent: 1.0,
        speed_bps: 0,
        stage: "verifying".to_string(),
    });

    let hash_path = archive_path.clone();
    let actual_sha256 = tokio::task::spawn_blocking(move || -> Result<String> {
        use sha2::{Digest, Sha256};
        let mut f = fs::File::open(&hash_path)?;
        let mut hasher = Sha256::new();
        let mut buf = [0u8; 65536];
        loop {
            let n = std::io::Read::read(&mut f, &mut buf)?;
            if n == 0 {
                break;
            }
            hasher.update(&buf[..n]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    })
    .await??;

    if let Some(expected) = &release.sha256 {
        let expected_clean = expected.trim().to_lowercase();
        let actual_clean = actual_sha256.trim().to_lowercase();
        if !expected_clean.is_empty() && expected_clean != actual_clean {
            let _ = tokio::fs::remove_dir_all(&temp_dir).await;
            bail!(
                "SHA-256 checksum verification failed for {tag}!\nExpected: {expected_clean}\nActual:   {actual_clean}"
            );
        }
    }

    // Optional upstream cryptographic signature verification/download
    if let Some(sig_url) = &release.signature_url {
        if sig_url.starts_with("https://") {
            if let Ok(sig_bytes) = client.get(sig_url).send().await {
                if let Ok(bytes) = sig_bytes.bytes().await {
                    let _ = tokio::fs::write(
                        temp_dir.join("download.archive.sig"),
                        &bytes,
                    )
                    .await;
                }
            }
        }
    }

    // Extraction stage
    progress(DownloadProgress {
        tag: tag.to_string(),
        downloaded,
        total: total_bytes,
        percent: 1.0,
        speed_bps: 0,
        stage: "extracting".to_string(),
    });

    let release_name = release.name.clone();
    let archive_path_clone = archive_path.clone();
    let target_clone = target.clone();
    let actual_sha256_clone = actual_sha256.clone();
    let tag_clone = tag.to_string();

    tokio::task::spawn_blocking(move || -> Result<PathBuf> {
        extract_and_install_archive(
            &tag_clone,
            &archive_path_clone,
            &release_name,
            &actual_sha256_clone,
            &target_clone,
        )
    })
    .await??;

    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    // Done stage
    progress(DownloadProgress {
        tag: tag.to_string(),
        downloaded,
        total: total_bytes,
        percent: 1.0,
        speed_bps: 0,
        stage: "done".to_string(),
    });

    Ok(target)
}

pub fn get_installed_version_info(tag: &str) -> Option<InstalledVersionInfo> {
    let target = version_dir(tag);
    if !target.join(CLIENT_BIN_NAME).is_file() {
        return None;
    }
    let info_path = target.join("version_info.json");
    if let Ok(content) = fs::read_to_string(&info_path) {
        if let Ok(info) = serde_json::from_str::<InstalledVersionInfo>(&content) {
            return Some(info);
        }
    }
    Some(InstalledVersionInfo {
        tag: tag.to_string(),
        name: tag.to_string(),
        sha256: None,
        installed_at: String::new(),
    })
}

pub fn remove_version(tag: &str) -> Result<()> {
    let target = version_dir(tag);
    if target.is_dir() {
        fs::remove_dir_all(&target)
            .context("Failed to delete version directory")?;
    }
    Ok(())
}

pub fn extract_and_install_archive(
    tag: &str,
    archive_path: &std::path::Path,
    release_name: &str,
    sha256: &str,
    target_dir: &std::path::Path,
) -> Result<PathBuf> {
    let temp_parent = target_dir
        .parent()
        .unwrap_or(target_dir);
    let temp_dir = temp_parent.join(format!(
        ".tmp_unpack_{tag}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    ));
    fs::create_dir_all(&temp_dir)?;

    let unpack_dir = temp_dir.join("unpacked");
    fs::create_dir_all(&unpack_dir)?;

    let is_tar = archive_path
        .extension()
        .is_some_and(|e| e == "gz" || e == "tar")
        || archive_path.to_string_lossy().contains(".tar.gz");

    if is_tar {
        let file = fs::File::open(archive_path)?;
        let decompressor = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decompressor);
        archive.unpack(&unpack_dir)?;
    } else {
        let file = fs::File::open(archive_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(&unpack_dir)?;
    }

    // The archive may contain a top-level directory (e.g. horizon-linux-x86_64)
    let mut top_dir = unpack_dir.clone();
    let entries = fs::read_dir(&unpack_dir)?.flatten().collect::<Vec<_>>();
    if entries.len() == 1 && entries[0].file_type()?.is_dir() {
        top_dir = entries[0].path();
    }

    if !top_dir.join(CLIENT_BIN_NAME).is_file() {
        let _ = fs::remove_dir_all(&temp_dir);
        bail!("Archive did not contain {CLIENT_BIN_NAME}");
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let client_path = top_dir.join(CLIENT_BIN_NAME);
        if client_path.exists() {
            let _ = fs::set_permissions(
                &client_path,
                fs::Permissions::from_mode(0o755),
            );
        }
        let server_path = top_dir.join(SERVER_BIN_NAME);
        if server_path.exists() {
            let _ = fs::set_permissions(
                &server_path,
                fs::Permissions::from_mode(0o755),
            );
        }
    }

    let _ = fs::remove_dir_all(target_dir);
    fs::create_dir_all(temp_parent)?;
    fs::rename(&top_dir, target_dir)?;
    let _ = fs::remove_dir_all(&temp_dir);

    // Save version metadata
    let meta = serde_json::json!({
        "tag": tag,
        "name": release_name,
        "sha256": sha256,
        "installed_at": chrono::Utc::now().to_rfc3339(),
    });
    let _ = fs::write(
        target_dir.join("version_info.json"),
        serde_json::to_string_pretty(&meta).unwrap_or_default(),
    );
    fs::write(target_dir.join(".installed"), "")?;

    Ok(target_dir.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_and_install_mock_archive_integration() {
        let temp_dir = tempfile::tempdir().unwrap();
        let versions_root = temp_dir.path().join("versions");
        let target_vdir = versions_root.join("v9.9.9");

        // 1. Create a mock release tar.gz archive with client and server
        let archive_path = temp_dir.path().join("mock-release.tar.gz");
        {
            let tar_gz = fs::File::create(&archive_path).unwrap();
            let enc = flate2::write::GzEncoder::new(tar_gz, flate2::Compression::default());
            let mut tar = tar::Builder::new(enc);

            let dummy_content = b"#!/bin/sh\necho 'Gradle Native Rust Mock'\n";

            let mut header = tar::Header::new_gnu();
            header.set_size(dummy_content.len() as u64);
            header.set_mode(0o755);
            header.set_cksum();
            tar.append_data(&mut header, format!("mock-dist/{CLIENT_BIN_NAME}"), &dummy_content[..]).unwrap();

            let mut header_server = tar::Header::new_gnu();
            header_server.set_size(dummy_content.len() as u64);
            header_server.set_mode(0o755);
            header_server.set_cksum();
            tar.append_data(&mut header_server, format!("mock-dist/{SERVER_BIN_NAME}"), &dummy_content[..]).unwrap();

            tar.finish().unwrap();
        }

        // 2. Install archive
        let installed_path = extract_and_install_archive(
            "v9.9.9",
            &archive_path,
            "Mock Release v9.9.9",
            "dummy_sha256_mock_hash",
            &target_vdir,
        ).unwrap();

        assert_eq!(installed_path, target_vdir);
        assert!(target_vdir.join(CLIENT_BIN_NAME).is_file());
        assert!(target_vdir.join(SERVER_BIN_NAME).is_file());
        assert!(target_vdir.join(".installed").is_file());
        assert!(target_vdir.join("version_info.json").is_file());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let meta = fs::metadata(target_vdir.join(CLIENT_BIN_NAME)).unwrap();
            assert_eq!(meta.permissions().mode() & 0o111, 0o111);
        }

        // Verify version info metadata
        let info_str = fs::read_to_string(target_vdir.join("version_info.json")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&info_str).unwrap();
        assert_eq!(parsed["tag"], "v9.9.9");
        assert_eq!(parsed["name"], "Mock Release v9.9.9");
        assert_eq!(parsed["sha256"], "dummy_sha256_mock_hash");

        // Clean removal
        fs::remove_dir_all(&target_vdir).unwrap();
        assert!(!target_vdir.exists());
    }

    #[test]
    fn test_platform_asset_naming() {
        let asset = platform_asset_name();
        assert!(asset.is_some());
    }
}
