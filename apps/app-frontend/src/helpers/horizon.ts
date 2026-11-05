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

export async function getHorizonReleases(): Promise<HorizonRelease[]> {
	return await invoke('plugin:horizon|horizon_get_releases')
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
