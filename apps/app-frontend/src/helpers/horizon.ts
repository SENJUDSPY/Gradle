import { invoke } from '@tauri-apps/api/core'
import { listen, type UnlistenFn } from '@tauri-apps/api/event'

export interface CheckItem {
	passed: boolean
	title: string
	details: string
	path: string | null
	error: string | null
	fix_action?: string | null
	fix_url?: string | null
}

export interface SystemRequirementsReport {
	all_passed: boolean
	minecraft_26_3: CheckItem
	java_25: CheckItem
	gpu_vulkan: CheckItem
	disk_space: CheckItem
	runtime_installed: CheckItem
	resources_prepared: CheckItem
	valid_account: CheckItem
}

export interface DiagnosticsReport {
	requirements: SystemRequirementsReport
	os_info: string
	gpu_info: string
	java_version: string
	installed_horizon_versions: string[]
	resources_manifest: string | null
	horizon_home: string
	generated_at: string
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
	sha256?: string | null
	signature_url?: string | null
}

export interface DownloadProgress {
	tag: string
	downloaded: number
	total: number | null
	percent: number
	speed_bps: number
	stage: 'downloading' | 'verifying' | 'extracting' | 'done'
}

export interface InstalledVersionInfo {
	tag: string
	name: string
	sha256: string | null
	installed_at: string
}

export interface ResourcesManifest {
	schema: number
	minecraft: string
	client_sha1: string
	client_sha256?: string | null
	resource_files: number
	has_sounds: boolean
	prepared_at: string
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

export async function runHorizonDiagnostics(
	customJar?: string | null,
): Promise<DiagnosticsReport> {
	return await invoke('plugin:horizon|horizon_run_diagnostics', {
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
	force?: boolean,
): Promise<string> {
	return await invoke('plugin:horizon|horizon_prepare_resources', {
		customJar: customJar ?? null,
		customMinecraftDir: customMinecraftDir ?? null,
		force: force ?? false,
	})
}

export async function cancelHorizonPreparation(): Promise<void> {
	await invoke('plugin:horizon|horizon_cancel_preparation')
}

export async function getHorizonResourcesManifest(): Promise<ResourcesManifest | null> {
	return await invoke('plugin:horizon|horizon_get_resources_manifest')
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

export async function getInstalledHorizonVersionInfo(
	tag: string,
): Promise<InstalledVersionInfo | null> {
	return await invoke('plugin:horizon|horizon_get_installed_version_info', { tag })
}

export async function onHorizonDownloadProgress(
	callback: (progress: DownloadProgress) => void,
): Promise<UnlistenFn> {
	return await listen<DownloadProgress>('horizon-download-progress', (event) => {
		callback(event.payload)
	})
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

export type ModpackCompatibilityLevel =
	| 'Supported'
	| 'PartiallySupported'
	| 'Unsupported'

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
	compatibility_level: ModpackCompatibilityLevel
	compatibility_summary: string
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

export type DetectedLoader =
	| 'fabric'
	| 'quilt'
	| 'forge'
	| 'neoforge'
	| 'horizonwasm'
	| 'unknown'

export type ModClassificationType =
	| 'RESOURCE_ONLY'
	| 'BUILT_IN_EQUIVALENT'
	| 'WASM_AVAILABLE'
	| 'UNSUPPORTED'

export interface ModClassification {
	type: ModClassificationType
	details: {
		reason: string
		feature?: string
		wasm_id?: string
		assets_found?: string[]
		class_count?: number
		mixin_count?: number
	}
}

export interface ModJarAnalysis {
	file_name: string
	mod_id: string
	name: string
	version: string
	loader: DetectedLoader
	minecraft_version: string | null
	entrypoints: string[]
	mixins: string[]
	dependencies: Record<string, string>
	class_count: number
	is_resource_only: boolean
	classification: ModClassification
}

export interface InstanceModCompatibilityReport {
	total_mods: number
	supported_count: number
	skipped_count: number
	summary: string
	can_launch_native: boolean
	mods: ModJarAnalysis[]
}

export async function classifyInstanceMods(
	instanceId: string,
): Promise<InstanceModCompatibilityReport> {
	return await invoke('plugin:horizon|horizon_classify_instance_mods', {
		instanceId,
	})
}

export async function classifyModFile(
	path: string,
): Promise<ModJarAnalysis> {
	return await invoke('plugin:horizon|horizon_classify_mod_file', {
		path,
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

	allow_ai_agents: boolean

	msaa_samples?: number
	shadow_quality?: string
	texture_filtering?: string
	fullscreen?: boolean
	resource_packs?: string[]
	keybinds?: Record<string, string>
}

export interface AgentClientConfig {
	id: string
	name: string
	target_file: string
	config_snippet: string
	add_command: string | null
}

export interface McpTestReport {
	success: boolean
	protocol_version: string
	server_name: string
	server_version: string
	tool_count: number
	tools: string[]
	message: string
}

export interface AgentControlStatus {
	allow_ai_agents: boolean
	horizon_ctl_installed: boolean
	horizon_ctl_path: string
	configs: AgentClientConfig[]
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

export async function getHorizonAgentControlStatus(
	instancePath?: string | null,
): Promise<AgentControlStatus> {
	return await invoke('plugin:horizon|horizon_get_agent_control_status', {
		instancePath: instancePath ?? null,
	})
}

export async function setHorizonAgentControlEnabled(
	enabled: boolean,
	instancePath?: string | null,
): Promise<HorizonSettings> {
	return await invoke('plugin:horizon|horizon_set_agent_control_enabled', {
		enabled,
		instancePath: instancePath ?? null,
	})
}

export async function testHorizonMcpServer(): Promise<McpTestReport> {
	return await invoke('plugin:horizon|horizon_test_mcp_server')
}

export async function getHorizonAgentConfigs(): Promise<AgentClientConfig[]> {
	return await invoke('plugin:horizon|horizon_get_agent_configs')
}

export interface AssetValidationIssue {
	path: string
	category: string
	reason: string
}

export interface ExtractionSummary {
	mod_id: string
	mod_name: string
	jar_file: string
	total_entries: number
	skipped_classes_count: number
	textures_recovered: number
	total_textures_in_jar: number
	models_recovered: number
	total_models_in_jar: number
	blockstates_recovered: number
	total_blockstates_in_jar: number
	sounds_recovered: number
	total_sounds_in_jar: number
	lang_files_recovered: number
	recipes_recovered: number
	total_recipes_in_jar: number
	loot_tables_recovered: number
	total_loot_tables_in_jar: number
	tags_recovered: number
	other_assets_recovered: number
	other_data_recovered: number
	total_recovered: number
	total_skipped: number
	validation_warnings: AssetValidationIssue[]
	formatted_report: string
	extracted_at: string
}

export interface LoadOrderItem {
	mod_id: string
	mod_name: string
	priority: number
	enabled: boolean
	relative_dir: string
}

export interface AssetOverrideRecord {
	relative_path: string
	winning_mod_id: string
	overridden_mod_id: string
}

export interface MergedAssetsState {
	total_unique_assets: number
	total_unique_data: number
	active_mods: string[]
	override_conflicts: AssetOverrideRecord[]
	last_merged_at: string
}

export async function extractHorizonModAssets(
	instanceId: string,
	jarPath: string,
): Promise<ExtractionSummary> {
	return await invoke('plugin:horizon|horizon_extract_mod_assets', {
		instanceId,
		jarPath,
	})
}

export async function getExtractedContentSummaries(
	instanceId: string,
): Promise<ExtractionSummary[]> {
	return await invoke('plugin:horizon|horizon_get_extracted_content_summaries', {
		instanceId,
	})
}

export async function getHorizonLoadOrder(
	instanceId: string,
): Promise<LoadOrderItem[]> {
	return await invoke('plugin:horizon|horizon_get_load_order', {
		instanceId,
	})
}

export async function updateHorizonLoadOrder(
	instanceId: string,
	orderedModIds: string[],
): Promise<LoadOrderItem[]> {
	return await invoke('plugin:horizon|horizon_update_load_order', {
		instanceId,
		orderedModIds,
	})
}

export async function toggleExtractedMod(
	instanceId: string,
	modId: string,
	enabled: boolean,
): Promise<void> {
	return await invoke('plugin:horizon|horizon_toggle_extracted_mod', {
		instanceId,
		modId,
		enabled,
	})
}

export async function remergeExtractedContent(
	instanceId: string,
): Promise<MergedAssetsState> {
	return await invoke('plugin:horizon|horizon_remerge_extracted_content', {
		instanceId,
	})
}

export interface LicenseVerdict {
	repo_path: string
	license_file_found: boolean
	license_type: any
	can_port: boolean
	requires_operator_override: boolean
	reason: string
}

export interface BlockerFinding {
	file_path: string
	line_number: number
	pattern_name: string
	category: 'DirectBytecodeInjection' | 'UnsafeJvmMemory' | 'DynamicReflection' | 'UnsupportedNativeLibrary' | 'ModApiExtensionNeeded'
	code_snippet: string
	rationale: string
	suggested_action: string
}

export interface PortingBlockerReport {
	repo_root: string
	total_source_files: number
	findings: BlockerFinding[]
	not_portable_count: number
	needs_api_extension_count: number
	is_portable_to_wasm: boolean
}

export interface PortingSessionReport {
	mod_id: string
	mod_name: string
	license_verdict: LicenseVerdict
	blocker_report: PortingBlockerReport
	generated_crate_dir: string | null
	attribution_markdown: string
	behavioral_comparison_markdown: string
	can_compile_wasm: boolean
	summary_message: string
}

export async function runHorizonPortingAssistant(
	repoRoot: string,
	outputDir: string,
	operatorConfirmedPermission?: boolean,
	modIdHint?: string | null,
	modNameHint?: string | null,
	authorHint?: string | null,
): Promise<PortingSessionReport> {
	return await invoke('plugin:horizon|horizon_run_porting_assistant', {
		repoRoot,
		outputDir,
		operatorConfirmedPermission: operatorConfirmedPermission ?? false,
		modIdHint: modIdHint ?? null,
		modNameHint: modNameHint ?? null,
		authorHint: authorHint ?? null,
	})
}
