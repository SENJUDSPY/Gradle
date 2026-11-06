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

fn get_fallback_releases(repo: &str) -> Vec<HorizonRelease> {
    if repo.contains("horizon-rust") || repo == "upstream" {
        vec![
            HorizonRelease {
                tag: "launcher@0.2.42".to_string(),
                name: "Horizon launcher 0.2.42".to_string(),
                body: "### New\n- Launcher: \"A new version is available\" with Update; copies that cannot update themselves say so (#9311)\n- Wardrobe: \"Claim for free\" where Paddle doesn't sell (#9419)\n- Launcher: choose game architecture independently on Windows ARM (#9480)\n\n### Fixed\n- launcher: dynamic right padding for friend invite code input (#9518)\n- launcher: allow editing instance icon from home and preserve picture in dialog (#9436)\n- Native mods: final GUI overlay and current-frame camera (#9556)\n- Native mods: world primitives hook (lines, boxes, beams, text, see-through) (#9568)\n- Fix native build companions in the macOS game app (#9529)\n- Launcher MCP: reach a game started from its window with Agent Control on (#9595)\n- launcher: auto-detect Minecraft 26.3 installs from Dawn Client (#9563)\n- Friends: bound wrong short-code guesses per sharing session (#9626)".to_string(),
                published_at: "2026-11-04".to_string(),
                prerelease: false,
                asset_url: None,
                asset_size: 42_500_000,
                installed: false,
            },
            HorizonRelease {
                tag: "nightly-2026-11-05".to_string(),
                name: "Nightly 2026-11-05 · f319701".to_string(),
                body: "### Changes\n- wgpu Vulkan pipeline shader optimization and entity culling improvements\n- Distant terrain rendering LOD mesh generator up to 1024 chunks\n- Low latency audio mixing thread improvements\n- Redstone wire update propagation parity fixes\n- Server performance: chunk packet serialization throughput enhanced".to_string(),
                published_at: "2026-11-05".to_string(),
                prerelease: true,
                asset_url: None,
                asset_size: 44_100_000,
                installed: false,
            },
            HorizonRelease {
                tag: "v0.2.97".to_string(),
                name: "Horizon 0.2.97".to_string(),
                body: "### Highlights\n- Initial WebAssembly mod loading support (`.wasm`)\n- Native gamepad / controller rumble and triggers\n- Full vanilla world save format compatibility (Anvil 26.3)\n- LAN play and p2p friend invite system".to_string(),
                published_at: "2026-10-22".to_string(),
                prerelease: false,
                asset_url: None,
                asset_size: 41_800_000,
                installed: false,
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
                asset_url: None,
                asset_size: 43_200_000,
                installed: false,
            },
            HorizonRelease {
                tag: "nightly-2026-11-01".to_string(),
                name: "Nightly 2026-11-01 · 6f3d7b2".to_string(),
                body: "### Changes\n- Bundled Horizon client (commit 6f3d7b2) with offline skins and offline multiplayer\n- Fixes for compressed packet handling in `horizon-protocol`\n- Gamepad deadzone calibration and vibration support".to_string(),
                published_at: "2026-11-01".to_string(),
                prerelease: true,
                asset_url: None,
                asset_size: 45_000_000,
                installed: false,
            },
            HorizonRelease {
                tag: "v0.2.97.1".to_string(),
                name: "Horizon v0.2.97.1 (offline build)".to_string(),
                body: "### Notes\n- Offline game client release based on upstream Horizon 0.2.97\n- Standalone dedicated server without JVM requirements\n- Support for custom resource packs and shaders".to_string(),
                published_at: "2026-10-24".to_string(),
                prerelease: false,
                asset_url: None,
                asset_size: 42_900_000,
                installed: false,
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

                if let Some(assets) = item["assets"].as_array() {
                    for a in assets {
                        let a_name = a["name"].as_str().unwrap_or_default();
                        if asset_wanted.is_some_and(|w| w == a_name) {
                            asset_url = a["browser_download_url"]
                                .as_str()
                                .map(|s| s.to_string());
                            asset_size = a["size"].as_u64().unwrap_or(0);
                            break;
                        }
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
    let target = version_dir(tag);
    if target.join(CLIENT_BIN_NAME).is_file() {
        return Ok(target);
    }

    let releases = fetch_releases().await?;
    let release = releases
        .iter()
        .find(|r| r.tag == tag)
        .ok_or_else(|| anyhow::anyhow!("Horizon release '{tag}' not found"))?;

    let download_url = release.asset_url.as_deref().ok_or_else(|| {
        anyhow::anyhow!(
            "Release '{tag}' does not have a binary for this platform"
        )
    })?;

    let v_dir = versions_dir();
    tokio::fs::create_dir_all(&v_dir).await?;

    let temp_dir = v_dir.join(format!(
        ".tmp_{tag}_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis()
    ));
    tokio::fs::create_dir_all(&temp_dir).await?;

    let archive_path = temp_dir.join("download.archive");

    let client = reqwest::Client::builder()
        .user_agent("Gradle")
        .build()?;
    let bytes = client
        .get(download_url)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    tokio::fs::write(&archive_path, &bytes).await?;

    let unpack_dir = temp_dir.join("unpacked");
    tokio::fs::create_dir_all(&unpack_dir).await?;

    let archive_path_clone = archive_path.clone();
    let unpack_dir_clone = unpack_dir.clone();

    tokio::task::spawn_blocking(move || -> Result<()> {
        let is_tar = archive_path_clone
            .extension()
            .is_some_and(|e| e == "gz" || e == "tar")
            || archive_path_clone.to_string_lossy().contains(".tar.gz");

        if is_tar {
            let file = fs::File::open(&archive_path_clone)?;
            let decompressor = flate2::read::GzDecoder::new(file);
            let mut archive = tar::Archive::new(decompressor);
            archive.unpack(&unpack_dir_clone)?;
        } else {
            let file = fs::File::open(&archive_path_clone)?;
            let mut archive = zip::ZipArchive::new(file)?;
            archive.extract(&unpack_dir_clone)?;
        }
        Ok(())
    })
    .await??;

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

    let _ = fs::remove_dir_all(&target);
    fs::rename(&top_dir, &target)?;
    let _ = fs::remove_dir_all(&temp_dir);
    fs::write(target.join(".installed"), "")?;

    Ok(target)
}

pub fn remove_version(tag: &str) -> Result<()> {
    let target = version_dir(tag);
    if target.is_dir() {
        fs::remove_dir_all(&target)
            .context("Failed to delete version directory")?;
    }
    Ok(())
}
