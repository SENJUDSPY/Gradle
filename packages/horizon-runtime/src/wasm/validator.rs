//! WebAssembly Binary Inspection & Capability Validator.
//!
//! Validates binary `.wasm` modules prior to execution:
//! - Parses the WebAssembly 1.0/2.0 binary container using a deterministic zero-dependency parser.
//! - Enforces memory limits (linear memory pages vs. `max_memory_mb`).
//! - Scans all imported host functions and strictly validates them against declared capabilities.
//! - Rejects any module calling disallowed, undeclared, or malicious host functions.
//! - Verifies existence of required mod lifecycle entrypoints.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use super::api::lookup_host_function;
use super::manifest::{GradleModManifest, ModCapability};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

/// Standard WebAssembly page size in bytes (64 KiB).
pub const WASM_PAGE_SIZE_BYTES: u64 = 65_536;

/// Standard WebAssembly binary magic bytes `\0asm`.
pub const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];

/// Standard WebAssembly binary version 1.
pub const WASM_VERSION_1: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

/// Decoded WebAssembly import item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmImport {
    pub module: String,
    pub name: String,
    pub kind: u8, // 0 = func, 1 = table, 2 = mem, 3 = global
}

/// Decoded WebAssembly export item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExport {
    pub name: String,
    pub kind: u8,
    pub index: u32,
}

/// Detailed validation and security report for a WebAssembly mod.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WasmValidationReport {
    pub valid: bool,
    pub mod_id: String,
    pub imports_count: usize,
    pub exports_count: usize,
    pub declared_capabilities: Vec<ModCapability>,
    pub required_capabilities: Vec<ModCapability>,
    pub initial_memory_pages: u32,
    pub max_memory_pages: Option<u32>,
    pub memory_exceeded: bool,
    pub disallowed_imports: Vec<String>,
    pub unknown_imports: Vec<String>,
    pub missing_entrypoint: Option<String>,
    pub errors: Vec<String>,
}

/// WebAssembly binary reader helper with bounds-checking and LEB128 decoding.
struct WasmReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> WasmReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn has_more(&self) -> bool {
        self.pos < self.data.len()
    }

    fn read_byte(&mut self) -> Result<u8> {
        if self.pos >= self.data.len() {
            bail!("Unexpected end of WASM binary data");
        }
        let b = self.data[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_bytes(&mut self, len: usize) -> Result<&'a [u8]> {
        if self.pos + len > self.data.len() {
            bail!("WASM slice of length {} out of bounds", len);
        }
        let slice = &self.data[self.pos..self.pos + len];
        self.pos += len;
        Ok(slice)
    }

    fn read_u32_leb128(&mut self) -> Result<u32> {
        let mut result = 0u32;
        let mut shift = 0;
        loop {
            let byte = self.read_byte()?;
            result |= ((byte & 0x7F) as u32) << shift;
            if (byte & 0x80) == 0 {
                break;
            }
            shift += 7;
            if shift >= 35 {
                bail!("Invalid LEB128 u32 integer encoding");
            }
        }
        Ok(result)
    }

    fn read_string(&mut self) -> Result<String> {
        let len = self.read_u32_leb128()? as usize;
        let bytes = self.read_bytes(len)?;
        String::from_utf8(bytes.to_vec()).context("Invalid UTF-8 string in WASM binary")
    }

    fn read_limits(&mut self) -> Result<(u32, Option<u32>)> {
        let flags = self.read_byte()?;
        let initial = self.read_u32_leb128()?;
        let max = if (flags & 0x01) != 0 {
            Some(self.read_u32_leb128()?)
        } else {
            None
        };
        Ok((initial, max))
    }
}

/// Validator for Gradle WebAssembly mods.
pub struct WasmModValidator;

impl WasmModValidator {
    /// Validates a raw `.wasm` binary against the declared `GradleModManifest`.
    pub fn validate(wasm_bytes: &[u8], manifest: &GradleModManifest) -> Result<WasmValidationReport> {
        if wasm_bytes.len() < 8 {
            bail!("Invalid WASM file: shorter than 8-byte header");
        }

        if &wasm_bytes[0..4] != WASM_MAGIC {
            bail!("Invalid WASM file: missing '\\0asm' magic signature");
        }

        if &wasm_bytes[4..8] != WASM_VERSION_1 {
            bail!("Unsupported WASM version: expected version 1");
        }

        let mut reader = WasmReader::new(&wasm_bytes[8..]);
        let mut imports = Vec::new();
        let mut exports = Vec::new();
        let mut initial_memory_pages = 0u32;
        let mut max_memory_pages = None;

        // Iterate over WASM binary sections
        while reader.has_more() {
            let section_id = reader.read_byte()?;
            let section_len = reader.read_u32_leb128()? as usize;
            let section_bytes = reader.read_bytes(section_len)?;

            match section_id {
                // Section 2: Import Section
                2 => {
                    let mut s_reader = WasmReader::new(section_bytes);
                    let count = s_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        let module = s_reader.read_string()?;
                        let name = s_reader.read_string()?;
                        let kind = s_reader.read_byte()?;
                        match kind {
                            0 => {
                                // function import: type index
                                let _type_idx = s_reader.read_u32_leb128()?;
                            }
                            1 => {
                                // table import
                                let _elem_type = s_reader.read_byte()?;
                                let _limits = s_reader.read_limits()?;
                            }
                            2 => {
                                // memory import
                                let (init, max) = s_reader.read_limits()?;
                                initial_memory_pages = init;
                                max_memory_pages = max;
                            }
                            3 => {
                                // global import
                                let _val_type = s_reader.read_byte()?;
                                let _mutability = s_reader.read_byte()?;
                            }
                            other => bail!("Unknown import kind {} in WASM section 2", other),
                        }
                        imports.push(WasmImport { module, name, kind });
                    }
                }
                // Section 5: Memory Section
                5 => {
                    let mut s_reader = WasmReader::new(section_bytes);
                    let count = s_reader.read_u32_leb128()?;
                    if count > 0 {
                        let (init, max) = s_reader.read_limits()?;
                        initial_memory_pages = init;
                        max_memory_pages = max;
                    }
                }
                // Section 7: Export Section
                7 => {
                    let mut s_reader = WasmReader::new(section_bytes);
                    let count = s_reader.read_u32_leb128()?;
                    for _ in 0..count {
                        let name = s_reader.read_string()?;
                        let kind = s_reader.read_byte()?;
                        let index = s_reader.read_u32_leb128()?;
                        exports.push(WasmExport { name, kind, index });
                    }
                }
                // Other sections are safely skipped for validation purposes
                _ => {}
            }
        }

        // Validate memory limits
        let max_allowed_pages = (manifest.capabilities.limits.max_memory_mb as u64 * 1024 * 1024)
            / WASM_PAGE_SIZE_BYTES;

        let mut errors = Vec::new();
        let mut memory_exceeded = false;

        if (initial_memory_pages as u64) > max_allowed_pages {
            memory_exceeded = true;
            errors.push(format!(
                "Initial memory {} pages ({} MB) exceeds declared limit of {} MB",
                initial_memory_pages,
                (initial_memory_pages as u64 * WASM_PAGE_SIZE_BYTES) / (1024 * 1024),
                manifest.capabilities.limits.max_memory_mb
            ));
        }

        if let Some(max_pages) = max_memory_pages {
            if (max_pages as u64) > max_allowed_pages {
                memory_exceeded = true;
                errors.push(format!(
                    "Maximum memory {} pages ({} MB) exceeds declared limit of {} MB",
                    max_pages,
                    (max_pages as u64 * WASM_PAGE_SIZE_BYTES) / (1024 * 1024),
                    manifest.capabilities.limits.max_memory_mb
                ));
            }
        }

        // Validate entrypoint export
        let mut missing_entrypoint = None;
        let entrypoint = &manifest.info.entrypoint;
        let has_entrypoint = exports
            .iter()
            .any(|e| e.kind == 0 && (e.name == *entrypoint || e.name == "_start" || e.name == "wasm_main"));

        if !has_entrypoint {
            missing_entrypoint = Some(entrypoint.clone());
            errors.push(format!(
                "Required entrypoint function '{}' was not found in module exports",
                entrypoint
            ));
        }

        // Validate imported host functions against permissions
        let mut disallowed_imports = Vec::new();
        let mut unknown_imports = Vec::new();
        let mut required_capabilities = Vec::new();

        for imp in &imports {
            if imp.kind != 0 {
                continue;
            }

            // Only inspect host function calls
            if imp.module == "gradle" {
                if let Some(spec) = lookup_host_function(&imp.module, &imp.name) {
                    if let Some(cap) = spec.required_capability {
                        if !required_capabilities.contains(&cap) {
                            required_capabilities.push(cap);
                        }
                        if !manifest.has_capability(cap) {
                            let msg = format!(
                                "Host function 'gradle::{}' requires capability '{}', which is missing from declared permissions",
                                imp.name,
                                cap.as_str()
                            );
                            disallowed_imports.push(msg.clone());
                            errors.push(msg);
                        }
                    }
                } else {
                    let msg = format!(
                        "Unknown or unsupported host function 'gradle::{}' in Gradle Mod API",
                        imp.name
                    );
                    unknown_imports.push(msg.clone());
                    errors.push(msg);
                }
            } else if imp.module == "wasi_snapshot_preview1" {
                // Safe WASI preview calls (standard I/O / exit)
                let is_safe_wasi = matches!(
                    imp.name.as_str(),
                    "fd_write" | "proc_exit" | "clock_time_get" | "sched_yield" | "random_get"
                );
                if !is_safe_wasi {
                    // Network / filesystem calls require explicit capabilities
                    if imp.name.starts_with("sock_") && !manifest.has_capability(ModCapability::Network) {
                        let msg = format!(
                            "WASI socket function '{}' requires 'network' capability",
                            imp.name
                        );
                        disallowed_imports.push(msg.clone());
                        errors.push(msg);
                    } else if (imp.name.starts_with("path_") || imp.name.starts_with("fd_read"))
                        && !manifest.has_capability(ModCapability::FilesystemRead)
                    {
                        let msg = format!(
                            "WASI filesystem function '{}' requires 'filesystem_read' capability",
                            imp.name
                        );
                        disallowed_imports.push(msg.clone());
                        errors.push(msg);
                    }
                }
            } else if imp.module == "env" {
                // Allow standard harmless math/compiler intrinsics
                let is_safe_intrinsic = matches!(
                    imp.name.as_str(),
                    "abort" | "__assert_fail" | "sin" | "cos" | "tan" | "fmod" | "pow" | "log"
                );
                if !is_safe_intrinsic {
                    let msg = format!("Disallowed external import 'env::{}'", imp.name);
                    unknown_imports.push(msg.clone());
                    errors.push(msg);
                }
            } else {
                let msg = format!(
                    "Disallowed external module import '{}:{}'",
                    imp.module, imp.name
                );
                unknown_imports.push(msg.clone());
                errors.push(msg);
            }
        }

        let valid = errors.is_empty();

        Ok(WasmValidationReport {
            valid,
            mod_id: manifest.info.id.clone(),
            imports_count: imports.len(),
            exports_count: exports.len(),
            declared_capabilities: manifest.capabilities.permissions.clone(),
            required_capabilities,
            initial_memory_pages,
            max_memory_pages,
            memory_exceeded,
            disallowed_imports,
            unknown_imports,
            missing_entrypoint,
            errors,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Constructs a minimal valid WASM binary with an exported `gradle_init` function.
    fn make_minimal_wasm(imports: &[(&str, &str)], memory_pages: u32, export_name: &str) -> Vec<u8> {
        let mut wasm = Vec::new();
        wasm.extend_from_slice(&WASM_MAGIC);
        wasm.extend_from_slice(&WASM_VERSION_1);

        // Type Section: 1 function type () -> ()
        wasm.push(1); // section id
        let type_sec = vec![
            1,    // 1 type
            0x60, // func
            0,    // 0 params
            0,    // 0 returns
        ];
        wasm.push(type_sec.len() as u8);
        wasm.extend_from_slice(&type_sec);

        // Import Section if requested
        if !imports.is_empty() {
            let mut imp_sec = Vec::new();
            imp_sec.push(imports.len() as u8);
            for (m, f) in imports {
                imp_sec.push(m.len() as u8);
                imp_sec.extend_from_slice(m.as_bytes());
                imp_sec.push(f.len() as u8);
                imp_sec.extend_from_slice(f.as_bytes());
                imp_sec.push(0); // func import
                imp_sec.push(0); // type 0
            }
            wasm.push(2);
            wasm.push(imp_sec.len() as u8);
            wasm.extend_from_slice(&imp_sec);
        }

        // Memory Section
        wasm.push(5);
        let mem_sec = vec![
            1, // 1 memory
            0, // flags: no max
            memory_pages as u8,
        ];
        wasm.push(mem_sec.len() as u8);
        wasm.extend_from_slice(&mem_sec);

        // Export Section
        wasm.push(7);
        let mut exp_sec = Vec::new();
        exp_sec.push(1); // 1 export
        exp_sec.push(export_name.len() as u8);
        exp_sec.extend_from_slice(export_name.as_bytes());
        exp_sec.push(0); // func
        exp_sec.push(0); // func index 0
        wasm.push(exp_sec.len() as u8);
        wasm.extend_from_slice(&exp_sec);

        wasm
    }

    #[test]
    fn test_validate_clean_module() {
        let wasm = make_minimal_wasm(&[("gradle", "log_info")], 2, "gradle_init");
        let manifest_toml = r#"
[mod]
id = "clean_mod"
name = "Clean Mod"
version = "1.0.0"
"#;
        let manifest = GradleModManifest::parse_str(manifest_toml).unwrap();
        let report = WasmModValidator::validate(&wasm, &manifest).unwrap();
        assert!(report.valid, "Report errors: {:?}", report.errors);
        assert_eq!(report.disallowed_imports.len(), 0);
        assert_eq!(report.unknown_imports.len(), 0);
    }

    #[test]
    fn test_reject_disallowed_host_function() {
        // Module calls set_block (requires WorldEdit), but manifest has no permissions!
        let wasm = make_minimal_wasm(&[("gradle", "set_block")], 2, "gradle_init");
        let manifest_toml = r#"
[mod]
id = "griefer_mod"
name = "Griefer Mod"
version = "1.0.0"
"#;
        let manifest = GradleModManifest::parse_str(manifest_toml).unwrap();
        let report = WasmModValidator::validate(&wasm, &manifest).unwrap();
        assert!(!report.valid);
        assert_eq!(report.disallowed_imports.len(), 1);
        assert!(report.disallowed_imports[0].contains("requires capability 'world_edit'"));
    }

    #[test]
    fn test_accept_permitted_host_function() {
        // Module calls set_block, and manifest declares WorldEdit permission!
        let wasm = make_minimal_wasm(&[("gradle", "set_block")], 2, "gradle_init");
        let manifest_toml = r#"
[mod]
id = "builder_mod"
name = "Builder Mod"
version = "1.0.0"

[capabilities]
permissions = ["world_edit"]
"#;
        let manifest = GradleModManifest::parse_str(manifest_toml).unwrap();
        let report = WasmModValidator::validate(&wasm, &manifest).unwrap();
        assert!(report.valid, "Expected valid, got: {:?}", report.errors);
    }

    #[test]
    fn test_reject_unknown_host_function() {
        // Module calls non-existent function in gradle namespace
        let wasm = make_minimal_wasm(&[("gradle", "steal_session_token")], 2, "gradle_init");
        let manifest_toml = r#"
[mod]
id = "sketchy_mod"
name = "Sketchy Mod"
version = "1.0.0"
"#;
        let manifest = GradleModManifest::parse_str(manifest_toml).unwrap();
        let report = WasmModValidator::validate(&wasm, &manifest).unwrap();
        assert!(!report.valid);
        assert_eq!(report.unknown_imports.len(), 1);
    }

    #[test]
    fn test_reject_missing_entrypoint() {
        let wasm = make_minimal_wasm(&[], 2, "some_other_function");
        let manifest_toml = r#"
[mod]
id = "bad_entry"
name = "Bad Entry"
version = "1.0.0"
entrypoint = "gradle_init"
"#;
        let manifest = GradleModManifest::parse_str(manifest_toml).unwrap();
        let report = WasmModValidator::validate(&wasm, &manifest).unwrap();
        assert!(!report.valid);
        assert!(report.missing_entrypoint.is_some());
    }
}
