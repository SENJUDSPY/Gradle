# Astralinth 🌌

Astralinth is a fast, lightweight, open-source voxel sandbox game engine written from scratch in pure Rust.

Inspired by modern high-performance voxel engine design (such as *Horizon*), Astralinth focuses on:
- **Zero garbage collection / pure native Rust performance**
- **Adaptive palette-based 16x16x16 chunk section storage** (homogeneous chunks take only 2 bytes of RAM)
- **Aggressive multithreaded-ready face culling** (only rendering visible boundary voxel quads)
- **AABB sweep physics & collision resolution** (gravity, jumping, friction, flight toggle)
- **Exact 3D DDA voxel raycasting** for high-precision block targeting, breaking, and placing
- **In-game 2D HUD** (crosshair, 9-slot hotbar with active item highlight and block icon previews)
- **Persistent world storage** (saving modified chunks to binary format on disk and reloading seamlessly)
- **Multi-threaded background chunk generation & meshing** (`rayon` + worker channels)
- **Deterministic procedural terrain generation** from 64-bit seed (coherent FBM noise, biomes, 3D caves, trees)
- **Modern cross-platform rendering** via `wgpu` (Vulkan on Linux/Windows, Metal on macOS Apple Silicon, DX12 on Windows) and `winit 0.30`
- **100% original identity, art, shaders, textures, and blocks**

---

## Workspace Architecture

```
astralrinth/game/
├── Cargo.toml                     # Standalone workspace root
├── README.md                      # Architecture, guide & controls
├── saves/
│   └── world_1/                   # Persistent world save directory
│       └── chunks/                # Binary modified chunk files (.bin)
├── assets/
│   └── textures/
│       └── blocks/                # 16x16 original pixel-art textures
│           ├── astral_crystal.png
│           ├── astral_stone.png
│           ├── cosmic_sand.png
│           ├── lumina_grass_side.png
│           ├── lumina_grass_top.png
│           ├── star_soil.png
│           ├── stellar_leaves.png
│           ├── stellar_log_side.png
│           └── stellar_log_top.png
└── crates/
    ├── astralinth-core/           # Coordinate systems, BlockId, Direction, PaletteStorage, PlayerPhysics, AABB
    ├── astralinth-world/          # Chunk columns (16x256x16), Biomes, Noise, 3D Caves, Raycast, WorldStorage, WorldManager
    ├── astralinth-render/         # wgpu renderer, pipeline, depth buffer, camera, texture array, HudRenderer
    ├── astralinth-client/         # Main binary, winit event loop, input manager, HUD controller
    ├── astralinth-net/            # Network protocol stub (Phase 6)
    ├── astralinth-server/         # Dedicated server stub (Phase 6)
    └── astralinth-mods/           # WASM sandboxed mod runtime stub (Phase 7)
```

---

## Phase 3 Implementation Summary

### What Works
1. **Player Physics & Collision Engine (`astralinth-core::physics`)**:
   - Axis-Aligned Bounding Box (AABB) collision resolution against solid voxels.
   - Independent axis resolution (Y, X, Z) allowing smooth wall sliding and ground landing.
   - Realistic gravity acceleration and jumping mechanics when grounded.
   - Mode switching: toggle between **Survival/Walking Mode** and **Creative Flight Mode** using key `F`.

2. **Fast 3D DDA Voxel Raycaster (`astralinth-world::raycast`)**:
   - Implements the 3D Digital Differential Analyzer (DDA) algorithm for high-speed, exact voxel traversal.
   - Accurately identifies targeted solid blocks and hit face normals within a 5.5-block reach radius.

3. **Real-Time Block Breaking & Placing**:
   - **Left Click**: Breaks targeted block into `Air`.
   - **Right Click**: Places selected block against targeted face (with collision checking to avoid placing inside player body).
   - Dynamic boundary re-meshing: updating border blocks automatically refreshes adjacent chunk sections.

4. **In-Game 2D HUD (`astralinth-render::hud`)**:
   - Custom screen-space WGSL overlay pipeline (`hud.wgsl`).
   - Crisp central crosshair (`+`).
   - 9-slot bottom Hotbar with glowing cyan selection frame and real-time block icon previews.
   - Hotbar selection via number keys `1`–`9` or mouse scroll wheel.

5. **World Persistence & Storage (`astralinth-world::storage`)**:
   - Binary chunk serialization (`ASTR` format) written to `saves/world_1/chunks/`.
   - Saves block edits atomically to disk; reloads modified chunks when restarting or re-entering the area.

---

## Controls

| Action | Input |
|---|---|
| **Lock / Unlock Mouse** | Left Click / `ESC` |
| **Move / Walk** | `W` / `A` / `S` / `D` |
| **Jump (Walk mode) / Fly Up (Flight mode)** | `Space` |
| **Fly Down (Flight mode)** | `Left Shift` |
| **Toggle Flight / Walk Mode** | `F` |
| **Speed Boost (Flight mode)** | `Left Ctrl` (Hold) |
| **Break Block** | Left Click (Targeted block) |
| **Place Block** | Right Click (Targeted face) |
| **Select Hotbar Slot** | Keys `1` through `9` or Mouse Scroll Wheel |

---

## How to Run

From the game directory:
```bash
cd /home/godxzailas/Downloads/AstralRinth-AR-0.22.01/astralrinth/game
cargo run --release --bin astralinth
```

To run all unit tests:
```bash
cargo test
```
All tests pass with zero warnings across all crates.
