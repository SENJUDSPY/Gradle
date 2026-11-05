import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface CheckItem {
	passed: boolean
	title: string
	details: string
	path: string | null
	error: string | null
}

export interface SystemRequirementsReport {
	all_passed: boolean
	minecraft_26_3: CheckItem
	java_25: CheckItem
	gpu_vulkan: CheckItem
}

export interface HorizonRelease {
	tag: string
	name: string
	body: string
	published_at: string
	prerelease: boolean
	asset_url: string | null
	asset_size: number
	installed: boolean
}

export interface PreparationProgress {
	stage: string
	details: string
	percent: number
	finished: boolean
	error: string | null
}

export async function checkHorizonRequirements(
	customJar?: string | null,
): Promise<SystemRequirementsReport> {
	return await invoke('plugin:horizon|horizon_check_requirements', {
		customJar: customJar ?? null,
	})
}

export async function isHorizonResourcesPrepared(): Promise<boolean> {
	return await invoke('plugin:horizon|horizon_is_resources_prepared')
}

export async function hasHorizonSounds(): Promise<boolean> {
	return await invoke('plugin:horizon|horizon_has_sounds')
}

export async function prepareHorizonResources(
	customJar?: string | null,
	customMinecraftDir?: string | null,
): Promise<string> {
	return await invoke('plugin:horizon|horizon_prepare_resources', {
		customJar: customJar ?? null,
		customMinecraftDir: customMinecraftDir ?? null,
	})
}

export async function onHorizonPrepareProgress(
	callback: (progress: PreparationProgress) => void,
): Promise<UnlistenFn> {
	return await listen<PreparationProgress>('horizon-prepare-progress', (event) => {
		callback(event.payload)
	})
}

export async function getHorizonReleases(repo?: string | null): Promise<HorizonRelease[]> {
	return await invoke('plugin:horizon|horizon_get_releases', { repo: repo ?? null })
}

export async function getInstalledHorizonVersions(): Promise<string[]> {
	return await invoke('plugin:horizon|horizon_get_installed_versions')
}

export async function installHorizonVersion(tag: string): Promise<string> {
	return await invoke('plugin:horizon|horizon_install_version', { tag })
}

export async function removeHorizonVersion(tag: string): Promise<void> {
	return await invoke('plugin:horizon|horizon_remove_version', { tag })
}

export interface HorizonServerConfig {
	name: string
	tag: string
	port: number
	motd: string
	max_players: number
	auto_backup: boolean
	tailscale_enabled: boolean
}

export interface HorizonServerInfo {
	name: string
	path: string
	config: HorizonServerConfig
	is_running: boolean
	pid: number | null
	backup_count: number
}

export async function listHorizonServers(): Promise<HorizonServerInfo[]> {
	return await invoke('plugin:horizon|horizon_list_servers')
}

export async function createHorizonServer(
	name: string,
	tag?: string | null,
	port?: number | null,
): Promise<HorizonServerInfo> {
	return await invoke('plugin:horizon|horizon_create_server', {
		name,
		tag: tag ?? null,
		port: port ?? null,
	})
}

export async function startHorizonServer(name: string): Promise<number> {
	return await invoke('plugin:horizon|horizon_start_server', { name })
}

export async function stopHorizonServer(name: string): Promise<void> {
	await invoke('plugin:horizon|horizon_stop_server', { name })
}

export async function backupHorizonServer(name: string): Promise<string> {
	return await invoke('plugin:horizon|horizon_backup_server', { name })
}

export async function updateHorizonServer(name: string, newTag: string): Promise<void> {
	await invoke('plugin:horizon|horizon_update_server', { name, newTag })
}

export async function deleteHorizonServer(name: string): Promise<void> {
	await invoke('plugin:horizon|horizon_delete_server', { name })
}

export async function getTailscaleIp(): Promise<string | null> {
	return await invoke('plugin:horizon|horizon_get_tailscale_ip')
}

export type ModpackItemCategory =
	| 'ResourcePack'
	| 'DataPack'
	| 'ShaderPack'
	| 'Configuration'
	| 'AssetMod'
	| 'JavaBytecodeMod'
	| 'Other'

export interface ModpackAnalyzedItem {
	name: string
	path: string
	category: ModpackItemCategory
	is_compatible: boolean
	file_size: number | null
	details: string
}

export interface ModpackAnalysis {
	pack_format: string
	pack_name: string
	pack_version: string
	author: string | null
	summary: string | null
	original_game_version: string
	original_loader: string
	total_items: number
	compatible_count: number
	incompatible_count: number
	items: ModpackAnalyzedItem[]
}

export async function analyzeModpackFile(path: string): Promise<ModpackAnalysis> {
	return await invoke('plugin:horizon|horizon_analyze_modpack', { path })
}

export async function extractModpackAssets(packPath: string, instanceDir: string): Promise<number> {
	return await invoke('plugin:horizon|horizon_extract_modpack_assets', {
		packPath,
		instanceDir,
	})
}

export interface HorizonSettings {
	graphics_preset: string
	render_distance: number
	lod_distance: number
	cave_culling: boolean
	entity_culling: boolean
	vsync: boolean
	max_fps: number
	fov: number
	graphics_api: string

	controller_enabled: boolean
	vibration: boolean
	adaptive_triggers: boolean
	lightbar: string
	look_sensitivity: number
	invert_look_y: boolean
	move_deadzone: number
	look_deadzone: number
	button_glyphs: string
	menu_navigation: string

	zoom_enabled: boolean
	minimap_enabled: boolean
	performance_overlay: string
	battery_mode: boolean
}

export async function getHorizonSettings(instanceDir?: string): Promise<HorizonSettings> {
	return await invoke('plugin:horizon|horizon_get_settings', {
		instanceDir: instanceDir ?? null,
	})
}

export async function saveHorizonSettings(
	settings: HorizonSettings,
	instanceDir?: string,
): Promise<void> {
	return await invoke('plugin:horizon|horizon_save_settings', {
		instanceDir: instanceDir ?? null,
		settings,
	})
}
