//! Gradle Mod Manifest specification and capability permission model.
//!
//! Handles `gradle-mod.toml` (and `horizon-mod.toml`) configuration files
//! defining mod metadata, capability-based permissions, resource limits,
//! and dependencies.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Capability-based permissions available to WebAssembly mods.
///
/// Under the Gradle Mod Security Model, WASM modules execute in a sandboxed
/// environment. Calling any host function requiring a capability that has not
/// been explicitly declared and approved results in immediate module validation rejection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModCapability {
    /// Allows registering HUD and 2D overlay rendering hooks.
    Rendering,
    /// Allows inspecting world state (block IDs, biomes, light levels).
    WorldRead,
    /// Allows modifying the world (setting blocks, spawning particles, registering blocks/items/recipes).
    WorldEdit,
    /// Allows registering chat and console commands and interacting with command feedback.
    Commands,
    /// Allows outbound HTTP requests to user-whitelisted endpoints.
    Network,
    /// Allows reading isolated configuration files in the mod's designated data folder.
    FilesystemRead,
    /// Allows writing isolated configuration and save files in the mod's designated data folder.
    FilesystemWrite,
    /// Allows playing custom audio effects and sound cues.
    Audio,
    /// Allows intercepting player lifecycle events (login, death, respawn, inventory changes).
    PlayerEvents,
}

impl ModCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rendering => "rendering",
            Self::WorldRead => "world_read",
            Self::WorldEdit => "world_edit",
            Self::Commands => "commands",
            Self::Network => "network",
            Self::FilesystemRead => "filesystem_read",
            Self::FilesystemWrite => "filesystem_write",
            Self::Audio => "audio",
            Self::PlayerEvents => "player_events",
        }
    }

    pub fn from_str_id(s: &str) -> Option<Self> {
        match s.trim().to_lowercase().as_str() {
            "rendering" => Some(Self::Rendering),
            "world_read" | "worldread" => Some(Self::WorldRead),
            "world_edit" | "worldedit" => Some(Self::WorldEdit),
            "commands" | "command" => Some(Self::Commands),
            "network" | "net" => Some(Self::Network),
            "filesystem_read" | "fs_read" => Some(Self::FilesystemRead),
            "filesystem_write" | "fs_write" => Some(Self::FilesystemWrite),
            "audio" | "sound" => Some(Self::Audio),
            "player_events" | "playerevents" => Some(Self::PlayerEvents),
            _ => None,
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Rendering => "Render HUD overlays and 2D elements",
            Self::WorldRead => "Query world blocks, entities, and biomes",
            Self::WorldEdit => "Modify blocks and register blocks, items, or recipes",
            Self::Commands => "Register and execute in-game chat and console commands",
            Self::Network => "Perform external HTTP network requests",
            Self::FilesystemRead => "Read local mod configuration and state files",
            Self::FilesystemWrite => "Write local mod configuration and state files",
            Self::Audio => "Play in-game sound effects and ambient audio",
            Self::PlayerEvents => "Listen to player join, interaction, and movement events",
        }
    }
}

/// Resource quotas and limits enforced on mod execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum linear memory allocation in Megabytes (default: 32 MB, hard max: 512 MB).
    #[serde(default = "default_max_memory_mb")]
    pub max_memory_mb: u32,

    /// Maximum fuel (instruction counter) allowed per tick/call to prevent infinite loops.
    /// Default: 10,000,000 units.
    #[serde(default = "default_max_fuel")]
    pub max_fuel: u64,
}

fn default_max_memory_mb() -> u32 {
    32
}

fn default_max_fuel() -> u64 {
    10_000_000
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_mb: default_max_memory_mb(),
            max_fuel: default_max_fuel(),
        }
    }
}

/// Mod capabilities and permission configuration section.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModCapabilitiesConfig {
    /// List of declared capabilities required by this mod.
    #[serde(default)]
    pub permissions: Vec<ModCapability>,

    /// Resource limits enforced by the WASM host engine.
    #[serde(default)]
    pub limits: ResourceLimits,
}

/// Core mod metadata block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModInfo {
    /// Unique lowercase identifier matching `^[a-z0-9_-]{2,64}$`.
    pub id: String,

    /// Human-friendly display name.
    pub name: String,

    /// Semantic version (e.g. `1.0.0`).
    pub version: String,

    /// Authors or contributors.
    #[serde(default)]
    pub authors: Vec<String>,

    /// Short summary of the mod's features.
    #[serde(default)]
    pub description: String,

    /// SPDX license identifier (e.g. `MIT`, `LGPL-3.0-only`, `ARR`).
    #[serde(default = "default_license")]
    pub license: String,

    /// Entrypoint export name in the WebAssembly module (default: `gradle_init`).
    #[serde(default = "default_entrypoint")]
    pub entrypoint: String,

    /// Optional project homepage or repository URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub website: Option<String>,
}

fn default_license() -> String {
    "All-Rights-Reserved".to_string()
}

fn default_entrypoint() -> String {
    "gradle_init".to_string()
}

/// Parsed `gradle-mod.toml` (or `horizon-mod.toml`) root structure.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GradleModManifest {
    /// Mod identification metadata.
    #[serde(rename = "mod")]
    pub info: ModInfo,

    /// Capability permissions and resource limits.
    #[serde(default)]
    pub capabilities: ModCapabilitiesConfig,

    /// Mod and engine dependencies (e.g. `horizon = ">=26.3"`).
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
}

impl GradleModManifest {
    /// Parses a manifest from a TOML string.
    pub fn parse_str(toml_str: &str) -> Result<Self> {
        let manifest: Self = toml::from_str(toml_str)
            .context("Failed to parse Gradle mod manifest TOML")?;
        manifest.validate()?;
        Ok(manifest)
    }

    /// Loads and parses a manifest from a file path.
    pub fn load_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read mod manifest at {}", path.display()))?;
        Self::parse_str(&content)
    }

    /// Validates fields and bounds according to the Gradle Mod specification.
    pub fn validate(&self) -> Result<()> {
        let id = &self.info.id;
        if id.is_empty() || id.len() > 64 {
            bail!("Mod ID must be between 1 and 64 characters long: '{}'", id);
        }
        if !id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            bail!("Mod ID '{}' contains invalid characters. Only lowercase a-z, 0-9, _, - are permitted.", id);
        }

        if self.info.name.trim().is_empty() {
            bail!("Mod name cannot be empty");
        }

        if self.capabilities.limits.max_memory_mb == 0 || self.capabilities.limits.max_memory_mb > 512 {
            bail!(
                "Configured max_memory_mb ({}) exceeds permitted range (1 MB - 512 MB)",
                self.capabilities.limits.max_memory_mb
            );
        }

        if self.capabilities.limits.max_fuel == 0 || self.capabilities.limits.max_fuel > 1_000_000_000 {
            bail!(
                "Configured max_fuel ({}) exceeds permitted range (1 - 1,000,000,000)",
                self.capabilities.limits.max_fuel
            );
        }

        Ok(())
    }

    /// Checks whether this mod has requested a specific capability.
    pub fn has_capability(&self, cap: ModCapability) -> bool {
        self.capabilities.permissions.contains(&cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_manifest() {
        let toml_str = r#"
[mod]
id = "minimap_pro"
name = "Minimap Pro"
version = "1.2.0"
authors = ["Gradle Team"]
description = "Fast real-time HUD minimap"
license = "MIT"
entrypoint = "wasm_main"

[capabilities]
permissions = ["rendering", "world_read"]

[capabilities.limits]
max_memory_mb = 64
max_fuel = 20000000

[dependencies]
horizon = ">=26.3"
"#;

        let manifest = GradleModManifest::parse_str(toml_str).unwrap();
        assert_eq!(manifest.info.id, "minimap_pro");
        assert_eq!(manifest.info.name, "Minimap Pro");
        assert_eq!(manifest.info.version, "1.2.0");
        assert_eq!(manifest.info.entrypoint, "wasm_main");
        assert!(manifest.has_capability(ModCapability::Rendering));
        assert!(manifest.has_capability(ModCapability::WorldRead));
        assert!(!manifest.has_capability(ModCapability::WorldEdit));
        assert_eq!(manifest.capabilities.limits.max_memory_mb, 64);
        assert_eq!(manifest.capabilities.limits.max_fuel, 20_000_000);
    }

    #[test]
    fn test_reject_invalid_id() {
        let toml_str = r#"
[mod]
id = "Invalid Mod ID!"
name = "Bad ID"
version = "1.0.0"
"#;
        assert!(GradleModManifest::parse_str(toml_str).is_err());
    }

    #[test]
    fn test_reject_excessive_memory() {
        let toml_str = r#"
[mod]
id = "hungry_mod"
name = "Memory Hog"
version = "1.0.0"

[capabilities.limits]
max_memory_mb = 1024
"#;
        assert!(GradleModManifest::parse_str(toml_str).is_err());
    }
}
