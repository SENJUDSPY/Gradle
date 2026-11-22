//! Blocker detector for the Mod Porting Assistant.
//!
//! Scans Java and Kotlin source files for unmappable patterns:
//! - Mixins into vanilla Minecraft internals
//! - Java Reflection and dynamic bytecode manipulation
//! - Heavy third-party framework dependencies
//! - Raw OpenGL calls and unsafe memory operations
//!
//! Categorizes findings as either 'Needs Mod API Extension' or 'Not Portable'.
//! Never silently approximates gameplay-affecting logic.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Severity classification for an AST/source pattern
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockerSeverity {
    /// Pattern requires an extension to Gradle Mod API (e.g. HUD event hook, food status hook)
    NeedsModApiExtension,
    /// Unmappable JVM pattern (reflection, bytecode manipulation, sun.misc.Unsafe)
    NotPortable,
    /// Pattern is already directly supported by gradle-mod-sdk
    MappableViaSdk,
}

impl BlockerSeverity {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NeedsModApiExtension => "Needs Mod API Extension",
            Self::NotPortable => "Not Portable (JVM Internal)",
            Self::MappableViaSdk => "Supported via Gradle SDK",
        }
    }
}

/// A specific blocker finding in a source file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockerFinding {
    pub file_path: String,
    pub line_number: usize,
    pub severity: BlockerSeverity,
    pub pattern_name: String,
    pub code_snippet: String,
    pub explanation: String,
    pub suggested_remedy: String,
}

/// Comprehensive blocker analysis across an entire mod source repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortingBlockerReport {
    pub repo_root: String,
    pub total_source_files: usize,
    pub total_lines_of_code: usize,
    pub findings: Vec<BlockerFinding>,
    pub needs_api_extension_count: usize,
    pub not_portable_count: usize,
    pub mappable_sdk_count: usize,
    pub is_cleanly_portable: bool,
    pub summary_verdict: String,
}

/// Scans a repository root for all Java/Kotlin files and inspects them for porting blockers
pub fn scan_repository_blockers(repo_root: &Path) -> Result<PortingBlockerReport> {
    let source_files = collect_source_files(repo_root)?;
    let total_source_files = source_files.len();
    let mut total_lines_of_code = 0usize;
    let mut findings = Vec::new();

    for path in &source_files {
        let Ok(content) = fs::read_to_string(path) else {
            continue;
        };

        let rel_path = path
            .strip_prefix(repo_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        let lines: Vec<&str> = content.lines().collect();
        total_lines_of_code += lines.len();

        for (idx, line) in lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();

            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }

            // 1. Detect Reflection
            if trimmed.contains("java.lang.reflect")
                || trimmed.contains("Class.forName(")
                || trimmed.contains(".getDeclaredMethod(")
                || trimmed.contains(".getDeclaredField(")
                || trimmed.contains(".setAccessible(true)")
            {
                findings.push(BlockerFinding {
                    file_path: rel_path.clone(),
                    line_number: line_num,
                    severity: BlockerSeverity::NotPortable,
                    pattern_name: "Java Reflection / Dynamic Introspection".to_string(),
                    code_snippet: trimmed.to_string(),
                    explanation: "Java reflection depends on JVM classloaders and runtime method tables. WebAssembly modules execute strictly typed static functions.".to_string(),
                    suggested_remedy: "Refactor to use explicit typed functions or Gradle Mod API event listeners.".to_string(),
                });
            }

            // 2. Detect Unsafe Memory
            if trimmed.contains("sun.misc.Unsafe") || trimmed.contains("jdk.internal.misc.Unsafe") {
                findings.push(BlockerFinding {
                    file_path: rel_path.clone(),
                    line_number: line_num,
                    severity: BlockerSeverity::NotPortable,
                    pattern_name: "Unsafe JVM Memory Access".to_string(),
                    code_snippet: trimmed.to_string(),
                    explanation: "Direct memory access via sun.misc.Unsafe violates WebAssembly sandboxing and capability security rules.".to_string(),
                    suggested_remedy: "Use safe linear memory buffers or Host API data exchange functions.".to_string(),
                });
            }

            // 3. Detect Mixin Injections into Vanilla Internals
            if trimmed.starts_with("@Mixin(") {
                findings.push(BlockerFinding {
                    file_path: rel_path.clone(),
                    line_number: line_num,
                    severity: BlockerSeverity::NeedsModApiExtension,
                    pattern_name: "@Mixin Class Target".to_string(),
                    code_snippet: trimmed.to_string(),
                    explanation: "Mixins patch vanilla Java bytecode in-place. In Horizon, game logic runs natively in compiled Rust.".to_string(),
                    suggested_remedy: "Map the injection target to a Gradle Mod API event hook (e.g. EventKind::RenderHud, EventKind::Tick).".to_string(),
                });
            }

            if trimmed.starts_with("@Inject(") || trimmed.starts_with("@Redirect(") || trimmed.starts_with("@ModifyVariable(") || trimmed.starts_with("@Overwrite") {
                findings.push(BlockerFinding {
                    file_path: rel_path.clone(),
                    line_number: line_num,
                    severity: BlockerSeverity::NeedsModApiExtension,
                    pattern_name: "Mixin Bytecode Injection Hook".to_string(),
                    code_snippet: trimmed.to_string(),
                    explanation: "Method-level bytecode injection cannot be dynamically spliced into native engine binaries.".to_string(),
                    suggested_remedy: "Implement using register_event_listener() or capability event subscribers.".to_string(),
                });
            }

            // 4. Detect Raw OpenGL
            if trimmed.contains("GL11.") || trimmed.contains("GlStateManager.") || trimmed.contains("RenderSystem.setShader") {
                findings.push(BlockerFinding {
                    file_path: rel_path.clone(),
                    line_number: line_num,
                    severity: BlockerSeverity::NeedsModApiExtension,
                    pattern_name: "Legacy Fixed-Function OpenGL / RenderSystem".to_string(),
                    code_snippet: trimmed.to_string(),
                    explanation: "Horizon utilizes a wgpu / Vulkan rendering pipeline with pre-compiled WGSL shaders rather than legacy OpenGL state machines.".to_string(),
                    suggested_remedy: "Use Gradle SDK high-level 2D draw commands (draw_rect, draw_text, draw_texture).".to_string(),
                });
            }

            // 5. Detect Mappable SDK patterns
            if trimmed.contains("register(") && (trimmed.contains("Registry.ITEM") || trimmed.contains("Registry.BLOCK")) {
                findings.push(BlockerFinding {
                    file_path: rel_path.clone(),
                    line_number: line_num,
                    severity: BlockerSeverity::MappableViaSdk,
                    pattern_name: "Block / Item Registration".to_string(),
                    code_snippet: trimmed.to_string(),
                    explanation: "Standard item and block definitions map cleanly to ItemBuilder and BlockBuilder in gradle-mod-sdk.".to_string(),
                    suggested_remedy: "Use ItemBuilder::new() or BlockBuilder::new() during gradle_init().".to_string(),
                });
            }
        }
    }

    let needs_api_extension_count = findings.iter().filter(|f| f.severity == BlockerSeverity::NeedsModApiExtension).count();
    let not_portable_count = findings.iter().filter(|f| f.severity == BlockerSeverity::NotPortable).count();
    let mappable_sdk_count = findings.iter().filter(|f| f.severity == BlockerSeverity::MappableViaSdk).count();

    let is_cleanly_portable = not_portable_count == 0;
    let summary_verdict = if not_portable_count > 0 {
        format!("BLOCKED: Found {not_portable_count} unmappable JVM patterns (reflection/unsafe). Requires structural redesign.")
    } else if needs_api_extension_count > 0 {
        format!("PORTABLE WITH EXTENSIONS: Found {needs_api_extension_count} patterns requiring Gradle Mod API event hooks.")
    } else {
        "CLEANLY PORTABLE: Source code uses standard concepts directly mappable to gradle-mod-sdk.".to_string()
    };

    Ok(PortingBlockerReport {
        repo_root: repo_root.display().to_string(),
        total_source_files,
        total_lines_of_code,
        findings,
        needs_api_extension_count,
        not_portable_count,
        mappable_sdk_count,
        is_cleanly_portable,
        summary_verdict,
    })
}

fn collect_source_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Ignore build/ and .git/ directories
                let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if name != "build" && name != ".git" && name != "gradle" && name != "target" {
                    files.extend(collect_source_files(&path)?);
                }
            } else if path.is_file() {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                if ext == "java" || ext == "kt" {
                    files.push(path);
                }
            }
        }
    }
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_reflection_and_mixins() {
        let temp = tempfile::tempdir().unwrap();
        let src_dir = temp.path().join("src/main/java/com/example");
        fs::create_dir_all(&src_dir).unwrap();

        let java_code = r#"
package com.example;

import org.spongepowered.asm.mixin.Mixin;
import org.spongepowered.asm.mixin.injection.Inject;
import java.lang.reflect.Method;

@Mixin(targets = "net.minecraft.client.gui.hud.InGameHud")
public class HudMixin {
    @Inject(method = "render", at = @At("HEAD"))
    public void onRender() {
        Class<?> clazz = Class.forName("com.example.Internal");
    }
}
"#;
        fs::write(src_dir.join("HudMixin.java"), java_code).unwrap();

        let report = scan_repository_blockers(temp.path()).unwrap();
        assert_eq!(report.total_source_files, 1);
        assert!(report.needs_api_extension_count >= 2); // @Mixin, @Inject
        assert!(report.not_portable_count >= 1); // Class.forName
        assert!(!report.is_cleanly_portable);
    }
}
