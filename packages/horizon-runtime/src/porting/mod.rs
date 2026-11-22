//! Mod Porting Assistant for Gradle Horizon Engine.
//!
//! Provides a human-reviewed, license-gated toolchain for porting open-source Java
//! mods to native WebAssembly targeting the Gradle Mod API (`packages/gradle-mod-sdk`).
//!
//! Enforces:
//! - Strict license validation (MIT, Apache-2.0, LGPL, GPL). Refuses "All Rights Reserved"
//!   without explicit operator override.
//! - Static blocker detection (Mixins, reflection, unsafe JVM memory).
//! - Draft compilation of .wasm-targeting Rust crates.
//! - Diff-style behavioral comparison and attribution manifests.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

pub mod attribution;
pub mod blocker_detector;
pub mod draft_generator;
pub mod license_gate;

pub use attribution::*;
pub use blocker_detector::*;
pub use draft_generator::*;
pub use license_gate::*;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Outcome of a full porting assistant run on a mod source repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortingSessionReport {
    pub mod_id: String,
    pub mod_name: String,
    pub license_verdict: LicenseVerdict,
    pub blocker_report: PortingBlockerReport,
    pub generated_crate_dir: Option<PathBuf>,
    pub attribution_markdown: String,
    pub behavioral_comparison_markdown: String,
    pub can_compile_wasm: bool,
    pub summary_message: String,
}

/// Runs the complete porting pipeline on an inspected source repository
pub fn execute_porting_pipeline(
    repo_root: &Path,
    output_parent_dir: &Path,
    operator_confirmed_permission: bool,
    mod_id_hint: Option<&str>,
    mod_name_hint: Option<&str>,
    author_hint: Option<&str>,
) -> Result<PortingSessionReport> {
    // 1. License Gate
    let license_verdict = evaluate_repository_license(repo_root, operator_confirmed_permission)
        .context("License gate evaluation failed")?;

    // 2. Blocker Analysis
    let blocker_report = scan_repository_blockers(repo_root)
        .context("Scanning source repository for blockers failed")?;

    // Extract basic metadata or infer from repository folder
    let folder_name = repo_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sample_mod");

    let mod_id = mod_id_hint
        .map(ToString::to_string)
        .unwrap_or_else(|| folder_name.replace('-', "_").to_lowercase());

    let mod_name = mod_name_hint
        .map(ToString::to_string)
        .unwrap_or_else(|| folder_name.replace(['-', '_'], " "));

    let author = author_hint.unwrap_or("Original Mod Author").to_string();

    let spdx = license_verdict
        .license_type
        .spdx_id()
        .unwrap_or("Custom Open Source")
        .to_string();

    // Check if source contains HUD patterns
    let has_hud_overlay = blocker_report.findings.iter().any(|f| {
        f.code_snippet.contains("InGameHud")
            || f.code_snippet.contains("render")
            || f.code_snippet.contains("Hud")
    });

    let has_blocks_or_items = blocker_report.findings.iter().any(|f| {
        f.pattern_name.contains("Block") || f.pattern_name.contains("Item")
    });

    let config = DraftGeneratorConfig {
        mod_id: mod_id.clone(),
        mod_name: mod_name.clone(),
        original_author: author.clone(),
        license_spdx: spdx.clone(),
        has_hud_overlay,
        has_blocks_or_items,
        sdk_path_relative: None,
    };

    // 3. Draft Crate Generation
    let draft_output = generate_draft_crate(output_parent_dir, &config)
        .context("Draft crate generation failed")?;

    // 4. Attribution Manifest
    let attribution = AttributionManifest {
        original_mod_id: mod_id.clone(),
        original_mod_name: mod_name.clone(),
        original_author: author,
        upstream_repository_url: repo_root.display().to_string(),
        upstream_license_spdx: spdx,
        port_author: "Gradle Horizon Porting Assistant".to_string(),
        port_date: chrono::Utc::now().to_rfc3339(),
        derivative_notice: "Ported to native Rust/WebAssembly under the terms of the upstream open-source license. Original copyright belongs to the original author.".to_string(),
    };

    let attribution_md = attribution.to_markdown();
    let _ = fs::write(draft_output.target_crate_dir.join("ATTRIBUTION.md"), &attribution_md);

    let can_compile_wasm = blocker_report.not_portable_count == 0;

    let summary_message = format!(
        "Porting Draft Complete: Generated '{}' with {} source files analyzed. Blocker status: {} blockers, {} API extensions needed.",
        mod_name,
        blocker_report.total_source_files,
        blocker_report.not_portable_count,
        blocker_report.needs_api_extension_count
    );

    Ok(PortingSessionReport {
        mod_id,
        mod_name,
        license_verdict,
        blocker_report,
        generated_crate_dir: Some(draft_output.target_crate_dir),
        attribution_markdown: attribution_md,
        behavioral_comparison_markdown: draft_output.behavioral_comparison_markdown,
        can_compile_wasm,
        summary_message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_end_to_end_porting_pipeline_on_clean_mod() {
        let temp_src = tempfile::tempdir().unwrap();
        let temp_out = tempfile::tempdir().unwrap();

        // 1. Create a clean mock open-source HUD mod
        let repo_root = temp_src.path();
        fs::write(
            repo_root.join("LICENSE"),
            "MIT License\n\nCopyright (c) 2026 Sample Author\n\nPermission is hereby granted...",
        )
        .unwrap();

        let java_dir = repo_root.join("src/main/java/org/sample/hud");
        fs::create_dir_all(&java_dir).unwrap();
        let java_hud = r#"
package org.sample.hud;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.Inject;

@Mixin(targets = "net.minecraft.client.gui.hud.InGameHud")
public class SampleHudMixin {
    @Inject(method = "render", at = @At("RETURN"))
    public void onRender() {
        // Draw HUD overlay
    }
}
"#;
        fs::write(java_dir.join("SampleHudMixin.java"), java_hud).unwrap();

        // 2. Run porting pipeline
        let report = execute_porting_pipeline(
            repo_root,
            temp_out.path(),
            false,
            Some("sample_hud"),
            Some("Sample HUD"),
            Some("Sample Author"),
        )
        .unwrap();

        assert_eq!(report.mod_id, "sample_hud");
        assert!(report.license_verdict.can_proceed);
        assert!(report.blocker_report.needs_api_extension_count >= 1);
        assert_eq!(report.blocker_report.not_portable_count, 0);
        assert!(report.can_compile_wasm);

        let crate_dir = report.generated_crate_dir.unwrap();
        assert!(crate_dir.join("Cargo.toml").is_file());
        assert!(crate_dir.join("gradle-mod.toml").is_file());
        assert!(crate_dir.join("src/lib.rs").is_file());
        assert!(crate_dir.join("ATTRIBUTION.md").is_file());
        assert!(crate_dir.join("BEHAVIORAL_COMPARISON.md").is_file());

        // Check generated code
        let lib_code = fs::read_to_string(crate_dir.join("src/lib.rs")).unwrap();
        assert!(lib_code.contains("export_gradle_mod!"));
        assert!(lib_code.contains("draw_rect"));
        assert!(lib_code.contains("register_event_listener"));
    }
}
