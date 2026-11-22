//! Instant Launch Pipeline and Optimization Subsystem.
//!
//! Provides the components required for near-instant (< 2s warm, < 5s cold)
//! launch of Gradle Rust Edition:
//! - Content-addressed pre-baked atlases and models with zstd compression
//! - (path, size, mtime) file integrity cache
//! - Persisted wgpu / Vulkan pipeline and shader cache
//! - AOT WASM mod compilation and serialization
//! - Cached system preflight checks
//! - Detailed launch stage tracing spans and benchmarking
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

pub mod benchmark;
pub mod content_cache;
pub mod preflight_cache;
pub mod stat_cache;
pub mod wasm_aot;
pub mod wgpu_cache;

pub use benchmark::*;
pub use content_cache::*;
pub use preflight_cache::*;
pub use stat_cache::*;
pub use wasm_aot::*;
pub use wgpu_cache::*;
