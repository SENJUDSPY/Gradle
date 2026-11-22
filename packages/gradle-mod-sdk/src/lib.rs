//! Official Gradle Mod SDK for Horizon Minecraft Engine.
//!
//! Provides safe, high-level Rust abstractions for building native WebAssembly mods.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

pub mod blocks;
pub mod commands;
pub mod events;
pub mod render;
pub mod sys;

pub use blocks::{BlockBuilder, ItemBuilder};
pub use commands::{register_command, send_feedback};
pub use events::{register_event_listener, EventKind};
pub use render::{draw_rect, draw_text, draw_texture, Color};

/// Logs an informational message to the engine console.
pub fn log_info(msg: &str) {
    let bytes = msg.as_bytes();
    unsafe { sys::log_info(bytes.as_ptr(), bytes.len()) };
}

/// Logs a warning message to the engine console.
pub fn log_warn(msg: &str) {
    let bytes = msg.as_bytes();
    unsafe { sys::log_warn(bytes.as_ptr(), bytes.len()) };
}

/// Logs an error message to the engine console.
pub fn log_error(msg: &str) {
    let bytes = msg.as_bytes();
    unsafe { sys::log_error(bytes.as_ptr(), bytes.len()) };
}

/// Convenience macro for defining the mod entrypoint.
#[macro_export]
macro_rules! export_gradle_mod {
    ($init_fn:ident) => {
        #[no_mangle]
        pub extern "C" fn gradle_init() {
            $init_fn();
        }
    };
}
