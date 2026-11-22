//! In-game command registration for Gradle mods.
//!
//! Requires `commands` capability declared in `gradle-mod.toml`.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::sys;

/// Registers a chat or console command with an associated callback identifier.
pub fn register_command(name: &str, callback_id: u32) -> bool {
    let bytes = name.as_bytes();
    let res = unsafe { sys::register_command(bytes.as_ptr(), bytes.len(), callback_id) };
    res == 0
}

/// Sends command feedback message to a player.
pub fn send_feedback(player_id: u64, message: &str) {
    let bytes = message.as_bytes();
    unsafe {
        sys::send_command_feedback(player_id, bytes.as_ptr(), bytes.len());
    }
}
