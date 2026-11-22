//! Custom Block and Item definition builders for Gradle mods.
//!
//! Requires `world_edit` capability declared in `gradle-mod.toml`.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::sys;
use serde::Serialize;

/// Builder for registering custom block types into the game engine.
#[derive(Debug, Clone, Serialize)]
pub struct BlockBuilder {
    pub identifier: String,
    pub name: String,
    pub hardness: f32,
    pub blast_resistance: f32,
    pub light_emission: u8,
    pub opaque: bool,
    pub drop_item_id: Option<String>,
}

impl BlockBuilder {
    pub fn new(identifier: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            name: name.into(),
            hardness: 1.5,
            blast_resistance: 6.0,
            light_emission: 0,
            opaque: true,
            drop_item_id: None,
        }
    }

    pub fn hardness(mut self, hardness: f32) -> Self {
        self.hardness = hardness;
        self
    }

    pub fn blast_resistance(mut self, resistance: f32) -> Self {
        self.blast_resistance = resistance;
        self
    }

    pub fn light_emission(mut self, level: u8) -> Self {
        self.light_emission = level.min(15);
        self
    }

    pub fn transparent(mut self) -> Self {
        self.opaque = false;
        self
    }

    pub fn drops(mut self, item_id: impl Into<String>) -> Self {
        self.drop_item_id = Some(item_id.into());
        self
    }

    /// Registers this block with the Horizon engine.
    pub fn register(&self) -> bool {
        if let Ok(json) = serde_json::to_string(self) {
            let bytes = json.as_bytes();
            let res = unsafe { sys::register_block(bytes.as_ptr(), bytes.len()) };
            res == 0
        } else {
            false
        }
    }
}

/// Builder for registering custom item types into the game engine.
#[derive(Debug, Clone, Serialize)]
pub struct ItemBuilder {
    pub identifier: String,
    pub name: String,
    pub max_stack_size: u32,
    pub durability: Option<u32>,
    pub food_points: Option<u32>,
    pub saturation: Option<f32>,
}

impl ItemBuilder {
    pub fn new(identifier: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            identifier: identifier.into(),
            name: name.into(),
            max_stack_size: 64,
            durability: None,
            food_points: None,
            saturation: None,
        }
    }

    pub fn max_stack_size(mut self, size: u32) -> Self {
        self.max_stack_size = size;
        self
    }

    pub fn durability(mut self, max_damage: u32) -> Self {
        self.durability = Some(max_damage);
        self.max_stack_size = 1;
        self
    }

    pub fn food(mut self, hunger: u32, saturation: f32) -> Self {
        self.food_points = Some(hunger);
        self.saturation = Some(saturation);
        self
    }

    /// Registers this item with the Horizon engine.
    pub fn register(&self) -> bool {
        if let Ok(json) = serde_json::to_string(self) {
            let bytes = json.as_bytes();
            let res = unsafe { sys::register_item(bytes.as_ptr(), bytes.len()) };
            res == 0
        } else {
            false
        }
    }
}
