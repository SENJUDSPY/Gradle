use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HorizonSettings {
    // Graphics
    pub graphics_preset: String, // "performance", "balanced", "quality", "ultra"
    pub render_distance: u32,    // 2 to 64 (default 12)
    pub lod_distance: u32,       // 32 to 1024 (default 128)
    pub cave_culling: bool,      // default true
    pub entity_culling: bool,    // default true
    pub vsync: bool,             // default true
    pub max_fps: u32,            // 0 = unlimited, or 60, 120, 144, 240
    pub fov: u32,                // 30 to 120 (default 70)
    pub graphics_api: String,    // "vulkan", "opengl", "dx12"

    // Controller
    pub controller_enabled: bool,
    pub vibration: bool,
    pub adaptive_triggers: bool,
    pub lightbar: String,        // "health", "dimension", "off"
    pub look_sensitivity: f32,   // 0.1 to 2.0 (default 1.0)
    pub invert_look_y: bool,
    pub move_deadzone: f32,      // 0.05 to 0.5 (default 0.15)
    pub look_deadzone: f32,      // 0.05 to 0.5 (default 0.15)
    pub button_glyphs: String,   // "auto", "xbox", "playstation", "nintendo"
    pub menu_navigation: String, // "focus", "pointer"

    // Extras
    pub zoom_enabled: bool,
    pub minimap_enabled: bool,
    pub performance_overlay: String, // "none", "fps", "fps_and_graph"
    pub battery_mode: bool,
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
        if instance_settings.is_file() {
            if let Ok(content) = fs::read_to_string(&instance_settings) {
                if let Ok(settings) = serde_json::from_str::<HorizonSettings>(&content) {
                    return settings;
                }
            }
        }
    }

    // 2. Try global settings
    let global_path = global_settings_path();
    if global_path.is_file() {
        if let Ok(content) = fs::read_to_string(&global_path) {
            if let Ok(settings) = serde_json::from_str::<HorizonSettings>(&content) {
                return settings;
            }
        }
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
    if let Some(dir) = instance_dir {
        if dir.exists() {
            fs::write(dir.join("horizon-settings.json"), &serialized)?;
            sync_to_options_txt(dir, settings)?;
        }
    }

    Ok(())
}

fn sync_to_options_txt(dir: &Path, settings: &HorizonSettings) -> Result<()> {
    let options_path = dir.join("options.txt");
    let mut lines = if options_path.is_file() {
        fs::read_to_string(&options_path)?
            .lines()
            .map(String::from)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    let updates = [
        ("renderDistance", settings.render_distance.to_string()),
        ("enableVsync", settings.vsync.to_string()),
        ("maxFps", settings.max_fps.to_string()),
        ("fov", settings.fov.to_string()),
    ];

    for (key, val) in updates {
        let prefix = format!("{key}:");
        if let Some(line) = lines.iter_mut().find(|l| l.starts_with(&prefix)) {
            *line = format!("{key}:{val}");
        } else {
            lines.push(format!("{key}:{val}"));
        }
    }

    fs::write(options_path, lines.join("\n") + "\n")?;
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
        assert!(defaults.controller_enabled);
        assert!(defaults.zoom_enabled);
    }

    #[test]
    fn test_save_and_load_instance_settings() {
        let dir = tempdir().unwrap();
        let mut settings = HorizonSettings::default();
        settings.render_distance = 32;
        settings.lod_distance = 512;
        settings.minimap_enabled = true;

        save_horizon_settings(Some(dir.path()), &settings).unwrap();

        let loaded = load_horizon_settings(Some(dir.path()));
        assert_eq!(loaded.render_distance, 32);
        assert_eq!(loaded.lod_distance, 512);
        assert!(loaded.minimap_enabled);

        // Verify options.txt updated
        let opts = fs::read_to_string(dir.path().join("options.txt")).unwrap();
        assert!(opts.contains("renderDistance:32"));
    }
}
