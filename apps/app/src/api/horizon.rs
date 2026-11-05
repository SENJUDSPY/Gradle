use crate::api::Result;
use horizon_runtime::{
	HorizonRelease, SystemRequirementsReport, check_system_requirements,
	fetch_releases, get_installed_versions, install_version, is_prepared,
	has_sounds, remove_version, run_preparation,
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
pub async fn horizon_prepare_resources(
	app: tauri::AppHandle,
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
pub async fn horizon_get_releases() -> Result<Vec<HorizonRelease>> {
	let releases = fetch_releases().await?;
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
		])
		.build()
}
