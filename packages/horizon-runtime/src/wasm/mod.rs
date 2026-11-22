//! WebAssembly Mod API, Manifest, Validation, and Build Pipeline for Horizon.
//!
//! Provides the runtime bridge, capability permission enforcement, and
//! build tooling for native `.wasm` and `.hmod` mods.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

pub mod api;
pub mod builder;
pub mod manifest;
pub mod validator;

pub use api::*;
pub use builder::*;
pub use manifest::*;
pub use validator::*;
