use crate::api::Result;
use horizon_runtime::{
	analyze_modpack_file, backup_server, check_system_requirements,
	create_server, delete_server, ensure_horizon_ctl,
	extract_compatible_assets_to_dir, fetch_releases_for,
	get_agent_configs, get_agent_control_status, get_installed_versions,
	has_sounds, install_version, is_prepared, list_servers,
	load_horizon_settings, remove_version, run_preparation,
	save_horizon_settings, set_agent_control_enabled, start_server, stop_server,
	test_mcp_server, update_server, AgentClientConfig, AgentControlStatus,
	HorizonRelease, HorizonServerInfo, HorizonSettings, McpTestReport,
	ModpackAnalysis, SystemRequirementsReport,
};
use std::path::{Path, PathBuf};
use tauri::Emitter;

#[tauri::command]
pub async fn horizon_check_requirements(
	custom_jar: Option<String>,
) -> Result<SystemRequirementsReport> {
	let custom_path = custom_jar.map(PathBuf::from);
	let report = tokio::task::spawn_blocking(move || {
		check_system_requirements(custom_path.as_deref())
	})
	.await?;
	Ok(report)
}

#[tauri::command]
pub async fn horizon_is_resources_prepared() -> Result<bool> {
	Ok(tokio::task::spawn_blocking(is_prepared).await?)
}

#[tauri::command]
pub async fn horizon_has_sounds() -> Result<bool> {
	Ok(tokio::task::spawn_blocking(has_sounds).await?)
}

#[tauri::command]
pub async fn horizon_prepare_resources<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	custom_jar: Option<String>,
	custom_minecraft_dir: Option<String>,
) -> Result<String> {
	let jar_path = custom_jar.map(PathBuf::from);
	let mc_path = custom_minecraft_dir.map(PathBuf::from);

	let app_clone = app.clone();
	let res = tokio::task::spawn_blocking(move || {
		run_preparation(jar_path, mc_path, move |prog| {
			let _ = app_clone.emit("horizon-prepare-progress", prog);
		})
	})
	.await??;

	Ok(res.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn horizon_get_releases(
	repo: Option<String>,
) -> Result<Vec<HorizonRelease>> {
	let releases = fetch_releases_for(repo.as_deref()).await?;
	Ok(releases)
}

#[tauri::command]
pub async fn horizon_get_installed_versions() -> Result<Vec<String>> {
	let versions = tokio::task::spawn_blocking(get_installed_versions).await?;
	Ok(versions)
}

#[tauri::command]
pub async fn horizon_install_version(tag: String) -> Result<PathBuf> {
	let path = install_version(&tag).await?;
	Ok(path)
}

#[tauri::command]
pub async fn horizon_remove_version(tag: String) -> Result<()> {
	tokio::task::spawn_blocking(move || remove_version(&tag)).await??;
	Ok(())
}

#[tauri::command]
pub async fn horizon_list_servers() -> Result<Vec<HorizonServerInfo>> {
	Ok(tokio::task::spawn_blocking(list_servers).await??)
}

#[tauri::command]
pub async fn horizon_create_server(
	name: String,
	tag: Option<String>,
	port: Option<u16>,
) -> Result<HorizonServerInfo> {
	Ok(tokio::task::spawn_blocking(move || create_server(&name, tag.as_deref(), port)).await??)
}

#[tauri::command]
pub async fn horizon_start_server(name: String) -> Result<u32> {
	Ok(tokio::task::spawn_blocking(move || start_server(&name)).await??)
}

#[tauri::command]
pub async fn horizon_stop_server(name: String) -> Result<()> {
	Ok(tokio::task::spawn_blocking(move || stop_server(&name)).await??)
}

#[tauri::command]
pub async fn horizon_backup_server(name: String) -> Result<String> {
	let path = tokio::task::spawn_blocking(move || backup_server(&name)).await??;
	Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn horizon_update_server(name: String, new_tag: String) -> Result<()> {
	Ok(tokio::task::spawn_blocking(move || update_server(&name, &new_tag)).await??)
}

#[tauri::command]
pub async fn horizon_delete_server(name: String) -> Result<()> {
	Ok(tokio::task::spawn_blocking(move || delete_server(&name)).await??)
}

#[tauri::command]
pub async fn horizon_get_tailscale_ip() -> Result<Option<String>> {
	let output = tokio::process::Command::new("tailscale")
		.args(["ip", "-4"])
		.output()
		.await;
	match output {
		Ok(out) if out.status.success() => {
			let ip = String::from_utf8_lossy(&out.stdout).trim().to_string();
			if !ip.is_empty() {
				Ok(Some(ip))
			} else {
				Ok(None)
			}
		}
		_ => Ok(None),
	}
}

#[tauri::command]
pub async fn horizon_analyze_modpack(path: String) -> Result<ModpackAnalysis> {
	let pack_path = PathBuf::from(path);
	let analysis = tokio::task::spawn_blocking(move || analyze_modpack_file(&pack_path)).await??;
	Ok(analysis)
}

#[tauri::command]
pub async fn horizon_extract_modpack_assets(
	pack_path: String,
	instance_dir: String,
) -> Result<usize> {
	let p_pack = PathBuf::from(pack_path);
	let p_dest = if Path::new(&instance_dir).is_absolute() {
		PathBuf::from(instance_dir)
	} else if let Ok(state) = theseus::State::get().await {
		state.directories.instances_dir().join(&instance_dir)
	} else {
		PathBuf::from(instance_dir)
	};

	let count = tokio::task::spawn_blocking(move || {
		extract_compatible_assets_to_dir(&p_pack, &p_dest)
	})
	.await??;
	Ok(count)
}

#[tauri::command]
pub async fn horizon_get_settings(
	instance_dir: Option<String>,
) -> Result<HorizonSettings> {
	let path = instance_dir.map(PathBuf::from);
	let settings = tokio::task::spawn_blocking(move || {
		load_horizon_settings(path.as_deref())
	})
	.await?;
	Ok(settings)
}

#[tauri::command]
pub async fn horizon_save_settings(
	instance_dir: Option<String>,
	settings: HorizonSettings,
) -> Result<()> {
	let path = instance_dir.map(PathBuf::from);
	tokio::task::spawn_blocking(move || {
		save_horizon_settings(path.as_deref(), &settings)
	})
	.await??;
	Ok(())
}

#[tauri::command]
pub async fn horizon_get_agent_control_status(
	instance_path: Option<String>,
) -> Result<AgentControlStatus> {
	let path = instance_path.map(PathBuf::from);
	Ok(tokio::task::spawn_blocking(move || {
		get_agent_control_status(path.as_deref())
	})
	.await??)
}

#[tauri::command]
pub async fn horizon_set_agent_control_enabled(
	enabled: bool,
	instance_path: Option<String>,
) -> Result<HorizonSettings> {
	let path = instance_path.map(PathBuf::from);
	Ok(tokio::task::spawn_blocking(move || {
		set_agent_control_enabled(enabled, path.as_deref())
	})
	.await??)
}

#[tauri::command]
pub async fn horizon_test_mcp_server() -> Result<McpTestReport> {
	Ok(tokio::task::spawn_blocking(test_mcp_server).await??)
}

#[tauri::command]
pub async fn horizon_get_agent_configs() -> Result<Vec<AgentClientConfig>> {
	let ctl = tokio::task::spawn_blocking(ensure_horizon_ctl).await??;
	let path_str = ctl.to_string_lossy().to_string();
	Ok(get_agent_configs(&path_str))
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("horizon")
		.invoke_handler(tauri::generate_handler![
			horizon_check_requirements,
			horizon_is_resources_prepared,
			horizon_has_sounds,
			horizon_prepare_resources,
			horizon_get_releases,
			horizon_get_installed_versions,
			horizon_install_version,
			horizon_remove_version,
			horizon_list_servers,
			horizon_create_server,
			horizon_start_server,
			horizon_stop_server,
			horizon_backup_server,
			horizon_update_server,
			horizon_delete_server,
			horizon_get_tailscale_ip,
			horizon_analyze_modpack,
			horizon_extract_modpack_assets,
			horizon_get_settings,
			horizon_save_settings,
			horizon_get_agent_control_status,
			horizon_set_agent_control_enabled,
			horizon_test_mcp_server,
			horizon_get_agent_configs,
		])
		.build()
}
