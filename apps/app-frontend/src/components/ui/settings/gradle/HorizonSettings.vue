<script setup lang="ts">
import {
	BotIcon,
	CheckIcon,
	CompassIcon,
	CopyIcon,
	GameIcon,
	SparklesIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import {
	Button,
	defineMessages,
	injectNotificationManager,
	Slider,
	Toggle,
	useSavable,
	useVIntl,
} from '@modrinth/ui'
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'

import GradleSettingsPage from '@/components/ui/settings/gradle/GradleSettingsPage.vue'
import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'
import {
	type AgentControlStatus,
	getHorizonAgentControlStatus,
	getHorizonSettings,
	type HorizonSettings,
	type McpTestReport,
	saveHorizonSettings,
	testHorizonMcpServer,
} from '@/helpers/horizon'

const settingsModal = injectAppSettingsModal()
const { formatMessage } = useVIntl()
const { handleError } = injectNotificationManager()

const messages = defineMessages({
	pageTitle: {
		id: 'gradle.app.settings.horizon.title',
		defaultMessage: 'Gradle Engine',
	},
	pageDescription: {
		id: 'gradle.app.settings.horizon.description',
		defaultMessage:
			'Configure the native Gradle Rust game client: graphics presets, distant terrain LOD up to 1024 chunks, gamepad haptics, and gameplay extras.',
	},
	graphicsSectionTitle: {
		id: 'gradle.app.settings.horizon.graphics.title',
		defaultMessage: 'Graphics & Distant Terrain',
	},
	graphicsSectionDesc: {
		id: 'gradle.app.settings.horizon.graphics.desc',
		defaultMessage:
			'Gradle renders Minecraft 26.3 natively with wgpu/Vulkan shaders. Distant terrain extends local worlds far beyond vanilla limits.',
	},
	graphicsPresetTitle: {
		id: 'gradle.app.settings.horizon.preset.title',
		defaultMessage: 'Graphics Preset',
	},
	presetPerformance: {
		id: 'gradle.app.settings.horizon.preset.performance',
		defaultMessage: 'Performance',
	},
	presetBalanced: {
		id: 'gradle.app.settings.horizon.preset.balanced',
		defaultMessage: 'Balanced',
	},
	presetQuality: {
		id: 'gradle.app.settings.horizon.preset.quality',
		defaultMessage: 'Quality',
	},
	presetUltra: {
		id: 'gradle.app.settings.horizon.preset.ultra',
		defaultMessage: 'Ultra',
	},
	renderDistanceTitle: {
		id: 'gradle.app.settings.horizon.render-distance.title',
		defaultMessage: 'Full Detail Render Distance',
	},
	lodDistanceTitle: {
		id: 'gradle.app.settings.horizon.lod-distance.title',
		defaultMessage: 'Distant Terrain LOD (Up to 1024 chunks)',
	},
	caveCullingTitle: {
		id: 'gradle.app.settings.horizon.cave-culling.title',
		defaultMessage: 'Cave Culling',
	},
	caveCullingDesc: {
		id: 'gradle.app.settings.horizon.cave-culling.desc',
		defaultMessage: 'Skips drawing hidden underground caves when viewing surface terrain.',
	},
	entityCullingTitle: {
		id: 'gradle.app.settings.horizon.entity-culling.title',
		defaultMessage: 'Entity Culling',
	},
	entityCullingDesc: {
		id: 'gradle.app.settings.horizon.entity-culling.desc',
		defaultMessage: 'Hides mob models behind opaque blocks to maximize frame rates.',
	},
	vsyncTitle: {
		id: 'gradle.app.settings.horizon.vsync.title',
		defaultMessage: 'VSync',
	},
	maxFpsTitle: {
		id: 'gradle.app.settings.horizon.max-fps.title',
		defaultMessage: 'Max Frame Rate',
	},
	unlimitedFps: {
		id: 'gradle.app.settings.horizon.unlimited-fps',
		defaultMessage: 'Unlimited',
	},
	controllerSectionTitle: {
		id: 'gradle.app.settings.horizon.controller.title',
		defaultMessage: 'Controller & Haptics',
	},
	controllerSectionDesc: {
		id: 'gradle.app.settings.horizon.controller.desc',
		defaultMessage:
			'Built-in controller support for Xbox, PlayStation DualSense (adaptive triggers, haptics), and Nintendo Switch pads.',
	},
	controllerEnabledTitle: {
		id: 'gradle.app.settings.horizon.controller-enabled.title',
		defaultMessage: 'Enable Controller Support',
	},
	vibrationTitle: {
		id: 'gradle.app.settings.horizon.vibration.title',
		defaultMessage: 'Vibration & Rumble',
	},
	adaptiveTriggersTitle: {
		id: 'gradle.app.settings.horizon.adaptive-triggers.title',
		defaultMessage: 'DualSense Adaptive Triggers',
	},
	lookSensitivityTitle: {
		id: 'gradle.app.settings.horizon.look-sensitivity.title',
		defaultMessage: 'Look Sensitivity',
	},
	invertLookYTitle: {
		id: 'gradle.app.settings.horizon.invert-y.title',
		defaultMessage: 'Invert Vertical Look (Y Axis)',
	},
	deadzoneTitle: {
		id: 'gradle.app.settings.horizon.deadzone.title',
		defaultMessage: 'Stick Deadzone',
	},
	buttonGlyphsTitle: {
		id: 'gradle.app.settings.horizon.glyphs.title',
		defaultMessage: 'Button Prompts',
	},
	menuNavTitle: {
		id: 'gradle.app.settings.horizon.menu-nav.title',
		defaultMessage: 'Menu Navigation Mode',
	},
	extrasSectionTitle: {
		id: 'gradle.app.settings.horizon.extras.title',
		defaultMessage: 'Gradle Extras',
	},
	extrasSectionDesc: {
		id: 'gradle.app.settings.horizon.extras.desc',
		defaultMessage:
			'Optional native additions engineered directly into the Rust game client with zero mod overhead.',
	},
	zoomTitle: {
		id: 'gradle.app.settings.horizon.zoom.title',
		defaultMessage: 'Camera Zoom (C Key)',
	},
	zoomDesc: {
		id: 'gradle.app.settings.horizon.zoom.desc',
		defaultMessage: 'Smooth native cinematic zoom with smooth mouse deceleration.',
	},
	minimapTitle: {
		id: 'gradle.app.settings.horizon.minimap.title',
		defaultMessage: 'In-Game Minimap',
	},
	minimapDesc: {
		id: 'gradle.app.settings.horizon.minimap.desc',
		defaultMessage: 'Real-time biome, elevation, and waypoint radar overlay.',
	},
	overlayTitle: {
		id: 'gradle.app.settings.horizon.overlay.title',
		defaultMessage: 'Performance Overlay',
	},
	batteryModeTitle: {
		id: 'gradle.app.settings.horizon.battery.title',
		defaultMessage: 'Battery Saver / Efficient Mode',
	},
	batteryModeDesc: {
		id: 'gradle.app.settings.horizon.battery.desc',
		defaultMessage:
			'Lowers power draw on laptops and handhelds by throttling background frames and thread workers.',
	},
	agentSectionTitle: {
		id: 'gradle.app.settings.horizon.agent.title',
		defaultMessage: 'AI Agent Control & MCP Server',
	},
	agentSectionDesc: {
		id: 'gradle.app.settings.horizon.agent.desc',
		defaultMessage:
			'Expose Gradle as a Model Context Protocol (MCP) server so AI coding assistants (Claude Code, Cursor, Codex) can observe the world and play the game.',
	},
	allowAiAgentsTitle: {
		id: 'gradle.app.settings.horizon.agent.allow-ai-agents.title',
		defaultMessage: 'Allow AI agents',
	},
	allowAiAgentsDesc: {
		id: 'gradle.app.settings.horizon.agent.allow-ai-agents.desc',
		defaultMessage:
			'Off by default. When enabled, Gradle starts with a local control socket and horizon-ctl exposes 80+ tools over stdio JSON-RPC.',
	},
	ctlInstalled: {
		id: 'gradle.app.settings.horizon.agent.ctl-installed',
		defaultMessage: 'Installed & Ready',
	},
	ctlMissing: {
		id: 'gradle.app.settings.horizon.agent.ctl-missing',
		defaultMessage: 'Missing binary',
	},
	testMcpBtn: {
		id: 'gradle.app.settings.horizon.agent.test-mcp-btn',
		defaultMessage: 'Test MCP Server',
	},
	connectAgentTitle: {
		id: 'gradle.app.settings.horizon.agent.connect.title',
		defaultMessage: 'Connect an AI Agent',
	},
	connectAgentDesc: {
		id: 'gradle.app.settings.horizon.agent.connect.desc',
		defaultMessage: 'Add the Gradle MCP server to your AI assistant configuration.',
	},
	oneClickCli: {
		id: 'gradle.app.settings.horizon.agent.one-click-cli',
		defaultMessage: 'Terminal command (one-step add)',
	},
	copyConfig: {
		id: 'gradle.app.settings.horizon.agent.copy-config',
		defaultMessage: 'Copy Configuration',
	},
	copyCommand: {
		id: 'gradle.app.settings.horizon.agent.copy-command',
		defaultMessage: 'Copy Command',
	},
	copied: {
		id: 'gradle.app.settings.horizon.agent.copied',
		defaultMessage: 'Copied!',
	},
})

const initialLoaded = ref(false)
const rawSettings = ref<HorizonSettings>({
	graphics_preset: 'balanced',
	render_distance: 12,
	lod_distance: 128,
	cave_culling: true,
	entity_culling: true,
	vsync: true,
	max_fps: 120,
	fov: 70,
	graphics_api: 'vulkan',

	controller_enabled: true,
	vibration: true,
	adaptive_triggers: true,
	lightbar: 'health',
	look_sensitivity: 1.0,
	invert_look_y: false,
	move_deadzone: 0.15,
	look_deadzone: 0.15,
	button_glyphs: 'auto',
	menu_navigation: 'focus',

	zoom_enabled: true,
	minimap_enabled: false,
	performance_overlay: 'none',
	battery_mode: false,
	allow_ai_agents: false,
})

const { saved, current, changes, saving, hasChanges, reset, save } = useSavable(
	() => JSON.parse(JSON.stringify(rawSettings.value)) as HorizonSettings,
	async () => {
		await saveHorizonSettings(current.value)
		rawSettings.value = JSON.parse(JSON.stringify(current.value)) as HorizonSettings
	},
)

async function loadData() {
	try {
		const s = await getHorizonSettings()
		rawSettings.value = s
		reset()
		initialLoaded.value = true
		void loadAgentStatus()
	} catch (e: unknown) {
		handleError(e)
	}
}

async function handleSaveSettings(): Promise<void> {
	try {
		await save()
	} catch (error) {
		handleError(error)
	}
}

onMounted(() => {
	loadData()
	settingsModal.registerUnsavedChangesController({
		hasChanges: () => hasChanges.value,
		getOriginal: () => saved.value,
		getModified: () => changes.value,
		isSaving: () => saving.value,
		reset,
		save: handleSaveSettings,
	})
})

onBeforeUnmount(() => {
	settingsModal.registerUnsavedChangesController(null)
})

const presets = ['performance', 'balanced', 'quality', 'ultra'] as const
const buttonGlyphOptions = ['auto', 'xbox', 'playstation', 'nintendo'] as const
const menuNavOptions = ['focus', 'pointer'] as const
const overlayOptions = ['none', 'fps', 'fps_and_graph'] as const

const maxFpsDisplay = computed(() => {
	return current.value.max_fps >= 260
		? formatMessage(messages.unlimitedFps)
		: `${current.value.max_fps} FPS`
})

const lodDistanceDisplay = computed(() => {
	return `${current.value.lod_distance} chunks (${Math.round((current.value.lod_distance * 16) / 1000)} km)`
})

const agentStatus = ref<AgentControlStatus | null>(null)
const testingMcp = ref(false)
const mcpTestReport = ref<McpTestReport | null>(null)
const mcpTestError = ref<string | null>(null)
const selectedClientId = ref<string>('claude-code')
const copiedField = ref<'config' | 'command' | null>(null)

const agentConfigs = computed(() => agentStatus.value?.configs ?? [])
const selectedClient = computed(() => {
	return (
		agentConfigs.value.find((c) => c.id === selectedClientId.value) ?? agentConfigs.value[0] ?? null
	)
})

async function loadAgentStatus() {
	try {
		agentStatus.value = await getHorizonAgentControlStatus()
	} catch (e: unknown) {
		console.error('Failed to load agent control status:', e)
	}
}

async function handleTestMcp() {
	testingMcp.value = true
	mcpTestReport.value = null
	mcpTestError.value = null
	try {
		const rep = await testHorizonMcpServer()
		mcpTestReport.value = rep
	} catch (e: unknown) {
		mcpTestError.value = e instanceof Error ? e.message : String(e)
	} finally {
		testingMcp.value = false
	}
}

async function copyText(text: string, field: 'config' | 'command') {
	await navigator.clipboard.writeText(text)
	copiedField.value = field
	setTimeout(() => {
		if (copiedField.value === field) {
			copiedField.value = null
		}
	}, 2000)
}

function toggleAgentControl(val: boolean) {
	current.value.allow_ai_agents = val
	if (val && !agentStatus.value) {
		void loadAgentStatus()
	}
}
</script>

<template>
	<GradleSettingsPage
		:title="formatMessage(messages.pageTitle)"
		:description="formatMessage(messages.pageDescription)"
	>
		<!-- Section 1: Graphics & Distant Terrain (LOD) -->
		<section
			class="glass-surface glass-surface--strong rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5 flex flex-col gap-5"
		>
			<div class="flex items-start gap-3">
				<div
					class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand/10 text-brand"
				>
					<SparklesIcon class="size-5" />
				</div>
				<div>
					<div class="flex flex-wrap items-center gap-2">
						<h2 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.graphicsSectionTitle) }}
						</h2>
						<span
							class="shrink-0 rounded-full bg-brand/10 px-2 py-0.5 text-xs font-bold text-brand"
						>
							wgpu / Vulkan
						</span>
					</div>
					<p class="m-0 mt-1 text-sm text-secondary">
						{{ formatMessage(messages.graphicsSectionDesc) }}
					</p>
				</div>
			</div>

			<!-- Preset buttons -->
			<div class="flex flex-col gap-2 pt-2 border-t border-solid border-surface-4">
				<span class="text-sm font-semibold text-contrast">
					{{ formatMessage(messages.graphicsPresetTitle) }}
				</span>
				<div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
					<button
						v-for="preset in presets"
						:key="preset"
						type="button"
						:class="[
							'rounded-xl px-3 py-2 text-xs font-bold capitalize transition-all border border-solid',
							current.graphics_preset === preset
								? 'bg-brand text-black border-brand shadow'
								: 'bg-surface-2 text-secondary border-surface-5 hover:border-surface-4 hover:text-contrast',
						]"
						:disabled="saving"
						@click="current.graphics_preset = preset"
					>
						{{ preset }}
					</button>
				</div>
			</div>

			<!-- Full Detail Render Distance -->
			<div class="flex flex-col gap-2">
				<div class="flex items-center justify-between text-sm">
					<span class="font-semibold text-contrast">
						{{ formatMessage(messages.renderDistanceTitle) }}
					</span>
					<span class="font-mono text-secondary">{{ current.render_distance }} chunks</span>
				</div>
				<Slider v-model="current.render_distance" :min="2" :max="64" :step="1" :disabled="saving" />
			</div>

			<!-- Distant Terrain LOD Distance -->
			<div class="flex flex-col gap-2">
				<div class="flex items-center justify-between text-sm">
					<span class="font-semibold text-contrast">
						{{ formatMessage(messages.lodDistanceTitle) }}
					</span>
					<span class="font-mono text-brand font-semibold">{{ lodDistanceDisplay }}</span>
				</div>
				<Slider
					v-model="current.lod_distance"
					:min="32"
					:max="1024"
					:step="16"
					:disabled="saving"
				/>
			</div>

			<!-- Max FPS -->
			<div class="flex flex-col gap-2">
				<div class="flex items-center justify-between text-sm">
					<span class="font-semibold text-contrast">
						{{ formatMessage(messages.maxFpsTitle) }}
					</span>
					<span class="font-mono text-secondary">{{ maxFpsDisplay }}</span>
				</div>
				<Slider v-model="current.max_fps" :min="30" :max="260" :step="10" :disabled="saving" />
			</div>

			<!-- Toggles: Cave Culling, Entity Culling, VSync -->
			<div
				class="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2 border-t border-solid border-surface-4"
			>
				<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
					<div class="flex flex-col">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.caveCullingTitle)
						}}</span>
						<span class="text-[11px] text-secondary">{{
							formatMessage(messages.caveCullingDesc)
						}}</span>
					</div>
					<Toggle
						id="horizon-cave-culling"
						:model-value="current.cave_culling"
						:disabled="saving"
						@update:model-value="current.cave_culling = $event ?? true"
					/>
				</div>

				<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
					<div class="flex flex-col">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.entityCullingTitle)
						}}</span>
						<span class="text-[11px] text-secondary">{{
							formatMessage(messages.entityCullingDesc)
						}}</span>
					</div>
					<Toggle
						id="horizon-entity-culling"
						:model-value="current.entity_culling"
						:disabled="saving"
						@update:model-value="current.entity_culling = $event ?? true"
					/>
				</div>

				<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
					<div class="flex flex-col">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.vsyncTitle)
						}}</span>
						<span class="text-[11px] text-secondary">Refresh rate pacing</span>
					</div>
					<Toggle
						id="horizon-vsync"
						:model-value="current.vsync"
						:disabled="saving"
						@update:model-value="current.vsync = $event ?? true"
					/>
				</div>
			</div>
		</section>

		<!-- Section 2: Controller & Haptics -->
		<section
			class="glass-surface glass-surface--strong rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5 flex flex-col gap-5 mt-4"
		>
			<div class="flex items-start gap-3">
				<div
					class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand/10 text-brand"
				>
					<GameIcon class="size-5" />
				</div>
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.controllerSectionTitle) }}
					</h2>
					<p class="m-0 mt-1 text-sm text-secondary">
						{{ formatMessage(messages.controllerSectionDesc) }}
					</p>
				</div>
			</div>

			<div
				class="flex items-center justify-between gap-4 pt-2 border-t border-solid border-surface-4"
			>
				<div class="flex flex-col">
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.controllerEnabledTitle) }}
					</span>
					<span class="text-xs text-secondary">Enable native gamepad input</span>
				</div>
				<Toggle
					id="horizon-controller-enabled"
					:model-value="current.controller_enabled"
					:disabled="saving"
					@update:model-value="current.controller_enabled = $event ?? true"
				/>
			</div>

			<div v-if="current.controller_enabled" class="flex flex-col gap-4">
				<!-- Toggles: Vibration & DualSense Adaptive Triggers -->
				<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
					<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.vibrationTitle)
						}}</span>
						<Toggle
							id="horizon-vibration"
							:model-value="current.vibration"
							:disabled="saving"
							@update:model-value="current.vibration = $event ?? true"
						/>
					</div>

					<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.adaptiveTriggersTitle)
						}}</span>
						<Toggle
							id="horizon-adaptive-triggers"
							:model-value="current.adaptive_triggers"
							:disabled="saving"
							@update:model-value="current.adaptive_triggers = $event ?? true"
						/>
					</div>
				</div>

				<!-- Look Sensitivity -->
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-sm">
						<span class="font-semibold text-contrast">{{
							formatMessage(messages.lookSensitivityTitle)
						}}</span>
						<span class="font-mono text-secondary">{{ current.look_sensitivity.toFixed(1) }}x</span>
					</div>
					<Slider
						v-model="current.look_sensitivity"
						:min="0.2"
						:max="3.0"
						:step="0.1"
						:disabled="saving"
					/>
				</div>

				<!-- Button Glyphs -->
				<div class="flex flex-col gap-2">
					<span class="text-sm font-semibold text-contrast">{{
						formatMessage(messages.buttonGlyphsTitle)
					}}</span>
					<div class="grid grid-cols-2 sm:grid-cols-4 gap-2">
						<button
							v-for="opt in buttonGlyphOptions"
							:key="opt"
							type="button"
							:class="[
								'rounded-xl px-3 py-2 text-xs font-bold uppercase transition-all border border-solid',
								current.button_glyphs === opt
									? 'bg-brand text-black border-brand shadow'
									: 'bg-surface-2 text-secondary border-surface-5 hover:border-surface-4 hover:text-contrast',
							]"
							:disabled="saving"
							@click="current.button_glyphs = opt"
						>
							{{ opt }}
						</button>
					</div>
				</div>

				<!-- Menu Navigation Mode -->
				<div class="flex flex-col gap-2">
					<span class="text-sm font-semibold text-contrast">{{
						formatMessage(messages.menuNavTitle)
					}}</span>
					<div class="grid grid-cols-2 gap-2">
						<button
							v-for="opt in menuNavOptions"
							:key="opt"
							type="button"
							:class="[
								'rounded-xl px-3 py-2 text-xs font-bold capitalize transition-all border border-solid',
								current.menu_navigation === opt
									? 'bg-brand text-black border-brand shadow'
									: 'bg-surface-2 text-secondary border-surface-5 hover:border-surface-4 hover:text-contrast',
							]"
							:disabled="saving"
							@click="current.menu_navigation = opt"
						>
							{{ opt }}
						</button>
					</div>
				</div>
			</div>
		</section>

		<!-- Section 3: Horizon Extras -->
		<section
			class="glass-surface glass-surface--strong rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5 flex flex-col gap-5 mt-4"
		>
			<div class="flex items-start gap-3">
				<div
					class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-brand/10 text-brand"
				>
					<CompassIcon class="size-5" />
				</div>
				<div>
					<h2 class="m-0 text-lg font-semibold text-contrast">
						{{ formatMessage(messages.extrasSectionTitle) }}
					</h2>
					<p class="m-0 mt-1 text-sm text-secondary">
						{{ formatMessage(messages.extrasSectionDesc) }}
					</p>
				</div>
			</div>

			<!-- Toggles: Zoom & Minimap -->
			<div
				class="grid grid-cols-1 md:grid-cols-2 gap-3 pt-2 border-t border-solid border-surface-4"
			>
				<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
					<div class="flex flex-col">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.zoomTitle)
						}}</span>
						<span class="text-[11px] text-secondary">{{ formatMessage(messages.zoomDesc) }}</span>
					</div>
					<Toggle
						id="horizon-zoom"
						:model-value="current.zoom_enabled"
						:disabled="saving"
						@update:model-value="current.zoom_enabled = $event ?? true"
					/>
				</div>

				<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
					<div class="flex flex-col">
						<span class="text-xs font-bold text-contrast">{{
							formatMessage(messages.minimapTitle)
						}}</span>
						<span class="text-[11px] text-secondary">{{
							formatMessage(messages.minimapDesc)
						}}</span>
					</div>
					<Toggle
						id="horizon-minimap"
						:model-value="current.minimap_enabled"
						:disabled="saving"
						@update:model-value="current.minimap_enabled = $event ?? false"
					/>
				</div>
			</div>

			<!-- Performance Overlay -->
			<div class="flex flex-col gap-2">
				<span class="text-sm font-semibold text-contrast">{{
					formatMessage(messages.overlayTitle)
				}}</span>
				<div class="grid grid-cols-3 gap-2">
					<button
						v-for="opt in overlayOptions"
						:key="opt"
						type="button"
						:class="[
							'rounded-xl px-3 py-2 text-xs font-bold capitalize transition-all border border-solid',
							current.performance_overlay === opt
								? 'bg-brand text-black border-brand shadow'
								: 'bg-surface-2 text-secondary border-surface-5 hover:border-surface-4 hover:text-contrast',
						]"
						:disabled="saving"
						@click="current.performance_overlay = opt"
					>
						{{ opt.replace(/_/g, ' ') }}
					</button>
				</div>
			</div>

			<!-- Battery Saver Mode -->
			<div class="flex items-center justify-between gap-4 rounded-xl bg-surface-2 p-3">
				<div class="flex flex-col">
					<span class="text-xs font-bold text-contrast">{{
						formatMessage(messages.batteryModeTitle)
					}}</span>
					<span class="text-[11px] text-secondary">{{
						formatMessage(messages.batteryModeDesc)
					}}</span>
				</div>
				<Toggle
					id="horizon-battery-mode"
					:model-value="current.battery_mode"
					:disabled="saving"
					@update:model-value="current.battery_mode = $event ?? false"
				/>
			</div>
		</section>

		<!-- Section 4: AI Agent Control & MCP Server -->
		<section
			class="glass-surface glass-surface--strong rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5 flex flex-col gap-5"
		>
			<div class="flex items-start gap-3">
				<div
					class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-purple-500/10 text-purple-400"
				>
					<BotIcon class="size-5" />
				</div>
				<div>
					<div class="flex flex-wrap items-center gap-2">
						<h2 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.agentSectionTitle) }}
						</h2>
						<span
							class="shrink-0 rounded-full bg-purple-500/10 px-2 py-0.5 text-xs font-bold text-purple-400"
						>
							Model Context Protocol (MCP)
						</span>
					</div>
					<p class="m-0 mt-1 text-sm text-secondary">
						{{ formatMessage(messages.agentSectionDesc) }}
					</p>
				</div>
			</div>

			<!-- Main Toggle: Allow AI Agents -->
			<div
				class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3.5 border border-solid border-surface-5"
			>
				<div class="flex flex-col">
					<span class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.allowAiAgentsTitle) }}
					</span>
					<span class="text-xs text-secondary mt-0.5">
						{{ formatMessage(messages.allowAiAgentsDesc) }}
					</span>
				</div>
				<Toggle
					id="horizon-allow-ai-agents"
					:model-value="current.allow_ai_agents"
					:disabled="saving"
					@update:model-value="toggleAgentControl($event ?? false)"
				/>
			</div>

			<!-- Subpanel when Allow AI Agents is ON -->
			<div v-if="current.allow_ai_agents" class="flex flex-col gap-4 pt-1">
				<!-- Binary Status & Verification Card -->
				<div
					class="flex flex-wrap items-center justify-between gap-3 rounded-xl border border-solid border-surface-5 bg-surface-2 p-4"
				>
					<div class="flex flex-col gap-1">
						<div class="flex items-center gap-2">
							<span class="text-xs font-bold text-contrast">horizon-ctl:</span>
							<span
								v-if="agentStatus?.horizon_ctl_installed"
								class="rounded bg-green-500/20 px-2 py-0.5 text-[11px] font-semibold text-green-400"
							>
								{{ formatMessage(messages.ctlInstalled) }}
							</span>
							<span
								v-else
								class="rounded bg-orange/20 px-2 py-0.5 text-[11px] font-semibold text-orange"
							>
								{{ formatMessage(messages.ctlMissing) }}
							</span>
						</div>
						<span class="font-mono text-[11px] text-secondary break-all">
							{{
								agentStatus?.horizon_ctl_path ||
								'Default: ~/.gemini/antigravity/horizon/bin/horizon-ctl'
							}}
						</span>
					</div>

					<Button size="sm" type="outlined" :disabled="testingMcp" @click="handleTestMcp">
						<SpinnerIcon v-if="testingMcp" class="size-4 animate-spin" />
						<CheckIcon v-else class="size-4 text-green-400" />
						<span>{{ formatMessage(messages.testMcpBtn) }}</span>
					</Button>
				</div>

				<!-- Test Results Banner -->
				<div
					v-if="mcpTestReport"
					class="flex items-center gap-2 rounded-xl bg-green-500/10 p-3 text-xs text-green-400 font-medium"
				>
					<CheckIcon class="size-4 shrink-0" />
					<span>{{ mcpTestReport.message }} (protocol {{ mcpTestReport.protocol_version }})</span>
				</div>
				<div
					v-else-if="mcpTestError"
					class="rounded-xl bg-red-500/10 p-3 text-xs text-red-400 font-medium"
				>
					{{ mcpTestError }}
				</div>

				<!-- Connect an Agent: Client Picker -->
				<div
					class="flex flex-col gap-3 rounded-xl border border-solid border-surface-5 bg-surface-2 p-4"
				>
					<div class="flex flex-col gap-0.5">
						<span class="text-sm font-semibold text-contrast">
							{{ formatMessage(messages.connectAgentTitle) }}
						</span>
						<span class="text-xs text-secondary">
							{{ formatMessage(messages.connectAgentDesc) }}
						</span>
					</div>

					<!-- Client tabs -->
					<div class="flex flex-wrap gap-2">
						<button
							v-for="c in agentConfigs"
							:key="c.id"
							type="button"
							:class="[
								'rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer border border-solid',
								selectedClient?.id === c.id
									? 'border-brand bg-brand/10 text-brand'
									: 'border-surface-5 bg-surface-3 text-secondary hover:text-contrast',
							]"
							@click="selectedClientId = c.id"
						>
							{{ c.name }}
						</button>
					</div>

					<!-- Selected Client Details -->
					<div
						v-if="selectedClient"
						class="flex flex-col gap-3 pt-2 border-t border-solid border-surface-4"
					>
						<div class="flex items-center justify-between text-xs">
							<span class="text-secondary">
								Target configuration file:
								<span class="font-mono text-contrast">{{ selectedClient.target_file }}</span>
							</span>
							<Button
								size="xs"
								type="quiet"
								@click="copyText(selectedClient.config_snippet, 'config')"
							>
								<CheckIcon v-if="copiedField === 'config'" class="size-3 text-green-400" />
								<CopyIcon v-else class="size-3" />
								<span>{{
									copiedField === 'config'
										? formatMessage(messages.copied)
										: formatMessage(messages.copyConfig)
								}}</span>
							</Button>
						</div>

						<pre
							class="m-0 rounded-xl bg-surface-1 p-3 font-mono text-[11px] text-contrast overflow-x-auto border border-solid border-surface-5"
							>{{ selectedClient.config_snippet }}</pre
						>

						<!-- CLI Command (if available) -->
						<div v-if="selectedClient.add_command" class="flex flex-col gap-1.5 mt-1">
							<div class="flex items-center justify-between text-xs">
								<span class="font-semibold text-contrast">{{
									formatMessage(messages.oneClickCli)
								}}</span>
								<Button
									size="xs"
									type="quiet"
									@click="copyText(selectedClient.add_command, 'command')"
								>
									<CheckIcon v-if="copiedField === 'command'" class="size-3 text-green-400" />
									<CopyIcon v-else class="size-3" />
									<span>{{
										copiedField === 'command'
											? formatMessage(messages.copied)
											: formatMessage(messages.copyCommand)
									}}</span>
								</Button>
							</div>
							<div
								class="flex items-center justify-between rounded-xl bg-surface-1 px-3 py-2 font-mono text-[11px] text-brand border border-solid border-surface-5"
							>
								<span class="truncate">{{ selectedClient.add_command }}</span>
							</div>
						</div>
					</div>
				</div>
			</div>
		</section>
	</GradleSettingsPage>
</template>
