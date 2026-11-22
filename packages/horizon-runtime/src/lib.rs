//! Horizon runtime integration for Gradle
//!
//! Provides release management, requirements validation, and client/server process execution
//! for Horizon (Minecraft 26.3 rewritten in native Rust).
//!
//! Copyright (c) 2026 Gradle contributors / Horizon authors (MIT License).

pub mod agent;
pub mod catalog;
pub mod classifier;
pub mod crash;
pub mod execution;
pub mod extractor;
pub mod modpack;
pub mod preparation;
pub mod pipeline;
pub mod porting;
pub mod releases;
pub mod requirements;
pub mod server;
pub mod settings;
pub mod wasm;

pub use agent::*;
pub use catalog::*;
pub use classifier::*;
pub use crash::*;
pub use execution::*;
pub use extractor::*;
pub use modpack::*;
pub use pipeline::*;
pub use porting::*;
pub use preparation::*;
pub use releases::*;
pub use requirements::*;
pub use server::*;
pub use settings::*;
pub use wasm::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_asset_resolution() {
        let asset = platform_asset_name();
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        assert_eq!(asset, Some("horizon-linux-x86_64.tar.gz"));
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert_eq!(asset, Some("horizon-macos-arm64.zip"));
        #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
        assert_eq!(asset, Some("horizon-windows-x64.zip"));
    }

    #[test]
    fn test_minecraft_26_3_constants() {
        assert_eq!(MINECRAFT_26_3_VERSION, "26.3");
        assert_eq!(
            MINECRAFT_26_3_CLIENT_SHA1,
            "e877b6a07acd633fb3bb475002175cec036e7b87"
        );
        assert_eq!(
            MINECRAFT_26_3_SERVER_SHA1,
            "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c"
        );
    }

    #[test]
    fn test_check_requirements_report() {
        let rep = check_system_requirements(None, None);
        assert_eq!(rep.minecraft_26_3.title, "Minecraft 26.3 Jar");
        assert_eq!(rep.java_25.title, "Java 25 (Resource Preparation)");
        assert_eq!(rep.disk_space.title, "Disk Space");
        assert_eq!(rep.runtime_installed.title, "Gradle Engine (Rust Edition)");
        assert_eq!(rep.resources_prepared.title, "Game Resources");
        assert_eq!(rep.valid_account.title, "Minecraft Account");
    }

    #[test]
    fn test_generate_diagnostics_report() {
        let diag = generate_diagnostics_report(None, Some("TestUser"));
        assert_eq!(diag.requirements.valid_account.passed, true);
        assert!(!diag.os_info.is_empty());
        assert!(!diag.generated_at.is_empty());
    }

    #[test]
    fn test_preparation_constants() {
        assert_eq!(MANIFEST_NAME, "horizon-resources.json");
        assert_eq!(SCHEMA_VERSION, 7);
        let res_dir = prepared_resources_dir();
        assert!(res_dir.ends_with("resources"));
    }

    #[test]
    fn test_paths() {
        let data_dir = horizon_data_dir();
        let vdir = version_dir("v1.3.0");
        assert!(vdir.starts_with(&data_dir));
        assert_eq!(vdir.file_name().unwrap(), "v1.3.0");
    }

    #[tokio::test]
    async fn test_fetch_releases_fallback_and_cache() {
        let releases = fetch_releases_for(Some("upstream")).await.unwrap();
        assert!(!releases.is_empty());
        let first = &releases[0];
        assert!(!first.tag.is_empty());
        assert!(!first.name.is_empty());
        assert!(!first.body.is_empty());

        let offline_releases = fetch_releases_for(None).await.unwrap();
        assert!(!offline_releases.is_empty());
    }
}
