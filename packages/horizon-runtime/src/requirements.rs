use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::preparation::is_prepared;
use crate::releases::{get_installed_versions, horizon_data_dir};

pub const MINECRAFT_26_3_VERSION: &str = "26.3";
pub const MINECRAFT_26_3_CLIENT_SHA1: &str =
    "e877b6a07acd633fb3bb475002175cec036e7b87";
pub const MINECRAFT_26_3_SERVER_SHA1: &str =
    "33680f5f2ac32864d6d7cf5e56a705fdb3e05f4c";

/// Minimum free disk space in bytes (2 GiB).
const MIN_FREE_DISK_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckItem {
    pub passed: bool,
    pub title: String,
    pub details: String,
    pub path: Option<String>,
    pub error: Option<String>,
    /// Machine-readable action identifier for fix buttons (e.g. "install_java", "open_url").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_action: Option<String>,
    /// URL to open when the user clicks the fix button.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fix_url: Option<String>,
}

impl CheckItem {
    fn pass(title: &str, details: &str) -> Self {
        Self {
            passed: true,
            title: title.to_string(),
            details: details.to_string(),
            path: None,
            error: None,
            fix_action: None,
            fix_url: None,
        }
    }

    fn fail(title: &str, details: &str, error: &str) -> Self {
        Self {
            passed: false,
            title: title.to_string(),
            details: details.to_string(),
            path: None,
            error: Some(error.to_string()),
            fix_action: None,
            fix_url: None,
        }
    }

    fn with_path(mut self, path: &str) -> Self {
        self.path = Some(path.to_string());
        self
    }

    fn with_fix(mut self, action: &str, url: Option<&str>) -> Self {
        self.fix_action = Some(action.to_string());
        self.fix_url = url.map(|u| u.to_string());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemRequirementsReport {
    pub all_passed: bool,
    pub minecraft_26_3: CheckItem,
    pub java_25: CheckItem,
    pub gpu_vulkan: CheckItem,
    pub disk_space: CheckItem,
    pub runtime_installed: CheckItem,
    pub resources_prepared: CheckItem,
    pub valid_account: CheckItem,
}

/// Full diagnostics export (superset of SystemRequirementsReport).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticsReport {
    pub requirements: SystemRequirementsReport,
    pub os_info: String,
    pub gpu_info: String,
    pub java_version: String,
    pub installed_horizon_versions: Vec<String>,
    pub resources_manifest: Option<String>,
    pub horizon_home: String,
    pub generated_at: String,
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
                    return CheckItem::pass(
                        "Minecraft 26.3 Jar",
                        "Found valid Minecraft Java 26.3 client jar at specified path.",
                    )
                    .with_path(&path.to_string_lossy());
                } else {
                    return CheckItem::fail(
                        "Minecraft 26.3 Jar",
                        &format!("Provided jar SHA-1 ({sha1}) does not match pinned 26.3 client jar ({MINECRAFT_26_3_CLIENT_SHA1})."),
                        "Checksum mismatch: file is not official 26.3 client jar.",
                    )
                    .with_path(&path.to_string_lossy())
                    .with_fix("choose_jar", None);
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
                    return CheckItem::pass(
                        "Minecraft 26.3 Jar",
                        "Found official Minecraft 26.3 client jar in local launcher library.",
                    )
                    .with_path(&candidate.to_string_lossy());
                }
            }
        }
    }

    CheckItem::fail(
        "Minecraft 26.3 Jar",
        "Minecraft Java Edition 26.3 was not found in local launcher directories.",
        "Run Minecraft 26.3 once in your Minecraft launcher to download it, or select your 26.3.jar manually.",
    )
    .with_fix("choose_jar", None)
}

#[cfg(unix)]
pub fn get_available_disk_space(path: &Path) -> Option<u64> {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = CString::new(path.as_os_str().as_bytes()).ok()?;
    let mut stat: libc::statvfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statvfs(c_path.as_ptr(), &mut stat) } == 0 {
        Some(stat.f_bavail as u64 * stat.f_frsize as u64)
    } else {
        None
    }
}

#[cfg(not(unix))]
pub fn get_available_disk_space(_path: &Path) -> Option<u64> {
    None
}

pub fn check_disk_space(target_dir: Option<&Path>) -> CheckItem {
    let dir = target_dir
        .map(PathBuf::from)
        .unwrap_or_else(horizon_data_dir);

    let probe_dir = if dir.exists() {
        dir.clone()
    } else {
        dir.parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."))
    };

    if let Some(free_bytes) = get_available_disk_space(&probe_dir) {
        let free_gb = free_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
        if free_bytes >= MIN_FREE_DISK_BYTES {
            CheckItem::pass(
                "Disk Space",
                &format!("{free_gb:.1} GB available (minimum 2.0 GB required)."),
            )
            .with_path(&probe_dir.to_string_lossy())
        } else {
            let free_mb = free_bytes / (1024 * 1024);
            CheckItem::fail(
                "Disk Space",
                &format!(
                    "Only {free_mb} MB free on partition for {}. At least 2.0 GB required.",
                    probe_dir.display()
                ),
                "Insufficient free disk space for game assets, client binary, and worlds.",
            )
            .with_path(&probe_dir.to_string_lossy())
            .with_fix("open_dir", Some(&probe_dir.to_string_lossy()))
        }
    } else {
        CheckItem::pass(
            "Disk Space",
            "Disk space check skipped (filesystem information unavailable).",
        )
        .with_path(&probe_dir.to_string_lossy())
    }
}

pub fn check_runtime_installed() -> CheckItem {
    let versions = get_installed_versions();
    if !versions.is_empty() {
        CheckItem::pass(
            "Gradle Engine (Rust Edition)",
            &format!("Installed release(s): {}", versions.join(", ")),
        )
    } else {
        CheckItem::fail(
            "Gradle Engine (Rust Edition)",
            "No Horizon runtime engine is installed.",
            "Install a Horizon engine release in Instance Settings or the Runtime Manager.",
        )
        .with_fix("install_runtime", None)
    }
}

pub fn check_resources_prepared() -> CheckItem {
    if is_prepared() {
        let manifest_path = crate::preparation::prepared_resources_dir()
            .join(crate::preparation::MANIFEST_NAME);
        CheckItem::pass(
            "Game Resources",
            "Game resources (textures, models, data, sounds) are prepared and verified.",
        )
        .with_path(&manifest_path.to_string_lossy())
    } else {
        CheckItem::fail(
            "Game Resources",
            "Game resources have not been prepared yet from Minecraft 26.3.",
            "Run one-time resource preparation from your genuine Minecraft 26.3 copy.",
        )
        .with_fix("prepare_resources", None)
    }
}

pub fn check_account(account_name: Option<&str>) -> CheckItem {
    if let Some(name) = account_name {
        if !name.trim().is_empty() {
            return CheckItem::pass(
                "Minecraft Account",
                &format!("Active account: {name}"),
            );
        }
    }
    CheckItem::fail(
        "Minecraft Account",
        "No active Minecraft account found in Gradle.",
        "Sign in to your Minecraft account in Settings or the account switcher to play.",
    )
    .with_fix("login", None)
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
        data.join("gradle/meta/java_versions/25/bin/java"),
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
                let version_first_line =
                    combined.lines().next().unwrap_or("Java 25").trim();
                return CheckItem::pass(
                    "Java 25 (Resource Preparation)",
                    &format!("{version_first_line} available for local game asset preparation."),
                )
                .with_path(&bin.to_string_lossy());
            }
        }
    }

    CheckItem::fail(
        "Java 25 (Resource Preparation)",
        "Java 25 is required only for one-time game file and bytecode preparation from your Minecraft copy.",
        "Install Java 25 (Adoptium Temurin 25) or use Gradle Java Settings to download it.",
    )
    .with_fix(
        "open_url",
        Some("https://adoptium.net/temurin/releases/?version=25"),
    )
}

pub fn check_gpu_compatibility() -> CheckItem {
    #[cfg(target_os = "macos")]
    {
        if cfg!(target_arch = "aarch64") {
            CheckItem::pass(
                "Graphics Hardware",
                "Apple Silicon GPU with native Metal renderer supported.",
            )
        } else {
            CheckItem::pass(
                "Graphics Hardware",
                "Intel Mac with Metal/MoltenVK support.",
            )
        }
    }

    #[cfg(target_os = "linux")]
    {
        if let Ok(output) = Command::new("vulkaninfo").arg("--summary").output()
        {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut device_name = String::new();
                let mut driver_info = String::new();
                let mut api_version = String::new();

                for line in stdout.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("deviceName")
                        && device_name.is_empty()
                    {
                        if let Some(val) = trimmed.split('=').nth(1) {
                            device_name = val.trim().to_string();
                        }
                    } else if trimmed.starts_with("driverInfo")
                        && driver_info.is_empty()
                    {
                        if let Some(val) = trimmed.split('=').nth(1) {
                            driver_info = val.trim().to_string();
                        }
                    } else if trimmed.starts_with("apiVersion")
                        && api_version.is_empty()
                    {
                        if let Some(val) = trimmed.split('=').nth(1) {
                            api_version = val.trim().to_string();
                        }
                    }
                }

                let summary = if !device_name.is_empty() {
                    let driver_str = if !driver_info.is_empty() {
                        format!(" ({driver_info})")
                    } else {
                        String::new()
                    };
                    let api_str = if !api_version.is_empty() {
                        format!("Vulkan {api_version}: ")
                    } else {
                        String::new()
                    };
                    format!("{api_str}{device_name}{driver_str}")
                } else {
                    "Vulkan loader and compatible GPU driver detected."
                        .to_string()
                };

                return CheckItem::pass("Vulkan GPU", &summary);
            }
        }

        let common_vulkan_libs = [
            "/usr/lib/x86_64-linux-gnu/libvulkan.so.1",
            "/usr/lib64/libvulkan.so.1",
            "/usr/lib/libvulkan.so.1",
            "/lib/x86_64-linux-gnu/libvulkan.so.1",
        ];
        let lib_found =
            common_vulkan_libs.iter().find(|p| Path::new(p).exists());

        let icd_dir = Path::new("/usr/share/vulkan/icd.d");
        let has_icd = icd_dir.is_dir()
            && fs::read_dir(icd_dir)
                .map(|mut d| d.next().is_some())
                .unwrap_or(false);

        if let Some(lib_path) = lib_found {
            return CheckItem::pass(
                "Vulkan GPU",
                "Vulkan loader (libvulkan.so.1) and ICD driver manifests detected on system.",
            )
            .with_path(lib_path);
        } else if has_icd {
            return CheckItem::pass(
                "Vulkan GPU",
                "Vulkan ICD drivers found in /usr/share/vulkan/icd.d.",
            );
        }

        CheckItem::fail(
            "Vulkan GPU",
            "No Vulkan driver or loader (libvulkan.so.1) detected on your system.",
            "A Vulkan 1.2+ capable GPU driver (Mesa, NVIDIA, AMD) is required to run the native wgpu renderer.",
        )
        .with_fix("open_url", Some("https://www.vulkan.org/"))
    }

    #[cfg(target_os = "windows")]
    {
        let sys32_vulkan = Path::new(r"C:\Windows\System32\vulkan-1.dll");
        if sys32_vulkan.is_file() {
            CheckItem::pass(
                "Vulkan GPU",
                "Vulkan 1.x runtime found in Windows System32.",
            )
            .with_path(&sys32_vulkan.to_string_lossy())
        } else {
            CheckItem::fail(
                "Vulkan GPU",
                "Vulkan runtime (vulkan-1.dll) not found in System32.",
                "Please install or update your graphics driver (NVIDIA, AMD, Intel) to provide Vulkan support.",
            )
            .with_fix("open_url", Some("https://www.vulkan.org/"))
        }
    }

    #[cfg(not(any(
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        CheckItem::pass(
            "Graphics Hardware",
            "Assuming compatible native graphics API.",
        )
    }
}

pub fn check_system_requirements(
    custom_jar: Option<&Path>,
    account_name: Option<&str>,
) -> SystemRequirementsReport {
    let minecraft_26_3 = check_minecraft_26_3(custom_jar);
    let java_25 = check_java_25();
    let gpu_vulkan = check_gpu_compatibility();
    let disk_space = check_disk_space(None);
    let runtime_installed = check_runtime_installed();
    let resources_prepared = check_resources_prepared();
    let valid_account = check_account(account_name);

    // Java 25 and client jar are only strictly required if game resources have NOT been prepared yet
    let java_ok = resources_prepared.passed || java_25.passed;
    let minecraft_ok = resources_prepared.passed || minecraft_26_3.passed;

    let all_passed = gpu_vulkan.passed
        && disk_space.passed
        && runtime_installed.passed
        && resources_prepared.passed
        && valid_account.passed
        && java_ok
        && minecraft_ok;

    SystemRequirementsReport {
        all_passed,
        minecraft_26_3,
        java_25,
        gpu_vulkan,
        disk_space,
        runtime_installed,
        resources_prepared,
        valid_account,
    }
}

pub fn generate_diagnostics_report(
    custom_jar: Option<&Path>,
    account_name: Option<&str>,
) -> DiagnosticsReport {
    let reqs = check_system_requirements(custom_jar, account_name);

    let os_info = if let Ok(content) = fs::read_to_string("/etc/os-release") {
        content
            .lines()
            .find(|l| l.starts_with("PRETTY_NAME="))
            .map(|l| {
                l.trim_start_matches("PRETTY_NAME=")
                    .trim_matches('"')
                    .to_string()
            })
            .unwrap_or_else(|| std::env::consts::OS.to_string())
    } else {
        std::env::consts::OS.to_string()
    };

    let gpu_info = Command::new("vulkaninfo")
        .arg("--summary")
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_else(|_| "vulkaninfo unavailable".to_string());

    let java_version = reqs
        .java_25
        .path
        .as_deref()
        .and_then(|p| Command::new(p).arg("-version").output().ok())
        .map(|o| {
            format!(
                "{}\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            )
            .trim()
            .to_string()
        })
        .unwrap_or_else(|| "Not installed".to_string());

    let manifest_path = crate::preparation::prepared_resources_dir()
        .join(crate::preparation::MANIFEST_NAME);
    let resources_manifest = fs::read_to_string(&manifest_path).ok();

    DiagnosticsReport {
        requirements: reqs,
        os_info,
        gpu_info,
        java_version,
        installed_horizon_versions: get_installed_versions(),
        resources_manifest,
        horizon_home: horizon_data_dir().to_string_lossy().to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
    }
}
