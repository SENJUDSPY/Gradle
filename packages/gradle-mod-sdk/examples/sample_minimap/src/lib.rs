//! Sample Minimap WASM mod for Gradle/Horizon Minecraft engine.
//!
//! Demonstrates:
//! - Entrypoint initialization via `export_gradle_mod!`
//! - Registering a HUD rendering callback
//! - Drawing transparent overlays, borders, and text on the screen
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use gradle_mod_sdk::{
    draw_rect, draw_text, export_gradle_mod, log_info, Color, EventKind, register_event_listener,
};

fn init() {
    log_info("[SampleMinimap] Initializing WASM minimap HUD overlay...");
    // Register callback for RenderHud event (function index 1)
    register_event_listener(EventKind::RenderHud, 1);
}

export_gradle_mod!(init);

/// Exported frame rendering callback invoked by Horizon every HUD frame.
#[no_mangle]
pub extern "C" fn on_render_hud(_screen_width: f32, _screen_height: f32) {
    // 1. Draw semi-transparent dark radar box in top-left
    draw_rect(16.0, 16.0, 110.0, 110.0, Color::TRANSPARENT_BLACK);

    // 2. Draw border
    draw_rect(16.0, 16.0, 110.0, 2.0, Color::WHITE);
    draw_rect(16.0, 124.0, 110.0, 2.0, Color::WHITE);
    draw_rect(16.0, 16.0, 2.0, 110.0, Color::WHITE);
    draw_rect(124.0, 16.0, 2.0, 110.0, Color::WHITE);

    // 3. Draw compass and status
    draw_text(66.0, 22.0, "N", Color::RED);
    draw_text(24.0, 68.0, "RADAR ON", Color::GREEN);
}
