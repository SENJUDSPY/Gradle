use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const MINECRAFT_26_3_VERSION: &str = "26.3";
pub const MINECRAFT_26_3_CLIENT_SHA1: &str =
    "e877b6a07acd633fb3bb475002175cec036e7b87";
pub const MINECRAFT_26_3_SERVER_SHA1: &str =
    "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    pub passed: bool,
    pub title: String,
    pub details: String,
    pub path: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirementsReport {
    pub all_passed: bool,
    pub minecraft_26_3: CheckItem,
    pub java_25: CheckItem,
    pub gpu_vulkan: CheckItem,
}

fn compute_sha1(path: &Path) -> Option<String> {
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha1::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

pub fn find_default_minecraft_dirs() -> Vec<PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let data = dirs::data_dir().unwrap_or_else(|| home.join(".local/share"));
    vec![
        home.join(".minecraft"),
        home.join(".var/app/com.mojang.Minecraft/.minecraft"),
        data.join("minecraft"),
        data.join(".minecraft"),
    ]
}

pub fn check_minecraft_26_3(custom_path: Option<&Path>) -> CheckItem {
    if let Some(path) = custom_path {
        if path.is_file() {
            if let Some(sha1) = compute_sha1(path) {
                if sha1 == MINECRAFT_26_3_CLIENT_SHA1 {
                    return CheckItem {
						passed: true,
						title: "Minecraft 26.3 Jar".to_string(),
						details: "Found valid Minecraft Java 26.3 client jar at specified path.".to_string(),
						path: Some(path.to_string_lossy().to_string()),
						error: None,
					};
                } else {
                    return CheckItem {
						passed: false,
						title: "Minecraft 26.3 Jar".to_string(),
						details: format!("Provided jar SHA-1 ({sha1}) does not match pinned 26.3 client jar ({MINECRAFT_26_3_CLIENT_SHA1})."),
						path: Some(path.to_string_lossy().to_string()),
						error: Some("Checksum mismatch: file is not official 26.3 client jar.".to_string()),
					};
                }
            }
        }
    }

    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let data = dirs::data_dir().unwrap_or_else(|| home.join(".local/share"));

    let candidate_locations = [
		// Official launcher
		home.join(".minecraft/versions/26.3/26.3.jar"),
		home.join(".var/app/com.mojang.Minecraft/.minecraft/versions/26.3/26.3.jar"),
		data.join("minecraft/versions/26.3/26.3.jar"),
		data.join(".minecraft/versions/26.3/26.3.jar"),
		// Prism Launcher / MultiMC
		data.join("PrismLauncher/libraries/com/mojang/minecraft/26.3/minecraft-26.3-client.jar"),
		data.join("org.prismlauncher.PrismLauncher/data/PrismLauncher/libraries/com/mojang/minecraft/26.3/minecraft-26.3-client.jar"),
		// Modrinth App
		data.join("ModrinthApp/meta/versions/26.3/26.3.jar"),
		data.join("com.modrinth.theseus/meta/versions/26.3/26.3.jar"),
		// ATLauncher
		data.join("atlauncher/versions/26.3/26.3.jar"),
	];

    for candidate in &candidate_locations {
        if candidate.is_file() {
            if let Some(sha1) = compute_sha1(candidate) {
                if sha1 == MINECRAFT_26_3_CLIENT_SHA1 {
                    return CheckItem {
						passed: true,
						title: "Minecraft 26.3 Jar".to_string(),
						details: "Found official Minecraft 26.3 client jar in local launcher library.".to_string(),
						path: Some(candidate.to_string_lossy().to_string()),
						error: None,
					};
                }
            }
        }
    }

    CheckItem {
		passed: false,
		title: "Minecraft 26.3 Jar".to_string(),
		details: "Minecraft Java Edition 26.3 was not found in local launcher directories.".to_string(),
		path: None,
		error: Some("Run Minecraft 26.3 once in your Minecraft launcher to download it, or select your 26.3.jar manually.".to_string()),
	}
}

pub fn check_java_25() -> CheckItem {
    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    let data = dirs::data_dir().unwrap_or_else(|| home.join(".local/share"));

    let possible_java_bins = [
        PathBuf::from("/usr/lib/jvm/java-25-openjdk/bin/java"),
        PathBuf::from("/usr/lib/jvm/temurin-25-jdk/bin/java"),
        home.join(".local/share/jdk-25/bin/java"),
        home.join(".local/share/jvm/jdk-25/bin/java"),
        home.join(".minecraft/runtime/java-runtime-epsilon/bin/java"),
        data.join("ModrinthApp/meta/java_versions/25/bin/java"),
        PathBuf::from("java"),
    ];

    for bin in &possible_java_bins {
        if let Ok(output) = Command::new(bin).arg("-version").output() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let combined = format!("{stderr}\n{stdout}");
            if combined.contains("\"25")
                || combined.contains("version 25")
                || combined.contains("openjdk 25")
            {
                return CheckItem {
					passed: true,
					title: "Java 25 (Resource Preparation)".to_string(),
					details: "Java 25 runtime is available for local game asset preparation.".to_string(),
					path: Some(bin.to_string_lossy().to_string()),
					error: None,
				};
            }
        }
    }

    CheckItem {
		passed: false,
		title: "Java 25 (Resource Preparation)".to_string(),
		details: "Java 25 is required for one-time game file and bytecode preparation from your Minecraft copy.".to_string(),
		path: None,
		error: Some("Install Java 25 (Adoptium Temurin 25: adoptium.net) or download it in AstralRinth Java Settings.".to_string()),
	}
}

pub fn check_gpu_compatibility() -> CheckItem {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "aarch64") {
            return CheckItem {
                passed: true,
                title: "Graphics Hardware".to_string(),
                details:
                    "Apple Silicon GPU with native Metal renderer supported."
                        .to_string(),
                path: None,
                error: None,
            };
        } else {
            return CheckItem {
                passed: true,
                title: "Graphics Hardware".to_string(),
                details: "Intel Mac with Metal/MoltenVK support.".to_string(),
                path: None,
                error: None,
            };
        }
    }

    #[cfg(target_os = "linux")]
    {
        let common_vulkan_libs = [
            "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
            "/usr/lib64/libvulkan.so.1",
            "/usr/lib/libvulkan.so.1",
            "/lib/x86_64-linux-gnu/libvulkan.so.1",
        ];
        let lib_found =
            common_vulkan_libs.iter().any(|p| Path::new(p).exists());

        let icd_dir = Path::new("/usr/share/vulkan/icd.d");
        let has_icd = icd_dir.is_dir()
            && fs::read_dir(icd_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);

        let vulkaninfo_ok = Command::new("vulkaninfo")
            .arg("--summary")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if lib_found || has_icd || vulkaninfo_ok {
            return CheckItem {
                passed: true,
                title: "Vulkan GPU".to_string(),
                details:
                    "Vulkan loader and graphics driver detected on system."
                        .to_string(),
                path: None,
                error: None,
            };
        }

        CheckItem {
			passed: false,
			title: "Vulkan GPU".to_string(),
			details: "No Vulkan driver or loader (libvulkan.so.1) found.".to_string(),
			path: None,
			error: Some("A Vulkan-capable GPU driver (Mesa, NVIDIA, AMD) is required to run Horizon's native wgpu renderer.".to_string()),
		}
    }

    #[cfg(target_os = "windows")]
    {
        let sys32_vulkan = Path::new(r"C:\Windows\System32\vulkan-1.dll");
        if sys32_vulkan.is_file() {
            return CheckItem {
                passed: true,
                title: "Vulkan GPU".to_string(),
                details: "Vulkan 1.x runtime found in Windows System32."
                    .to_string(),
                path: Some(sys32_vulkan.to_string_lossy().to_string()),
                error: None,
            };
        }

        CheckItem {
			passed: false,
			title: "Vulkan GPU".to_string(),
			details: "Vulkan runtime (vulkan-1.dll) not found in System32.".to_string(),
			path: None,
			error: Some("Please install or update your graphics driver (NVIDIA, AMD, Intel) to provide Vulkan support.".to_string()),
		}
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        CheckItem {
            passed: true,
            title: "Graphics Hardware".to_string(),
            details: "Assuming compatible native graphics API.".to_string(),
            path: None,
            error: None,
        }
    }
}

pub fn check_system_requirements(
    custom_jar: Option<&Path>,
) -> SystemRequirementsReport {
    let minecraft_26_3 = check_minecraft_26_3(custom_jar);
    let java_25 = check_java_25();
    let gpu_vulkan = check_gpu_compatibility();
    let all_passed =
        minecraft_26_3.passed && java_25.passed && gpu_vulkan.passed;

    SystemRequirementsReport {
        all_passed,
        minecraft_26_3,
        java_25,
        gpu_vulkan,
    }
}
