//! Event handling and lifecycle callback registration for Gradle mods.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use crate::sys;

/// Supported event hooks for native Gradle mods.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Init = 1,
    Tick = 2,
    BlockBreak = 3,
    BlockPlace = 4,
    EntityInteract = 5,
    Chat = 6,
    RenderHud = 7,
}

/// Registers an event listener function index with the host engine.
pub fn register_event_listener(event: EventKind, callback_id: u32) {
    unsafe {
        sys::register_event(event as u32, callback_id);
    }
}
