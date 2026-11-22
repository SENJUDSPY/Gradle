use crate::api::Result;
use horizon_runtime::{
	analyze_jar_file, analyze_modpack_file, backup_server, cancel_preparation,
	check_system_requirements, classify_instance_mods_dir, clear_crash, create_server,
	delete_server, ensure_horizon_ctl, extract_compatible_assets_to_dir, fetch_releases_for,
	generate_diagnostics_report, get_agent_configs, get_agent_control_status,
	get_installed_version_info, get_installed_versions, get_latest_crash,
	get_resources_manifest, has_sounds, install_version_with_progress, is_prepared,
	list_servers, load_horizon_settings, remove_version, run_preparation,
	save_horizon_settings, set_agent_control_enabled, start_server, stop_server,
	test_mcp_server, update_server, AgentClientConfig, AgentControlStatus,
	DiagnosticsReport, HorizonCrashReport, HorizonRelease, HorizonServerInfo,
	HorizonSettings, InstalledVersionInfo, InstanceModCompatibilityReport, McpTestReport,
	ModJarAnalysis, ModpackAnalysis, ResourcesManifest, SystemRequirementsReport,
	CatalogEntry, CatalogManager, CatalogRoot, CatalogUpdateResult,
	extract_jar_from_file, get_instance_extracted_summaries, load_load_order,
	update_load_order, toggle_extracted_mod, remerge_extracted_content,
	ExtractionSummary, LoadOrderItem, MergedAssetsState,
	execute_porting_pipeline, PortingSessionReport,
};
use std::path::{Path, PathBuf};
use tauri::Emitter;

async fn get_active_account_name() -> Option<String> {
	let default_uuid = theseus::prelude::minecraft_auth::get_default_user()
		.await
		.ok()
		.flatten();
	let users = theseus::prelude::minecraft_auth::users().await.ok()?;
	if let Some(target_uuid) = default_uuid {
		users
			.into_iter()
			.find(|u| u.offline_profile.id == target_uuid)
			.map(|u| u.offline_profile.name)
	} else {
		users
			.into_iter()
			.find(|u| u.active)
			.map(|u| u.offline_profile.name)
	}
}

#[tauri::command]
pub async fn horizon_check_requirements(
	custom_jar: Option<String>,
) -> Result<SystemRequirementsReport> {
	let custom_path = custom_jar.map(PathBuf::from);
	let account_name = get_active_account_name().await;
	let report = tokio::task::spawn_blocking(move || {
		check_system_requirements(custom_path.as_deref(), account_name.as_deref())
	})
	.await?;
	Ok(report)
}

#[tauri::command]
pub async fn horizon_run_diagnostics(
	custom_jar: Option<String>,
) -> Result<DiagnosticsReport> {
	let custom_path = custom_jar.map(PathBuf::from);
	let account_name = get_active_account_name().await;
	let report = tokio::task::spawn_blocking(move || {
		generate_diagnostics_report(custom_path.as_deref(), account_name.as_deref())
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
	force: Option<bool>,
) -> Result<String> {
	let jar_path = custom_jar.map(PathBuf::from);
	let mc_path = custom_minecraft_dir.map(PathBuf::from);
	let force_val = force.unwrap_or(false);

	let app_clone = app.clone();
	let res = tokio::task::spawn_blocking(move || {
		run_preparation(jar_path, mc_path, force_val, move |prog| {
			let _ = app_clone.emit("horizon-prepare-progress", prog);
		})
	})
	.await??;

	Ok(res.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn horizon_cancel_preparation() -> Result<()> {
	cancel_preparation();
	Ok(())
}

#[tauri::command]
pub async fn horizon_get_resources_manifest() -> Result<Option<ResourcesManifest>> {
	Ok(tokio::task::spawn_blocking(get_resources_manifest).await?)
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
pub async fn horizon_install_version<R: tauri::Runtime>(
	app: tauri::AppHandle<R>,
	tag: String,
) -> Result<PathBuf> {
	let app_clone = app.clone();
	let path = install_version_with_progress(&tag, move |prog| {
		let _ = app_clone.emit("horizon-download-progress", prog);
	})
	.await?;
	Ok(path)
}

#[tauri::command]
pub async fn horizon_get_installed_version_info(
	tag: String,
) -> Result<Option<InstalledVersionInfo>> {
	Ok(tokio::task::spawn_blocking(move || get_installed_version_info(&tag)).await?)
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
pub async fn horizon_classify_instance_mods(
	instance_id: String,
) -> Result<InstanceModCompatibilityReport> {
	let instance_dir = theseus::instance::get_full_path(&instance_id).await?;
	let mods_dir = instance_dir.join("mods");
	let report = tokio::task::spawn_blocking(move || {
		classify_instance_mods_dir(&mods_dir)
	})
	.await??;
	Ok(report)
}

#[tauri::command]
pub async fn horizon_classify_mod_file(
	path: String,
) -> Result<ModJarAnalysis> {
	let p = PathBuf::from(path);
	let analysis = tokio::task::spawn_blocking(move || {
		analyze_jar_file(&p)
	})
	.await??;
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

async fn resolve_instance_path(id_or_path: &str) -> crate::api::Result<PathBuf> {
	if Path::new(id_or_path).is_absolute() {
		Ok(PathBuf::from(id_or_path))
	} else if let Ok(p) = theseus::instance::get_full_path(id_or_path).await {
		Ok(p)
	} else if let Ok(state) = theseus::State::get().await {
		Ok(state.directories.instances_dir().join(id_or_path))
	} else {
		Ok(PathBuf::from(id_or_path))
	}
}

#[tauri::command]
pub async fn horizon_extract_mod_assets(
	instance_id: String,
	jar_path: String,
) -> Result<ExtractionSummary> {
	let instance_dir = resolve_instance_path(&instance_id).await?;
	let p_jar = PathBuf::from(jar_path);
	let summary = tokio::task::spawn_blocking(move || {
		extract_jar_from_file(&p_jar, &instance_dir)
	})
	.await??;
	Ok(summary)
}

#[tauri::command]
pub async fn horizon_get_extracted_content_summaries(
	instance_id: String,
) -> Result<Vec<ExtractionSummary>> {
	let instance_dir = resolve_instance_path(&instance_id).await?;
	let summaries = tokio::task::spawn_blocking(move || {
		get_instance_extracted_summaries(&instance_dir)
	})
	.await??;
	Ok(summaries)
}

#[tauri::command]
pub async fn horizon_get_load_order(
	instance_id: String,
) -> Result<Vec<LoadOrderItem>> {
	let instance_dir = resolve_instance_path(&instance_id).await?;
	let order = tokio::task::spawn_blocking(move || {
		load_load_order(&instance_dir)
	})
	.await??;
	Ok(order)
}

#[tauri::command]
pub async fn horizon_update_load_order(
	instance_id: String,
	ordered_mod_ids: Vec<String>,
) -> Result<Vec<LoadOrderItem>> {
	let instance_dir = resolve_instance_path(&instance_id).await?;
	let new_order = tokio::task::spawn_blocking(move || {
		update_load_order(&instance_dir, &ordered_mod_ids)
	})
	.await??;
	Ok(new_order)
}

#[tauri::command]
pub async fn horizon_toggle_extracted_mod(
	instance_id: String,
	mod_id: String,
	enabled: bool,
) -> Result<()> {
	let instance_dir = resolve_instance_path(&instance_id).await?;
	tokio::task::spawn_blocking(move || {
		toggle_extracted_mod(&instance_dir, &mod_id, enabled)
	})
	.await??;
	Ok(())
}

#[tauri::command]
pub async fn horizon_remerge_extracted_content(
	instance_id: String,
) -> Result<MergedAssetsState> {
	let instance_dir = resolve_instance_path(&instance_id).await?;
	let state = tokio::task::spawn_blocking(move || {
		remerge_extracted_content(&instance_dir)
	})
	.await??;
	Ok(state)
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

#[tauri::command]
pub async fn horizon_get_latest_crash(instance_id: String) -> Result<Option<HorizonCrashReport>> {
	Ok(get_latest_crash(&instance_id))
}

#[tauri::command]
pub async fn horizon_clear_crash(instance_id: String) -> Result<()> {
	clear_crash(&instance_id);
	Ok(())
}

fn get_catalog_manager() -> &'static CatalogManager {
	static MANAGER: std::sync::OnceLock<CatalogManager> = std::sync::OnceLock::new();
	MANAGER.get_or_init(CatalogManager::new)
}

#[tauri::command]
pub async fn horizon_get_mod_catalog() -> Result<CatalogRoot> {
	let manager = get_catalog_manager();
	Ok((*manager.get_catalog()).clone())
}

#[tauri::command]
pub async fn horizon_update_mod_catalog(
	url: Option<String>,
) -> Result<CatalogUpdateResult> {
	let manager = get_catalog_manager();
	let res = manager.update(url.as_deref()).await?;
	Ok(res)
}

#[tauri::command]
pub async fn horizon_lookup_catalog_entry(
	mod_id: String,
) -> Result<Option<CatalogEntry>> {
	let manager = get_catalog_manager();
	Ok(manager.lookup(&mod_id))
}

#[tauri::command]
pub async fn horizon_validate_mod_license(
	mod_id: String,
) -> Result<bool> {
	let manager = get_catalog_manager();
	if let Some(entry) = manager.lookup(&mod_id) {
		CatalogRoot::validate_port_license(&entry)?;
		Ok(true)
	} else {
		Ok(true)
	}
}

#[tauri::command]
pub async fn horizon_run_porting_assistant(
	repo_root: String,
	output_dir: String,
	operator_confirmed_permission: Option<bool>,
	mod_id_hint: Option<String>,
	mod_name_hint: Option<String>,
	author_hint: Option<String>,
) -> Result<PortingSessionReport> {
	let repo_path = PathBuf::from(repo_root);
	let out_path = PathBuf::from(output_dir);
	let operator_confirmed = operator_confirmed_permission.unwrap_or(false);

	let report = tokio::task::spawn_blocking(move || {
		execute_porting_pipeline(
			&repo_path,
			&out_path,
			operator_confirmed,
			mod_id_hint.as_deref(),
			mod_name_hint.as_deref(),
			author_hint.as_deref(),
		)
	})
	.await??;

	Ok(report)
}

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
	tauri::plugin::Builder::new("horizon")
		.invoke_handler(crate::commands::horizon_handler!())
		.build()
}
