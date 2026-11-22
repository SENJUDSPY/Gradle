//! WebAssembly Mod Packaging and Build Pipeline.
//!
//! Provides the tooling to build, validate, and package native Gradle WebAssembly mods:
//! - Compiles Rust projects to `wasm32-unknown-unknown`.
//! - Validates imports, capabilities, and memory limits against `gradle-mod.toml`.
//! - Packages `mod.wasm`, manifest, assets, and datapacks into an `.hmod` archive.
//! - Computes cryptographic SHA-256 integrity digests.
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

use super::manifest::GradleModManifest;
use super::validator::{WasmModValidator, WasmValidationReport};
use anyhow::{Context, Result, bail};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

/// Options controlling the mod build and packaging process.
#[derive(Debug, Clone)]
pub struct ModBuildOptions {
    /// Directory containing `gradle-mod.toml` and mod source files.
    pub project_dir: PathBuf,
    /// Build with `--release` profile (optimizations enabled).
    pub release: bool,
    /// Optional output destination directory for the generated `.hmod`.
    pub out_dir: Option<PathBuf>,
}

/// Resulting artifacts from a successful mod build.
#[derive(Debug, Clone)]
pub struct ModBuildArtifact {
    pub hmod_path: PathBuf,
    pub wasm_path: PathBuf,
    pub sha256: String,
    pub size_bytes: u64,
    pub manifest: GradleModManifest,
    pub validation_report: WasmValidationReport,
}

/// Builds, validates, and packages a Gradle WASM mod.
pub fn build_and_package_mod(options: &ModBuildOptions) -> Result<ModBuildArtifact> {
    let project_dir = &options.project_dir;
    if !project_dir.is_dir() {
        bail!("Mod project directory does not exist: {}", project_dir.display());
    }

    // 1. Locate and parse manifest (gradle-mod.toml or horizon-mod.toml)
    let manifest_path = if project_dir.join("gradle-mod.toml").is_file() {
        project_dir.join("gradle-mod.toml")
    } else if project_dir.join("horizon-mod.toml").is_file() {
        project_dir.join("horizon-mod.toml")
    } else {
        bail!(
            "Missing mod manifest in {}. Expected 'gradle-mod.toml' or 'horizon-mod.toml'",
            project_dir.display()
        );
    };

    let manifest = GradleModManifest::load_file(&manifest_path)?;

    // 2. Compile or locate WebAssembly binary
    let cargo_toml = project_dir.join("Cargo.toml");
    let wasm_file = if cargo_toml.is_file() {
        compile_rust_wasm(project_dir, options.release, &manifest.info.id)?
    } else {
        // Fall back to looking for precompiled mod.wasm or <id>.wasm in project dir
        let direct_wasm = project_dir.join(format!("{}.wasm", manifest.info.id));
        let default_wasm = project_dir.join("mod.wasm");
        if direct_wasm.is_file() {
            direct_wasm
        } else if default_wasm.is_file() {
            default_wasm
        } else {
            bail!(
                "No Cargo.toml or pre-compiled .wasm found in {}",
                project_dir.display()
            );
        }
    };

    // 3. Read and validate WASM binary against capability manifest
    let wasm_bytes = fs::read(&wasm_file)
        .with_context(|| format!("Failed to read compiled wasm file at {}", wasm_file.display()))?;

    let validation_report = WasmModValidator::validate(&wasm_bytes, &manifest)?;
    if !validation_report.valid {
        bail!(
            "Mod validation failed for '{}' ({}):\n{}",
            manifest.info.name,
            manifest.info.id,
            validation_report.errors.join("\n- ")
        );
    }

    // 4. Prepare output bundle directory
    let target_out_dir = match &options.out_dir {
        Some(dir) => dir.clone(),
        None => project_dir.join("dist"),
    };
    fs::create_dir_all(&target_out_dir)?;

    let hmod_file_name = format!("{}-{}.hmod", manifest.info.id, manifest.info.version);
    let hmod_path = target_out_dir.join(hmod_file_name);

    // 5. Package .hmod (Zip archive)
    package_hmod_archive(&hmod_path, &manifest_path, &wasm_file, project_dir)?;

    // 6. Compute SHA-256
    let hmod_bytes = fs::read(&hmod_path)?;
    let mut hasher = Sha256::new();
    hasher.update(&hmod_bytes);
    let sha256 = hex::encode(hasher.finalize());
    let size_bytes = hmod_bytes.len() as u64;

    Ok(ModBuildArtifact {
        hmod_path,
        wasm_path: wasm_file,
        sha256,
        size_bytes,
        manifest,
        validation_report,
    })
}

/// Runs `cargo build --target wasm32-unknown-unknown`
fn compile_rust_wasm(project_dir: &Path, release: bool, mod_id: &str) -> Result<PathBuf> {
    let mut cmd = Command::new("cargo");
    cmd.arg("build")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .current_dir(project_dir);

    if release {
        cmd.arg("--release");
    }

    let status = cmd
        .status()
        .with_context(|| "Failed to execute 'cargo build --target wasm32-unknown-unknown'")?;

    if !status.success() {
        bail!("Compilation of WASM mod failed (cargo exited with error)");
    }

    let profile = if release { "release" } else { "debug" };
    let target_dir = project_dir.join("target").join("wasm32-unknown-unknown").join(profile);

    // Look for matching .wasm in target dir
    let mut candidates = Vec::new();
    if let Ok(entries) = fs::read_dir(&target_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("wasm") {
                candidates.push(path);
            }
        }
    }

    // Prefer exact match or single output
    if let Some(exact) = candidates.iter().find(|p| {
        p.file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s == mod_id || s == mod_id.replace('-', "_"))
            .unwrap_or(false)
    }) {
        return Ok(exact.clone());
    }

    if candidates.len() == 1 {
        return Ok(candidates[0].clone());
    }

    bail!(
        "Could not find compiled .wasm for mod '{}' in {}",
        mod_id,
        target_dir.display()
    );
}

/// Packages manifest, wasm binary, and optional assets into `.hmod` archive.
fn package_hmod_archive(
    out_hmod: &Path,
    manifest_path: &Path,
    wasm_path: &Path,
    project_dir: &Path,
) -> Result<()> {
    let file = File::create(out_hmod)
        .with_context(|| format!("Failed to create .hmod file at {}", out_hmod.display()))?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // 1. Write gradle-mod.toml
    zip.start_file("gradle-mod.toml", options)?;
    let manifest_bytes = fs::read(manifest_path)?;
    zip.write_all(&manifest_bytes)?;

    // 2. Write mod.wasm
    zip.start_file("mod.wasm", options)?;
    let wasm_bytes = fs::read(wasm_path)?;
    zip.write_all(&wasm_bytes)?;

    // 3. Package assets/ directory if present
    let assets_dir = project_dir.join("assets");
    if assets_dir.is_dir() {
        add_dir_to_zip(&mut zip, &assets_dir, Path::new("assets"), options)?;
    }

    // 4. Package datapacks/ directory if present
    let datapacks_dir = project_dir.join("datapacks");
    if datapacks_dir.is_dir() {
        add_dir_to_zip(&mut zip, &datapacks_dir, Path::new("datapacks"), options)?;
    }

    zip.finish()?;
    Ok(())
}

fn add_dir_to_zip<W: Write + std::io::Seek>(
    zip: &mut ZipWriter<W>,
    src_dir: &Path,
    prefix_in_zip: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(src_dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let zip_path = prefix_in_zip.join(&name);

        if path.is_dir() {
            add_dir_to_zip(zip, &path, &zip_path, options)?;
        } else if path.is_file() {
            let zip_path_str = zip_path.to_string_lossy().replace('\\', "/");
            zip.start_file(zip_path_str, options)?;
            let data = fs::read(&path)?;
            zip.write_all(&data)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_package_precompiled_hmod() {
        let dir = tempdir().unwrap();
        let manifest_content = r#"
[mod]
id = "test_pack"
name = "Test Pack"
version = "1.0.0"

[capabilities]
permissions = ["rendering"]
"#;
        fs::write(dir.path().join("gradle-mod.toml"), manifest_content).unwrap();

        // Minimal valid wasm
        let mut wasm_bytes = Vec::new();
        wasm_bytes.extend_from_slice(&[0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);
        // export gradle_init
        wasm_bytes.extend_from_slice(&[
            1, 4, 1, 0x60, 0, 0, // type section () -> ()
            5, 3, 1, 0, 1,       // memory section (1 page)
            7, 15, 1, 11, b'g', b'r', b'a', b'd', b'l', b'e', b'_', b'i', b'n', b'i', b't', 0, 0, // export
        ]);
        fs::write(dir.path().join("mod.wasm"), wasm_bytes).unwrap();

        // Create assets folder
        let assets_dir = dir.path().join("assets").join("test_pack");
        fs::create_dir_all(&assets_dir).unwrap();
        fs::write(assets_dir.join("test.txt"), b"sample asset").unwrap();

        let options = ModBuildOptions {
            project_dir: dir.path().to_path_buf(),
            release: false,
            out_dir: None,
        };

        let artifact = build_and_package_mod(&options).unwrap();
        assert!(artifact.hmod_path.is_file());
        assert_eq!(artifact.manifest.info.id, "test_pack");
        assert!(!artifact.sha256.is_empty());
        assert!(artifact.size_bytes > 0);

        // Verify inside zip
        let zip_file = File::open(&artifact.hmod_path).unwrap();
        let mut archive = zip::ZipArchive::new(zip_file).unwrap();
        assert!(archive.by_name("gradle-mod.toml").is_ok());
        assert!(archive.by_name("mod.wasm").is_ok());
        assert!(archive.by_name("assets/test_pack/test.txt").is_ok());
    }
}
