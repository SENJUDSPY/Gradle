//! Sample custom block and item registration mod for Gradle / Horizon.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use gradle_mod_sdk::{export_gradle_mod, log_info, BlockBuilder, ItemBuilder};

fn init() {
    log_info("[RubyCraft] Registering Ruby custom content...");

    // Register Ruby Block
    BlockBuilder::new("ruby_craft:ruby_ore", "Ruby Ore")
        .hardness(3.0)
        .blast_resistance(9.0)
        .light_emission(3)
        .drops("ruby_craft:ruby_gem")
        .register();

    // Register Ruby Gem Item
    ItemBuilder::new("ruby_craft:ruby_gem", "Ruby Gem")
        .max_stack_size(64)
        .register();

    // Register Ruby Pickaxe Tool
    ItemBuilder::new("ruby_craft:ruby_pickaxe", "Ruby Pickaxe")
        .durability(1561)
        .register();

    log_info("[RubyCraft] Registered 1 block and 2 items successfully.");
}

export_gradle_mod!(init);
