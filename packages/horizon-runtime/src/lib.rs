//! Horizon runtime integration for AstralRinth
//!
//! Provides release management, requirements validation, and client/server process execution
//! for Horizon (Minecraft 26.3 rewritten in native Rust).
//!
//! Copyright (c) 2026 AstralRinth contributors / Horizon authors (MIT License).

pub mod execution;
pub mod preparation;
pub mod releases;
pub mod requirements;
pub mod server;

pub use execution::*;
pub use preparation::*;
pub use releases::*;
pub use requirements::*;
pub use server::*;

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
        let rep = check_system_requirements(None);
        assert_eq!(rep.minecraft_26_3.title, "Minecraft 26.3 Jar");
        assert_eq!(rep.java_25.title, "Java 25 (Resource Preparation)");
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
}
