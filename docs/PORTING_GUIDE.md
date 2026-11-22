# Gradle / Horizon WebAssembly Mod Porting Guide

A developer guide for porting Java Minecraft mods (Fabric, Quilt, Forge, NeoForge) to **native WebAssembly mods** (`.wasm` / `.hmod`) running on **Gradle Rust Edition** (Horizon).

---

## 1. Architectural Overview: Java (JVM) vs. Gradle (WASM)

| Dimension | Java Edition (Fabric/Forge) | Gradle Rust Edition (Horizon) |
|---|---|---|
| **Runtime** | Java Virtual Machine (JVM 21/25) | Native Rust wgpu Engine + WebAssembly Sandbox |
| **Modification Mechanism** | Bytecode manipulation, ASM, Mixins | Declarative Event Hooks & Host ABI |
| **Startup Time** | 15 – 45 seconds (bytecode transform) | Instant (< 2 seconds warm, < 5 seconds cold) |
| **Memory Footprint** | 2 – 8 GB RAM (GC pauses) | 256 MB – 1 GB RAM (zero garbage collector) |
| **Security & Isolation** | Unrestricted JVM process access | Capability-based security + strict instruction fuel metering |
| **Mod Format** | `.jar` (ZIP of `.class` files) | `.hmod` (ZIP containing `mod.wasm` + `gradle-mod.toml`) |

---

## 2. Mod Manifest (`gradle-mod.toml`)

Every native mod must provide a manifest file at the root named `gradle-mod.toml` (or `horizon-mod.toml`):

```toml
[mod]
id = "minimap_pro"
name = "Minimap Pro"
version = "1.0.0"
authors = ["YourName"]
description = "Fast real-time HUD minimap rendered with native wgpu"
license = "MIT"
entrypoint = "gradle_init"

[capabilities]
permissions = ["rendering", "world_read"]

[capabilities.limits]
max_memory_mb = 32
max_fuel = 10000000

[dependencies]
horizon = ">=26.3"
```

### Capability-Based Security Model
Gradle enforces strict least-privilege security. If your module imports a host function without declaring the corresponding capability, the launcher **rejects** the mod before execution:

| Capability | Allowed Host Operations |
|---|---|
| `rendering` | Draw 2D HUD rectangles, text, and textured sprites |
| `world_read` | Query block IDs, biomes, and player positions |
| `world_edit` | Place/break blocks, register blocks, items, recipes, spawn particles |
| `commands` | Register in-game slash commands and send player feedback |
| `network` | Perform asynchronous outbound HTTP GET/POST requests |
| `filesystem_read` | Read configuration files in the mod's isolated data sandbox |
| `filesystem_write` | Save configuration files in the mod's isolated data sandbox |
| `audio` | Play custom 3D positional and ambient audio effects |
| `player_events` | Listen to player join, interaction, and movement events |

---

## 3. Step-by-Step Porting Walkthrough

### Step 1: Converting the Entrypoint

In Fabric Java:
```java
public class ExampleMod implements ModInitializer {
    @Override
    public void onInitialize() {
        System.out.println("Hello from Fabric!");
    }
}
```

In Gradle Rust:
```rust
use gradle_mod_sdk::{export_gradle_mod, log_info};

fn init() {
    log_info("Hello from Gradle WebAssembly!");
}

export_gradle_mod!(init);
```

---

### Step 2: Replacing Mixin HUD Injections with Rendering Hooks

In Fabric, developers typically inject into `InGameHud.render()` using Mixins:
```java
@Mixin(InGameHud.class)
public class InGameHudMixin {
    @Inject(method = "render", at = @At("RETURN"))
    private void renderRadar(DrawContext context, RenderTickCounter tickCounter, CallbackInfo ci) {
        context.fill(10, 10, 100, 100, 0x80000000);
    }
}
```

In Gradle, declare `permissions = ["rendering"]` in `gradle-mod.toml` and register the `RenderHud` callback:
```rust
use gradle_mod_sdk::{
    draw_rect, draw_text, export_gradle_mod, register_event_listener, Color, EventKind,
};

fn init() {
    register_event_listener(EventKind::RenderHud, 1);
}

export_gradle_mod!(init);

#[no_mangle]
pub extern "C" fn on_render_hud(screen_width: f32, screen_height: f32) {
    // 1. Draw transparent HUD panel
    draw_rect(10.0, 10.0, 90.0, 90.0, Color::TRANSPARENT_BLACK);
    // 2. Draw border
    draw_rect(10.0, 10.0, 90.0, 2.0, Color::WHITE);
    // 3. Draw text label
    draw_text(16.0, 16.0, "RADAR", Color::GREEN);
}
```

---

### Step 3: Registering Custom Blocks and Items

In Fabric:
```java
public static final Block RUBY_ORE = Registry.register(
    Registries.BLOCK, new Identifier("mymod", "ruby_ore"),
    new Block(FabricBlockSettings.create().strength(3.0f))
);
```

In Gradle (requires `permissions = ["world_edit"]`):
```rust
use gradle_mod_sdk::{export_gradle_mod, BlockBuilder, ItemBuilder};

fn init() {
    // Custom block
    BlockBuilder::new("mymod:ruby_ore", "Ruby Ore")
        .hardness(3.0)
        .blast_resistance(9.0)
        .light_emission(3)
        .drops("mymod:ruby_gem")
        .register();

    // Custom item
    ItemBuilder::new("mymod:ruby_gem", "Ruby Gem")
        .max_stack_size(64)
        .register();
}

export_gradle_mod!(init);
```

---

## 4. Building and Packaging Your Mod

### Prerequisites
Add the WebAssembly target to your Rust toolchain:
```bash
rustup target add wasm32-unknown-unknown
```

### Compiling to WASM
In your mod's project directory:
```bash
cargo build --target wasm32-unknown-unknown --release
```

### Packaging into `.hmod`
An `.hmod` file is a ZIP archive containing:
1. `gradle-mod.toml`
2. `mod.wasm`
3. Optional assets: `assets/<mod_id>/textures/...`

Using the Horizon CLI tool:
```bash
horizon-ctl mod package --project-dir . --out-dir dist/
```

Or using standard zip utilities:
```bash
zip -r dist/mymod-1.0.0.hmod gradle-mod.toml mod.wasm assets/
```

---

## 5. Troubleshooting Validation Failures

If the Gradle launcher rejects your mod upon launch with:
```text
Host function 'gradle::draw_rect' requires capability 'rendering', which is missing from declared permissions
```
**Resolution:** Add `"rendering"` to `[capabilities] permissions` in your `gradle-mod.toml`:
```toml
[capabilities]
permissions = ["rendering"]
```

If rejected with:
```text
Initial memory 1024 pages (64 MB) exceeds declared limit of 32 MB
```
**Resolution:** Increase `max_memory_mb` in `[capabilities.limits]` or optimize linear memory allocations in your Rust source.
