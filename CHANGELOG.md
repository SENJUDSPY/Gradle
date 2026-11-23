# Changelog

## 1.1.0 - 2026-11-23

### Features
- **Horizon Engine Mod Compatibility**: Complete Phases 1–3 compatibility architecture:
  - Phase 1: Loader-agnostic asset and data extractor with vanilla override rules and strict schema validation.
  - Phase 2: 4-bucket mod compatibility scanner with signature-verified default catalog.
  - Phase 3: Mod porting assistant featuring license verification gates, bytecode blocker detection, and draft Rust/WASM crate generation.
- **UI/UX Anti-Slop Rebuild**: Overhauled design system according to Apple HIG and Linear/Figma design standards:
  - Purged neon halo effects, fuzzy drop shadows, and multi-color gradient sweeps.
  - Implemented critically damped Apple fluid spring motion (`cubic-bezier(0.16, 1, 0.3, 1)` and responsive `80ms` active state scaling).
  - Modernized `Checkbox`, `Toggle`, `Slider`, `NavTabs`, `NavButton`, and `ButtonFrame` with tactile mechanical feedback.
- **Dual Runtime Isolation**: Seamlessly switch between native Rust Horizon engine and classic Java runtime with isolated settings and directory partitioning.
- **Testing Infrastructure**: Built-in 4-tier E2E testing framework runner.

### Security
- Added Zip-Slip path traversal protection in mod archive and modpack extractors.
- Implemented deterministic WASM binary validation and strict linear memory limits.
- Hardened LEB128 shift integer parsing against potential arithmetic overflows.
- Re-architected server world backup to stream directly to zip files, preventing Out-Of-Memory spikes.
