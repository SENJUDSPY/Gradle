//! Stable Gradle Mod API & Host ABI Specification (v1).
//!
//! Defines the low-level Foreign Function Interface (FFI) contracts between
//! the Horizon Rust engine and sandboxed WebAssembly mod modules.
//!
//! Functions are categorized by capability: calling a host function without the
//! required capability declared in `gradle-mod.toml` causes validation or execution termination.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use super::manifest::ModCapability;
use serde::{Deserialize, Serialize};

/// Numeric identifiers for game lifecycle and interaction events.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GradleEventKind {
    /// Fired once when the mod is loaded and the world state is initialized.
    Init = 1,
    /// Fired every game tick (nominally 20 times per second).
    Tick = 2,
    /// Fired when a block is broken by a player or entity.
    BlockBreak = 3,
    /// Fired when a block is placed into the world.
    BlockPlace = 4,
    /// Fired when a player interacts with an entity (right-click / mount / trade).
    EntityInteract = 5,
    /// Fired when an incoming or outgoing chat message is processed.
    Chat = 6,
    /// Fired every frame during the 2D HUD rendering pass.
    RenderHud = 7,
}

impl GradleEventKind {
    pub fn from_u32(val: u32) -> Option<Self> {
        match val {
            1 => Some(Self::Init),
            2 => Some(Self::Tick),
            3 => Some(Self::BlockBreak),
            4 => Some(Self::BlockPlace),
            5 => Some(Self::EntityInteract),
            6 => Some(Self::Chat),
            7 => Some(Self::RenderHud),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Tick => "tick",
            Self::BlockBreak => "block_break",
            Self::BlockPlace => "block_place",
            Self::EntityInteract => "entity_interact",
            Self::Chat => "chat",
            Self::RenderHud => "render_hud",
        }
    }
}

/// Block registration definition passed from WASM to host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockDescriptor {
    pub identifier: String,
    pub name: String,
    pub hardness: f32,
    pub blast_resistance: f32,
    pub light_emission: u8,
    pub opaque: bool,
    pub drop_item_id: Option<String>,
}

/// Item registration definition passed from WASM to host.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemDescriptor {
    pub identifier: String,
    pub name: String,
    pub max_stack_size: u32,
    pub durability: Option<u32>,
    pub food_points: Option<u32>,
    pub saturation: Option<f32>,
}

/// Recipe type supported by the crafting engine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecipeKind {
    CraftingShaped,
    CraftingShapeless,
    Smelting,
    Blasting,
    Smoking,
}

/// Recipe registration definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RecipeDescriptor {
    pub identifier: String,
    pub kind: RecipeKind,
    pub ingredients: Vec<String>,
    pub result_item_id: String,
    pub result_count: u32,
    pub experience: Option<f32>,
    pub cook_time_ticks: Option<u32>,
}

/// In-game command registration descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub name: String,
    pub description: String,
    pub usage: String,
    pub permission_level: u8,
}

/// HUD Rendering context supplied by the host during `RenderHud` frame callback.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderHudContext {
    pub screen_width: f32,
    pub screen_height: f32,
    pub scale_factor: f32,
    pub delta_seconds: f32,
}

/// Specification of a host function exposed by the Horizon engine to WASM mods.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostFunctionSpec {
    pub module: &'static str,
    pub name: &'static str,
    pub required_capability: Option<ModCapability>,
    pub description: &'static str,
}

/// Canonical table of all official Gradle Mod API host functions.
pub static OFFICIAL_HOST_FUNCTIONS: &[HostFunctionSpec] = &[
    // --- Diagnostics & Logging (Safe: No capability required) ---
    HostFunctionSpec {
        module: "gradle",
        name: "log_info",
        required_capability: None,
        description: "Log informational message to engine console",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "log_warn",
        required_capability: None,
        description: "Log warning message to engine console",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "log_error",
        required_capability: None,
        description: "Log error message to engine console",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "register_event",
        required_capability: None,
        description: "Register a callback function for a GradleEventKind",
    },

    // --- World Reading (Requires WorldRead capability) ---
    HostFunctionSpec {
        module: "gradle",
        name: "get_block",
        required_capability: Some(ModCapability::WorldRead),
        description: "Query block identifier and metadata at (x, y, z)",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "get_biome",
        required_capability: Some(ModCapability::WorldRead),
        description: "Query biome identifier at (x, y, z)",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "get_player_pos",
        required_capability: Some(ModCapability::WorldRead),
        description: "Get active player coordinates (x, y, z, yaw, pitch)",
    },

    // --- World Modification & Registration (Requires WorldEdit capability) ---
    HostFunctionSpec {
        module: "gradle",
        name: "set_block",
        required_capability: Some(ModCapability::WorldEdit),
        description: "Modify block at (x, y, z) in the active world",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "register_block",
        required_capability: Some(ModCapability::WorldEdit),
        description: "Register a custom block definition into the block palette",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "register_item",
        required_capability: Some(ModCapability::WorldEdit),
        description: "Register a custom item definition into the item registry",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "register_recipe",
        required_capability: Some(ModCapability::WorldEdit),
        description: "Register a custom crafting or furnace recipe",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "spawn_particles",
        required_capability: Some(ModCapability::WorldEdit),
        description: "Spawn visual particles at a position",
    },

    // --- Commands (Requires Commands capability) ---
    HostFunctionSpec {
        module: "gradle",
        name: "register_command",
        required_capability: Some(ModCapability::Commands),
        description: "Register an in-game command handler",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "send_command_feedback",
        required_capability: Some(ModCapability::Commands),
        description: "Send command reply or feedback message to player",
    },

    // --- HUD Rendering (Requires Rendering capability) ---
    HostFunctionSpec {
        module: "gradle",
        name: "draw_rect",
        required_capability: Some(ModCapability::Rendering),
        description: "Draw a filled 2D rectangle with RGBA color on the HUD",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "draw_text",
        required_capability: Some(ModCapability::Rendering),
        description: "Draw formatted 2D text with RGBA color on the HUD",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "draw_texture",
        required_capability: Some(ModCapability::Rendering),
        description: "Draw textured 2D sprite or icon with UV mapping on the HUD",
    },

    // --- Audio (Requires Audio capability) ---
    HostFunctionSpec {
        module: "gradle",
        name: "play_sound",
        required_capability: Some(ModCapability::Audio),
        description: "Trigger in-world 3D positional or ambient sound playback",
    },

    // --- Network (Requires Network capability) ---
    HostFunctionSpec {
        module: "gradle",
        name: "http_get",
        required_capability: Some(ModCapability::Network),
        description: "Perform asynchronous HTTP GET request to approved endpoint",
    },

    // --- Storage / Filesystem (Requires FilesystemRead / FilesystemWrite) ---
    HostFunctionSpec {
        module: "gradle",
        name: "fs_read_config",
        required_capability: Some(ModCapability::FilesystemRead),
        description: "Read mod configuration file from isolated sandbox",
    },
    HostFunctionSpec {
        module: "gradle",
        name: "fs_write_config",
        required_capability: Some(ModCapability::FilesystemWrite),
        description: "Save mod configuration file to isolated sandbox",
    },
];

/// Look up the specification for a host function by module and name.
pub fn lookup_host_function(module: &str, name: &str) -> Option<&'static HostFunctionSpec> {
    OFFICIAL_HOST_FUNCTIONS
        .iter()
        .find(|spec| spec.module == module && spec.name == name)
}
