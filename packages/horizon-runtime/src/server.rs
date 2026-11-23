//! Horizon dedicated server management (native Rust, zero JVM)
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, bail};
use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::OnceLock;
use zip::write::SimpleFileOptions;

use crate::execution::build_server_command;
use crate::preparation::prepared_resources_dir;
use crate::releases::{get_installed_versions, horizon_data_dir};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonServerConfig {
    pub name: String,
    pub tag: String,
    pub port: u16,
    pub motd: String,
    pub max_players: u32,
    pub auto_backup: bool,
    pub tailscale_enabled: bool,
}

impl Default for HorizonServerConfig {
    fn default() -> Self {
        Self {
            name: "Horizon Server".to_string(),
            tag: "latest".to_string(),
            port: 25565,
            motd: "A Native Rust Horizon Server on Gradle".to_string(),
            max_players: 20,
            auto_backup: true,
            tailscale_enabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonServerInfo {
    pub name: String,
    pub path: String,
    pub config: HorizonServerConfig,
    pub is_running: bool,
    pub pid: Option<u32>,
    pub backup_count: usize,
}

static RUNNING_SERVERS: OnceLock<Mutex<HashMap<String, Child>>> =
    OnceLock::new();

fn get_running_map() -> &'static Mutex<HashMap<String, Child>> {
    RUNNING_SERVERS.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn servers_dir() -> PathBuf {
    horizon_data_dir().join("servers")
}

pub fn server_dir(name: &str) -> PathBuf {
    let slug = sanitize_filename(name);
    servers_dir().join(slug)
}

fn sanitize_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if s.trim().is_empty() {
        "server".to_string()
    } else {
        s
    }
}

pub fn list_servers() -> Result<Vec<HorizonServerInfo>> {
    let root = servers_dir();
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut map = get_running_map().lock();
    let mut list = Vec::new();

    for entry in fs::read_dir(&root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let config_file = path.join("server.json");
        let config = if config_file.is_file() {
            let content = fs::read_to_string(&config_file)?;
            serde_json::from_str::<HorizonServerConfig>(&content)
                .unwrap_or_else(|_| HorizonServerConfig {
                    name: entry.file_name().to_string_lossy().to_string(),
                    ..Default::default()
                })
        } else {
            HorizonServerConfig {
                name: entry.file_name().to_string_lossy().to_string(),
                ..Default::default()
            }
        };

        // Check if process is still active
        let (is_running, pid) = if let Some(child) = map.get_mut(&config.name) {
            match child.try_wait() {
                Ok(None) => (true, Some(child.id())),
                _ => (false, None),
            }
        } else {
            (false, None)
        };

        if !is_running {
            map.remove(&config.name);
        }

        // Count backups
        let backups_dir = path.join("backups");
        let backup_count = if backups_dir.is_dir() {
            fs::read_dir(&backups_dir)
                .map(|rd| {
                    rd.filter_map(|e| e.ok())
                        .filter(|e| {
                            e.path().extension().is_some_and(|ext| ext == "zip")
                        })
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };

        list.push(HorizonServerInfo {
            name: config.name.clone(),
            path: path.to_string_lossy().to_string(),
            config,
            is_running,
            pid,
            backup_count,
        });
    }

    Ok(list)
}

pub fn create_server(
    name: &str,
    tag: Option<&str>,
    port: Option<u16>,
) -> Result<HorizonServerInfo> {
    let dir = server_dir(name);
    if dir.exists() {
        bail!("Server directory already exists: {}", dir.display());
    }

    fs::create_dir_all(&dir).context("creating server directory")?;
    let backups_dir = dir.join("backups");
    let _ = fs::create_dir_all(&backups_dir);

    let installed = get_installed_versions();
    let resolved_tag = if let Some(t) = tag {
        t.to_string()
    } else if let Some(first) = installed.first() {
        first.clone()
    } else {
        "latest".to_string()
    };

    let server_port = port.unwrap_or(25565);

    let config = HorizonServerConfig {
        name: name.to_string(),
        tag: resolved_tag,
        port: server_port,
        motd: format!("{name} - Native Rust Minecraft Server"),
        max_players: 20,
        auto_backup: true,
        tailscale_enabled: false,
    };

    // Save server.json
    let json = serde_json::to_string_pretty(&config)?;
    fs::write(dir.join("server.json"), json)?;

    // Write server.properties
    let props = format!(
        "server-port={server_port}\nmotd={motd}\nmax-players={max_players}\npvp=true\nonline-mode=false\ndifficulty=easy\n",
        motd = config.motd,
        max_players = config.max_players,
    );
    fs::write(dir.join("server.properties"), props)?;

    Ok(HorizonServerInfo {
        name: config.name.clone(),
        path: dir.to_string_lossy().to_string(),
        config,
        is_running: false,
        pid: None,
        backup_count: 0,
    })
}

pub fn start_server(name: &str) -> Result<u32> {
    let mut map = get_running_map().lock();

    if let Some(child) = map.get_mut(name)
        && let Ok(None) = child.try_wait() {
            bail!("Server '{name}' is already running with PID {}", child.id());
        }

    let dir = server_dir(name);
    if !dir.is_dir() {
        bail!("Server directory not found at {}", dir.display());
    }

    let config_file = dir.join("server.json");
    let config: HorizonServerConfig = if config_file.is_file() {
        serde_json::from_str(&fs::read_to_string(&config_file)?)?
    } else {
        HorizonServerConfig {
            name: name.to_string(),
            ..Default::default()
        }
    };

    // Determine installed version to execute
    let installed = get_installed_versions();
    let tag = if installed.contains(&config.tag) {
        config.tag.clone()
    } else if let Some(first) = installed.first() {
        first.clone()
    } else {
        bail!(
            "No Horizon version installed. Please install a Horizon release first."
        );
    };

    let resources = prepared_resources_dir();
    let mut cmd = build_server_command(&tag, &dir, &resources, &[])?;

    let child = cmd.spawn().context("spawning horizon-server process")?;
    let pid = child.id();
    map.insert(name.to_string(), child);

    Ok(pid)
}

pub fn stop_server(name: &str) -> Result<()> {
    let mut map = get_running_map().lock();

    if let Some(mut child) = map.remove(name) {
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    } else {
        bail!("Server '{name}' is not running");
    }
}

pub fn backup_server(name: &str) -> Result<PathBuf> {
    let dir = server_dir(name);
    if !dir.is_dir() {
        bail!("Server directory not found at {}", dir.display());
    }

    let backups_dir = dir.join("backups");
    fs::create_dir_all(&backups_dir)?;

    let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
    let backup_file = backups_dir.join(format!("backup_{timestamp}.zip"));

    let file = File::create(&backup_file)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();

        // Skip backups directory itself to prevent recursive backup
        if file_name == "backups" {
            continue;
        }

        if path.is_file() {
            zip.start_file(&file_name, options)?;
            let mut f = File::open(&path)?;
            std::io::copy(&mut f, &mut zip)?;
        } else if path.is_dir() {
            // World directory or similar
            for sub in walkdir_simple(&path)? {
                let rel = sub.strip_prefix(&dir).unwrap_or(&sub);
                let rel_str = rel.to_string_lossy().replace('\\', "/");
                if sub.is_file() {
                    zip.start_file(&rel_str, options)?;
                    let mut f = File::open(&sub)?;
                    std::io::copy(&mut f, &mut zip)?;
                }
            }
        }
    }

    zip.finish()?;
    Ok(backup_file)
}

fn walkdir_simple(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let p = entry.path();
            if p.is_dir() {
                files.extend(walkdir_simple(&p)?);
            } else {
                files.push(p);
            }
        }
    }
    Ok(files)
}

pub fn update_server(name: &str, new_tag: &str) -> Result<()> {
    let dir = server_dir(name);
    let config_file = dir.join("server.json");
    if !config_file.is_file() {
        bail!("Server config not found at {}", config_file.display());
    }

    let mut config: HorizonServerConfig =
        serde_json::from_str(&fs::read_to_string(&config_file)?)?;
    config.tag = new_tag.to_string();

    let json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_file, json)?;
    Ok(())
}

pub fn delete_server(name: &str) -> Result<()> {
    let _ = stop_server(name);
    let dir = server_dir(name);
    if dir.is_dir() {
        fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;
    use zip::ZipArchive;

    #[test]
    fn test_streaming_server_backup() {
        let test_name = format!("test_backup_srv_{}", Utc::now().timestamp_nanos_opt().unwrap_or(0));
        let srv_info = create_server(&test_name, Some("latest"), Some(25565)).unwrap();
        let srv_path = PathBuf::from(&srv_info.path);

        // Populate with files to back up: a top-level file and a nested world file
        let server_prop = srv_path.join("server.properties");
        fs::write(&server_prop, b"motd=Test Server\npvp=true\n").unwrap();

        let world_dir = srv_path.join("world/region");
        fs::create_dir_all(&world_dir).unwrap();
        let chunk_file = world_dir.join("r.0.0.mca");
        fs::write(&chunk_file, vec![0x42; 65536]).unwrap(); // 64 KB test chunk

        // Perform streaming backup
        let backup_zip_path = backup_server(&test_name).unwrap();
        assert!(backup_zip_path.is_file(), "Backup zip file was not created");

        // Verify zip integrity and streaming content copy
        let file = File::open(&backup_zip_path).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();

        let mut found_prop = false;
        let mut found_chunk = false;

        for i in 0..archive.len() {
            let mut entry = archive.by_index(i).unwrap();
            let name = entry.name().to_string();
            if name == "server.properties" {
                found_prop = true;
                let mut content = String::new();
                entry.read_to_string(&mut content).unwrap();
                assert!(content.contains("motd=Test Server"));
            } else if name == "world/region/r.0.0.mca" {
                found_chunk = true;
                let mut data = Vec::new();
                entry.read_to_end(&mut data).unwrap();
                assert_eq!(data.len(), 65536);
                assert_eq!(data[0], 0x42);
            }
        }

        assert!(found_prop, "server.properties not found in backup zip");
        assert!(found_chunk, "world chunk not found in backup zip");

        // Clean up
        let _ = delete_server(&test_name);
    }
}
