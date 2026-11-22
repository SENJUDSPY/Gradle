//! High-level 2D HUD rendering helpers for Gradle WebAssembly mods.
//!
//! Requires `rendering` capability declared in `gradle-mod.toml`.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::sys;

/// Packed 32-bit RGBA color representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const WHITE: Self = Self::rgba(255, 255, 255, 255);
    pub const BLACK: Self = Self::rgba(0, 0, 0, 255);
    pub const RED: Self = Self::rgba(255, 50, 50, 255);
    pub const GREEN: Self = Self::rgba(50, 255, 50, 255);
    pub const BLUE: Self = Self::rgba(50, 100, 255, 255);
    pub const TRANSPARENT_BLACK: Self = Self::rgba(0, 0, 0, 160);

    pub const fn to_u32(self) -> u32 {
        ((self.r as u32) << 24) | ((self.g as u32) << 16) | ((self.b as u32) << 8) | (self.a as u32)
    }
}

/// Draws a filled rectangle on the HUD overlay.
pub fn draw_rect(x: f32, y: f32, width: f32, height: f32, color: Color) {
    unsafe {
        sys::draw_rect(x, y, width, height, color.to_u32());
    }
}

/// Draws formatted text on the HUD overlay.
pub fn draw_text(x: f32, y: f32, text: &str, color: Color) {
    let bytes = text.as_bytes();
    unsafe {
        sys::draw_text(x, y, bytes.as_ptr(), bytes.len(), color.to_u32());
    }
}

/// Draws a textured 2D sprite with UV coordinates on the HUD.
pub fn draw_texture(
    texture_id: u32,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    u0: f32,
    v0: f32,
    u1: f32,
    v1: f32,
) {
    unsafe {
        sys::draw_texture(texture_id, x, y, width, height, u0, v0, u1, v1);
    }
}
