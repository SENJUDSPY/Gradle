use tauri_build::{AppManifest, InlinedPlugin};

#[path = "src/commands.rs"]
mod commands;

fn main() {
    println!("cargo:rerun-if-env-changed=PROFILE");
    let profile = std::env::var("PROFILE").unwrap_or_default();
    let devtools_cap = std::path::Path::new("capabilities/devtools.json");
    let devtools_perms = if profile == "debug" {
        r#""core:webview:allow-internal-toggle-devtools""#
    } else {
        ""
    };
    let devtools_json = format!(
        r#"{{
	"identifier": "debug-devtools",
	"description": "Developer tools toggle (debug builds only)",
	"local": true,
	"windows": ["main"],
	"permissions": [{devtools_perms}]
}}
"#
    );
    if std::fs::read_to_string(devtools_cap).ok().as_deref() != Some(devtools_json.as_str()) {
        std::fs::write(devtools_cap, devtools_json)
            .expect("failed to generate profile-scoped devtools capability");
    }

    let mut attributes = tauri_build::Attributes::new()
        .codegen(tauri_build::CodegenContext::new())
        .app_manifest(
            AppManifest::new()
                .commands(commands::APP_COMMANDS)
                .permissions_path_pattern("permissions/app.toml"),
        );

    attributes = attributes.plugin(
        "auth",
        InlinedPlugin::new()
            .commands(commands::AUTH_COMMANDS)
            .permissions_path_pattern("permissions/auth.toml"),
    );
    attributes = attributes.plugin(
        "cache",
        InlinedPlugin::new()
            .commands(commands::CACHE_COMMANDS)
            .permissions_path_pattern("permissions/cache.toml"),
    );
    attributes = attributes.plugin(
        "files",
        InlinedPlugin::new()
            .commands(commands::FILES_COMMANDS)
            .permissions_path_pattern("permissions/files.toml"),
    );
    attributes = attributes.plugin(
        "friends",
        InlinedPlugin::new()
            .commands(commands::FRIENDS_COMMANDS)
            .permissions_path_pattern("permissions/friends.toml"),
    );
    attributes = attributes.plugin(
        "horizon",
        InlinedPlugin::new()
            .commands(commands::HORIZON_COMMANDS)
            .permissions_path_pattern("permissions/horizon.toml"),
    );
    attributes = attributes.plugin(
        "import",
        InlinedPlugin::new()
            .commands(commands::IMPORT_COMMANDS)
            .permissions_path_pattern("permissions/import.toml"),
    );
    attributes = attributes.plugin(
        "install",
        InlinedPlugin::new()
            .commands(commands::INSTALL_COMMANDS)
            .permissions_path_pattern("permissions/install.toml"),
    );
    attributes = attributes.plugin(
        "instance",
        InlinedPlugin::new()
            .commands(commands::INSTANCE_COMMANDS)
            .permissions_path_pattern("permissions/instance.toml"),
    );
    attributes = attributes.plugin(
        "jre",
        InlinedPlugin::new()
            .commands(commands::JRE_COMMANDS)
            .permissions_path_pattern("permissions/jre.toml"),
    );
    attributes = attributes.plugin(
        "logs",
        InlinedPlugin::new()
            .commands(commands::LOGS_COMMANDS)
            .permissions_path_pattern("permissions/logs.toml"),
    );
    attributes = attributes.plugin(
        "metadata",
        InlinedPlugin::new()
            .commands(commands::METADATA_COMMANDS)
            .permissions_path_pattern("permissions/metadata.toml"),
    );
    attributes = attributes.plugin(
        "minecraft-skins",
        InlinedPlugin::new()
            .commands(commands::MINECRAFT_SKINS_COMMANDS)
            .permissions_path_pattern("permissions/minecraft-skins.toml"),
    );
    attributes = attributes.plugin(
        "mr-auth",
        InlinedPlugin::new()
            .commands(commands::MR_AUTH_COMMANDS)
            .permissions_path_pattern("permissions/mr-auth.toml"),
    );
    attributes = attributes.plugin(
        "onboarding-checklist",
        InlinedPlugin::new()
            .commands(commands::ONBOARDING_CHECKLIST_COMMANDS)
            .permissions_path_pattern("permissions/onboarding-checklist.toml"),
    );
    attributes = attributes.plugin(
        "p2p",
        InlinedPlugin::new()
            .commands(commands::P2P_COMMANDS)
            .permissions_path_pattern("permissions/p2p.toml"),
    );
    attributes = attributes.plugin(
        "process",
        InlinedPlugin::new()
            .commands(commands::PROCESS_COMMANDS)
            .permissions_path_pattern("permissions/process.toml"),
    );
    attributes = attributes.plugin(
        "reports",
        InlinedPlugin::new()
            .commands(commands::REPORTS_COMMANDS)
            .permissions_path_pattern("permissions/reports.toml"),
    );
    attributes = attributes.plugin(
        "settings",
        InlinedPlugin::new()
            .commands(commands::SETTINGS_COMMANDS)
            .permissions_path_pattern("permissions/settings.toml"),
    );
    attributes = attributes.plugin(
        "shortcuts",
        InlinedPlugin::new()
            .commands(commands::SHORTCUTS_COMMANDS)
            .permissions_path_pattern("permissions/shortcuts.toml"),
    );
    attributes = attributes.plugin(
        "tags",
        InlinedPlugin::new()
            .commands(commands::TAGS_COMMANDS)
            .permissions_path_pattern("permissions/tags.toml"),
    );
    attributes = attributes.plugin(
        "users",
        InlinedPlugin::new()
            .commands(commands::USERS_COMMANDS)
            .permissions_path_pattern("permissions/users.toml"),
    );
    attributes = attributes.plugin(
        "utils",
        InlinedPlugin::new()
            .commands(commands::UTILS_COMMANDS)
            .permissions_path_pattern("permissions/utils.toml"),
    );
    attributes = attributes.plugin(
        "worlds",
        InlinedPlugin::new()
            .commands(commands::WORLDS_COMMANDS)
            .permissions_path_pattern("permissions/worlds.toml"),
    );

    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
