//! Settings configuration and Minecraft options.txt synchronizer for Gradle Rust Edition (Horizon)
//!
//! Maps Gradle renderer options, graphics backend, resource packs, and keybinds to
//! instance-level horizon-settings.json and standard options.txt.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

fn default_msaa() -> u32 {
	1
}

fn default_shadow_quality() -> String {
	"medium".to_string()
}

fn default_texture_filtering() -> String {
	"trilinear".to_string()
}

fn default_resource_packs() -> Vec<String> {
	vec!["vanilla".to_string()]
}

fn default_keybinds() -> HashMap<String, String> {
	let mut map = HashMap::new();
	map.insert("key.forward".to_string(), "key.keyboard.w".to_string());
	map.insert("key.left".to_string(), "key.keyboard.a".to_string());
	map.insert("key.back".to_string(), "key.keyboard.s".to_string());
	map.insert("key.right".to_string(), "key.keyboard.d".to_string());
	map.insert("key.jump".to_string(), "key.keyboard.space".to_string());
	map.insert("key.sneak".to_string(), "key.keyboard.left.shift".to_string());
	map.insert("key.sprint".to_string(), "key.keyboard.left.control".to_string());
	map.insert("key.attack".to_string(), "key.mouse.left".to_string());
	map.insert("key.use".to_string(), "key.mouse.right".to_string());
	map.insert("key.zoom".to_string(), "key.keyboard.c".to_string());
	map
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HorizonSettings {
	// Graphics & Renderer options
	pub graphics_preset: String, // "performance", "balanced", "quality", "ultra"
	pub render_distance: u32,    // 2 to 64 (default 12)
	pub lod_distance: u32,       // 32 to 1024 (default 128)
	pub cave_culling: bool,      // default true
	pub entity_culling: bool,    // default true
	pub vsync: bool,             // default true
	pub max_fps: u32,            // 0 = unlimited, or 60, 120, 144, 240
	pub fov: u32,                // 30 to 120 (default 70)
	pub graphics_api: String,    // "vulkan", "opengl", "dx12"
	#[serde(default = "default_msaa")]
	pub msaa_samples: u32,       // 1, 2, 4, 8
	#[serde(default = "default_shadow_quality")]
	pub shadow_quality: String,  // "off", "low", "medium", "high"
	#[serde(default = "default_texture_filtering")]
	pub texture_filtering: String, // "nearest", "bilinear", "trilinear", "anisotropic"
	#[serde(default)]
	pub fullscreen: bool,

	// Controller
	pub controller_enabled: bool,
	pub vibration: bool,
	pub adaptive_triggers: bool,
	pub lightbar: String, // "health", "dimension", "off"
	pub look_sensitivity: f32, // 0.1 to 2.0 (default 1.0)
	pub invert_look_y: bool,
	pub move_deadzone: f32, // 0.05 to 0.5 (default 0.15)
	pub look_deadzone: f32, // 0.05 to 0.5 (default 0.15)
	pub button_glyphs: String, // "auto", "xbox", "playstation", "nintendo"
	pub menu_navigation: String, // "focus", "pointer"

	// Extras
	pub zoom_enabled: bool,
	pub minimap_enabled: bool,
	pub performance_overlay: String, // "none", "fps", "fps_and_graph"
	pub battery_mode: bool,

	// AI Agents / MCP
	#[serde(default)]
	pub allow_ai_agents: bool,

	// Resource Packs
	#[serde(default = "default_resource_packs")]
	pub resource_packs: Vec<String>,

	// Keybinds (action name -> key code string)
	#[serde(default = "default_keybinds")]
	pub keybinds: HashMap<String, String>,
}

impl Default for HorizonSettings {
	fn default() -> Self {
		Self {
			graphics_preset: "balanced".to_string(),
			render_distance: 12,
			lod_distance: 128,
			cave_culling: true,
			entity_culling: true,
			vsync: true,
			max_fps: 120,
			fov: 70,
			graphics_api: "vulkan".to_string(),
			msaa_samples: 1,
			shadow_quality: "medium".to_string(),
			texture_filtering: "trilinear".to_string(),
			fullscreen: false,

			controller_enabled: true,
			vibration: true,
			adaptive_triggers: true,
			lightbar: "health".to_string(),
			look_sensitivity: 1.0,
			invert_look_y: false,
			move_deadzone: 0.15,
			look_deadzone: 0.15,
			button_glyphs: "auto".to_string(),
			menu_navigation: "focus".to_string(),

			zoom_enabled: true,
			minimap_enabled: false,
			performance_overlay: "none".to_string(),
			battery_mode: false,

			allow_ai_agents: false,
			resource_packs: default_resource_packs(),
			keybinds: default_keybinds(),
		}
	}
}

pub fn global_settings_path() -> PathBuf {
	dirs::config_dir()
		.unwrap_or_else(|| PathBuf::from("."))
		.join("horizon")
		.join("settings.json")
}

pub fn load_horizon_settings(instance_dir: Option<&Path>) -> HorizonSettings {
	// 1. Try instance-specific settings first
	if let Some(dir) = instance_dir {
		let instance_settings = dir.join("horizon-settings.json");
		if instance_settings.is_file()
			&& let Ok(content) = fs::read_to_string(&instance_settings)
				&& let Ok(mut settings) =
					serde_json::from_str::<HorizonSettings>(&content)
				{
					let _ = sync_from_options_txt(dir, &mut settings);
					return settings;
				}
	}

	// 2. Try global settings
	let global_path = global_settings_path();
	if global_path.is_file()
		&& let Ok(content) = fs::read_to_string(&global_path)
			&& let Ok(settings) =
				serde_json::from_str::<HorizonSettings>(&content)
			{
				return settings;
			}

	// 3. Fallback to default
	HorizonSettings::default()
}

pub fn save_horizon_settings(
	instance_dir: Option<&Path>,
	settings: &HorizonSettings,
) -> Result<()> {
	let serialized = serde_json::to_string_pretty(settings)?;

	// 1. Save global settings
	let global_path = global_settings_path();
	if let Some(parent) = global_path.parent() {
		fs::create_dir_all(parent)?;
	}
	fs::write(&global_path, &serialized)?;

	// 2. Save instance-specific settings if provided
	if let Some(dir) = instance_dir
		&& dir.exists() {
			fs::write(dir.join("horizon-settings.json"), &serialized)?;
			sync_to_options_txt(dir, settings)?;
		}

	Ok(())
}

/// Syncs settings into standard options.txt
pub fn sync_to_options_txt(dir: &Path, settings: &HorizonSettings) -> Result<()> {
	let options_path = dir.join("options.txt");
	let mut lines = if options_path.is_file() {
		fs::read_to_string(&options_path)?
			.lines()
			.map(String::from)
			.collect::<Vec<_>>()
	} else {
		Vec::new()
	};

	let packs_json = serde_json::to_string(&settings.resource_packs)
		.unwrap_or_else(|_| "[\"vanilla\"]".to_string());

	let updates = [
		("renderDistance", settings.render_distance.to_string()),
		("enableVsync", settings.vsync.to_string()),
		("maxFps", settings.max_fps.to_string()),
		("fov", settings.fov.to_string()),
		("fullscreen", settings.fullscreen.to_string()),
		("graphicsApi", settings.graphics_api.clone()),
		("msaa", settings.msaa_samples.to_string()),
		("shadows", settings.shadow_quality.clone()),
		("textureFiltering", settings.texture_filtering.clone()),
		("resourcePacks", packs_json),
	];

	for (key, val) in updates {
		let prefix = format!("{key}:");
		if let Some(line) = lines.iter_mut().find(|l| l.starts_with(&prefix)) {
			*line = format!("{key}:{val}");
		} else {
			lines.push(format!("{key}:{val}"));
		}
	}

	// Sync keybinds: key_{action}:{binding}
	for (action, binding) in &settings.keybinds {
		let prefix = format!("key_{action}:");
		let full_entry = format!("key_{action}:{binding}");
		if let Some(line) = lines.iter_mut().find(|l| l.starts_with(&prefix)) {
			*line = full_entry;
		} else {
			lines.push(full_entry);
		}
	}

	fs::write(options_path, lines.join("\n") + "\n")?;
	Ok(())
}

/// Reads options.txt and updates settings
pub fn sync_from_options_txt(dir: &Path, settings: &mut HorizonSettings) -> Result<()> {
	let options_path = dir.join("options.txt");
	if !options_path.is_file() {
		return Ok(());
	}

	let content = fs::read_to_string(&options_path)?;
	for line in content.lines() {
		let line = line.trim();
		if line.is_empty() || line.starts_with('#') {
			continue;
		}

		if let Some((key, val)) = line.split_once(':') {
			let key = key.trim();
			let val = val.trim();
			match key {
				"renderDistance" => {
					if let Ok(v) = val.parse::<u32>() {
						settings.render_distance = v.clamp(2, 64);
					}
				}
				"enableVsync" => {
					if let Ok(v) = val.parse::<bool>() {
						settings.vsync = v;
					}
				}
				"maxFps" => {
					if let Ok(v) = val.parse::<u32>() {
						settings.max_fps = v;
					}
				}
				"fov" => {
					if let Ok(v) = val.parse::<u32>() {
						settings.fov = v.clamp(30, 120);
					}
				}
				"fullscreen" => {
					if let Ok(v) = val.parse::<bool>() {
						settings.fullscreen = v;
					}
				}
				"graphicsApi"
					if !val.is_empty() => {
						settings.graphics_api = val.to_string();
					}
				"msaa" => {
					if let Ok(v) = val.parse::<u32>() {
						settings.msaa_samples = v;
					}
				}
				"shadows"
					if !val.is_empty() => {
						settings.shadow_quality = val.to_string();
					}
				"textureFiltering"
					if !val.is_empty() => {
						settings.texture_filtering = val.to_string();
					}
				"resourcePacks" => {
					if let Ok(packs) = serde_json::from_str::<Vec<String>>(val) {
						settings.resource_packs = packs;
					}
				}
				k if k.starts_with("key_") => {
					let action = &k[4..];
					settings.keybinds.insert(action.to_string(), val.to_string());
				}
				_ => {}
			}
		}
	}

	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	use tempfile::tempdir;

	#[test]
	fn test_default_horizon_settings() {
		let defaults = HorizonSettings::default();
		assert_eq!(defaults.render_distance, 12);
		assert_eq!(defaults.lod_distance, 128);
		assert_eq!(defaults.graphics_preset, "balanced");
		assert_eq!(defaults.msaa_samples, 1);
		assert_eq!(defaults.shadow_quality, "medium");
		assert_eq!(defaults.texture_filtering, "trilinear");
		assert!(defaults.controller_enabled);
		assert!(defaults.zoom_enabled);
		assert_eq!(defaults.resource_packs, vec!["vanilla".to_string()]);
		assert_eq!(defaults.keybinds.get("key.forward").unwrap(), "key.keyboard.w");
	}

	#[test]
	fn test_save_and_load_instance_settings() {
		let dir = tempdir().unwrap();
		let mut settings = HorizonSettings::default();
		settings.render_distance = 32;
		settings.lod_distance = 512;
		settings.minimap_enabled = true;
		settings.resource_packs.push("my_texture_pack.zip".to_string());
		settings.keybinds.insert("key.jump".to_string(), "key.keyboard.space".to_string());

		save_horizon_settings(Some(dir.path()), &settings).unwrap();

		let loaded = load_horizon_settings(Some(dir.path()));
		assert_eq!(loaded.render_distance, 32);
		assert_eq!(loaded.lod_distance, 512);
		assert!(loaded.minimap_enabled);
		assert!(loaded.resource_packs.contains(&"my_texture_pack.zip".to_string()));

		// Verify options.txt updated
		let opts = fs::read_to_string(dir.path().join("options.txt")).unwrap();
		assert!(opts.contains("renderDistance:32"));
		assert!(opts.contains("my_texture_pack.zip"));
		assert!(opts.contains("key_key.jump:key.keyboard.space"));
	}

	#[test]
	fn test_sync_from_options_txt() {
		let dir = tempdir().unwrap();
		let options_file = dir.path().join("options.txt");
		fs::write(
			&options_file,
			"renderDistance:24\nmaxFps:240\nenableVsync:false\nkey_key.forward:key.keyboard.up\n",
		)
		.unwrap();

		let mut settings = HorizonSettings::default();
		sync_from_options_txt(dir.path(), &mut settings).unwrap();

		assert_eq!(settings.render_distance, 24);
		assert_eq!(settings.max_fps, 240);
		assert!(!settings.vsync);
		assert_eq!(settings.keybinds.get("key.forward").unwrap(), "key.keyboard.up");
	}
}
