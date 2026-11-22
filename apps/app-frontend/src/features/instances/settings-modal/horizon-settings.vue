<script setup lang="ts">
import {
	CheckIcon,
	CompassIcon,
	GameIcon,
	LayersIcon,
	SparklesIcon,
	SpinnerIcon,
} from '@modrinth/assets'
import {
	Button,
	defineMessages,
	injectNotificationManager,
	SegmentedControl,
	Slider,
	Toggle,
	useVIntl,
} from '@modrinth/ui'
import { computed, onMounted, ref } from 'vue'

import {
	getHorizonSettings,
	type HorizonSettings,
	saveHorizonSettings,
} from '@/helpers/horizon'

import DefaultSettingsIntro from './default-settings-intro.vue'
import { injectInstanceSettings } from './instance-settings-context'

const { handleError, addNotification } = injectNotificationManager()
const { formatMessage } = useVIntl()
const { instance } = injectInstanceSettings()

const messages = defineMessages({
	introDesc: {
		id: 'instance.settings.horizon.intro.desc',
		defaultMessage:
			'Customize Gradle native Rust runtime settings for this instance. These settings are stored separately in the instance folder and never affect Java runtime settings.',
	},
	savedNotice: {
		id: 'instance.settings.horizon.saved.notice',
		defaultMessage: 'Gradle settings saved',
	},
	graphicsSection: {
		id: 'instance.settings.horizon.graphics.title',
		defaultMessage: 'Graphics & Distant Terrain',
	},
	presetLabel: {
		id: 'instance.settings.horizon.preset.label',
		defaultMessage: 'Graphics Preset',
	},
	renderDistanceLabel: {
		id: 'instance.settings.horizon.render-distance.label',
		defaultMessage: 'Render Distance',
	},
	renderDistanceDesc: {
		id: 'instance.settings.horizon.render-distance.desc',
		defaultMessage: '{chunks} chunks ({distance} meters)',
	},
	lodDistanceLabel: {
		id: 'instance.settings.horizon.lod-distance.label',
		defaultMessage: 'Distant Terrain LOD',
	},
	lodDistanceDesc: {
		id: 'instance.settings.horizon.lod-distance.desc',
		defaultMessage: '{chunks} chunks (~{km} km)',
	},
	maxFpsLabel: {
		id: 'instance.settings.horizon.max-fps.label',
		defaultMessage: 'Max Frame Rate',
	},
	unlimitedFps: {
		id: 'instance.settings.horizon.max-fps.unlimited',
		defaultMessage: 'Unlimited',
	},
	caveCullingTitle: {
		id: 'instance.settings.horizon.cave-culling.title',
		defaultMessage: 'Cave Culling',
	},
	caveCullingDesc: {
		id: 'instance.settings.horizon.cave-culling.desc',
		defaultMessage: 'Skip occluded underground chunks for higher FPS',
	},
	entityCullingTitle: {
		id: 'instance.settings.horizon.entity-culling.title',
		defaultMessage: 'Entity Culling',
	},
	entityCullingDesc: {
		id: 'instance.settings.horizon.entity-culling.desc',
		defaultMessage: 'Do not render entities hidden behind solid blocks',
	},
	vsyncTitle: {
		id: 'instance.settings.horizon.vsync.title',
		defaultMessage: 'VSync',
	},
	vsyncDesc: {
		id: 'instance.settings.horizon.vsync.desc',
		defaultMessage: 'Eliminate screen tearing with display refresh rate pacing',
	},
	controllerSection: {
		id: 'instance.settings.horizon.controller.title',
		defaultMessage: 'Controller & Haptics',
	},
	controllerEnabledTitle: {
		id: 'instance.settings.horizon.controller-enabled.title',
		defaultMessage: 'Gamepad Support',
	},
	controllerEnabledDesc: {
		id: 'instance.settings.horizon.controller-enabled.desc',
		defaultMessage: 'Native DualSense, Xbox, and Switch controller input',
	},
	vibrationTitle: {
		id: 'instance.settings.horizon.vibration.title',
		defaultMessage: 'Haptic Feedback',
	},
	vibrationDesc: {
		id: 'instance.settings.horizon.vibration.desc',
		defaultMessage: 'Vibration and haptic effects on damage, mining, and explosions',
	},
	gameplaySection: {
		id: 'instance.settings.horizon.gameplay.title',
		defaultMessage: 'Gameplay & Utilities',
	},
	zoomTitle: {
		id: 'instance.settings.horizon.zoom.title',
		defaultMessage: 'Cinematic Zoom (C key)',
	},
	zoomDesc: {
		id: 'instance.settings.horizon.zoom.desc',
		defaultMessage: 'Smooth camera zoom without requiring OptiFine or Zoomify mods',
	},
	minimapTitle: {
		id: 'instance.settings.horizon.minimap.title',
		defaultMessage: 'Built-in Minimap',
	},
	minimapDesc: {
		id: 'instance.settings.horizon.minimap.desc',
		defaultMessage: 'Lightweight native HUD minimap with player coordinates and biome',
	},
	performanceOverlayTitle: {
		id: 'instance.settings.horizon.overlay.title',
		defaultMessage: 'Performance Overlay',
	},
	overlayNone: {
		id: 'instance.settings.horizon.overlay.none',
		defaultMessage: 'None',
	},
	overlayFps: {
		id: 'instance.settings.horizon.overlay.fps',
		defaultMessage: 'FPS Only',
	},
	overlayDetailed: {
		id: 'instance.settings.horizon.overlay.detailed',
		defaultMessage: 'Detailed',
	},
	saveButton: {
		id: 'instance.settings.horizon.save.button',
		defaultMessage: 'Save Gradle Settings',
	},
})

const loading = ref(true)
const saving = ref(false)
const savedJustNow = ref(false)

const settings = ref<HorizonSettings>({
	graphics_preset: 'balanced',
	render_distance: 16,
	lod_distance: 64,
	cave_culling: true,
	entity_culling: true,
	vsync: true,
	max_fps: 140,
	fov: 70,
	graphics_api: 'vulkan',
	controller_enabled: true,
	vibration: true,
	adaptive_triggers: true,
	lightbar: 'health',
	look_sensitivity: 100,
	invert_look_y: false,
	move_deadzone: 10,
	look_deadzone: 10,
	button_glyphs: 'auto',
	menu_navigation: 'focus',
	zoom_enabled: true,
	minimap_enabled: false,
	performance_overlay: 'none',
	battery_mode: false,
	allow_ai_agents: false,
})

const presets = ['performance', 'balanced', 'quality', 'ultra'] as const
const overlayOptions = ['none', 'fps', 'fps_and_graph'] as const
const graphicsApis = ['vulkan', 'opengl', 'dx12'] as const
const shadowOptions = ['off', 'low', 'medium', 'high'] as const
const msaaOptions = [1, 2, 4, 8] as const
const filteringOptions = ['nearest', 'bilinear', 'trilinear', 'anisotropic'] as const

function resetKeybinds() {
	settings.value.keybinds = {
		'key.forward': 'key.keyboard.w',
		'key.left': 'key.keyboard.a',
		'key.back': 'key.keyboard.s',
		'key.right': 'key.keyboard.d',
		'key.jump': 'key.keyboard.space',
		'key.sneak': 'key.keyboard.left.shift',
		'key.sprint': 'key.keyboard.left.control',
		'key.attack': 'key.mouse.left',
		'key.use': 'key.mouse.right',
		'key.zoom': 'key.keyboard.c',
	}
}

const maxFpsDisplay = computed(() => {
	return settings.value.max_fps >= 260
		? formatMessage(messages.unlimitedFps)
		: `${settings.value.max_fps} FPS`
})

const lodDistanceDisplay = computed(() => {
	const km = Math.round((settings.value.lod_distance * 16) / 1000)
	return formatMessage(messages.lodDistanceDesc, {
		chunks: settings.value.lod_distance,
		km,
	})
})

const renderDistanceDisplay = computed(() => {
	return formatMessage(messages.renderDistanceDesc, {
		chunks: settings.value.render_distance,
		distance: settings.value.render_distance * 16,
	})
})

async function loadSettings() {
	loading.value = true
	try {
		const s = await getHorizonSettings(instance.value.path)
		settings.value = s
	} catch (error) {
		handleError(error)
	} finally {
		loading.value = false
	}
}

async function saveSettings() {
	if (saving.value) return
	saving.value = true
	try {
		await saveHorizonSettings(settings.value, instance.value.path)
		savedJustNow.value = true
		setTimeout(() => {
			savedJustNow.value = false
		}, 2000)
		addNotification({
			type: 'success',
			title: formatMessage(messages.savedNotice),
		})
	} catch (error) {
		handleError(error)
	} finally {
		saving.value = false
	}
}

onMounted(() => {
	void loadSettings()
})
</script>

<template>
	<div class="flex flex-col gap-6">
		<DefaultSettingsIntro
			:description="formatMessage(messages.introDesc)"
			tab="horizon"
		/>

		<div v-if="loading" class="flex items-center justify-center py-12 text-secondary">
			<SpinnerIcon class="size-6 animate-spin text-brand" />
		</div>

		<template v-else>
			<!-- Section 1: Graphics & LOD -->
			<section class="glass-surface glass-surface--strong flex flex-col gap-5 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5">
				<div class="flex items-start gap-3">
					<div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-purple-500/10 text-purple-400">
						<SparklesIcon class="size-5" />
					</div>
					<div>
						<h2 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.graphicsSection) }}
						</h2>
						<p class="m-0 mt-1 text-sm text-secondary">
							{{ formatMessage(messages.introDesc) }}
						</p>
					</div>
				</div>

				<!-- Graphics Preset -->
				<div class="flex flex-col gap-2">
					<label class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.presetLabel) }}
					</label>
					<SegmentedControl
						:model-value="settings.graphics_preset"
						:items="[...presets]"
						:format-label="(p) => p.charAt(0).toUpperCase() + p.slice(1)"
						@update:model-value="(val) => val && (settings.graphics_preset = val)"
					/>
				</div>

				<!-- Graphics Backend API -->
				<div class="flex flex-col gap-2">
					<label class="text-sm font-semibold text-contrast">
						Graphics Backend
					</label>
					<SegmentedControl
						:model-value="settings.graphics_api"
						:items="[...graphicsApis]"
						:format-label="(api) => api.toUpperCase()"
						@update:model-value="(val) => val && (settings.graphics_api = val)"
					/>
				</div>

				<!-- Shadow Quality & Anti-Aliasing (MSAA) -->
				<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
					<div class="flex flex-col gap-2">
						<label class="text-xs font-semibold text-contrast">
							Shadows
						</label>
						<SegmentedControl
							:model-value="settings.shadow_quality ?? 'medium'"
							:items="[...shadowOptions]"
							:format-label="(s) => s.charAt(0).toUpperCase() + s.slice(1)"
							@update:model-value="(val) => val && (settings.shadow_quality = val)"
						/>
					</div>

					<div class="flex flex-col gap-2">
						<label class="text-xs font-semibold text-contrast">
							Anti-Aliasing (MSAA)
						</label>
						<SegmentedControl
							:model-value="settings.msaa_samples ?? 1"
							:items="[...msaaOptions]"
							:format-label="(m) => m === 1 ? 'Off' : `${m}x`"
							@update:model-value="(val) => val && (settings.msaa_samples = val)"
						/>
					</div>
				</div>

				<!-- Texture Filtering -->
				<div class="flex flex-col gap-2">
					<label class="text-xs font-semibold text-contrast">
						Texture Filtering
					</label>
					<SegmentedControl
						:model-value="settings.texture_filtering ?? 'trilinear'"
						:items="[...filteringOptions]"
						:format-label="(f) => f.charAt(0).toUpperCase() + f.slice(1)"
						@update:model-value="(val) => val && (settings.texture_filtering = val)"
					/>
				</div>

				<!-- Render Distance Slider (2 - 64) -->
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-sm">
						<span class="font-semibold text-contrast">
							{{ formatMessage(messages.renderDistanceLabel) }}
						</span>
						<span class="font-mono text-xs text-secondary">
							{{ renderDistanceDisplay }}
						</span>
					</div>
					<Slider
						v-model="settings.render_distance"
						:min="2"
						:max="64"
						:step="1"
					/>
				</div>

				<!-- Distant Terrain LOD Slider (16 - 1024) -->
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-sm">
						<span class="font-semibold text-contrast">
							{{ formatMessage(messages.lodDistanceLabel) }}
						</span>
						<span class="font-mono text-xs text-secondary">
							{{ lodDistanceDisplay }}
						</span>
					</div>
					<Slider
						v-model="settings.lod_distance"
						:min="16"
						:max="1024"
						:step="16"
					/>
				</div>

				<!-- Max FPS Slider -->
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between text-sm">
						<span class="font-semibold text-contrast">
							{{ formatMessage(messages.maxFpsLabel) }}
						</span>
						<span class="font-mono text-xs text-secondary">
							{{ maxFpsDisplay }}
						</span>
					</div>
					<Slider
						v-model="settings.max_fps"
						:min="30"
						:max="260"
						:step="10"
					/>
				</div>

				<!-- Toggles: Cave Culling, Entity Culling, VSync -->
				<div class="grid grid-cols-1 md:grid-cols-3 gap-3 border-t border-solid border-surface-4 pt-3">
					<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.caveCullingTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.caveCullingDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-cave-culling"
							:model-value="settings.cave_culling"
							@update:model-value="(v) => (settings.cave_culling = v ?? true)"
						/>
					</div>

					<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.entityCullingTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.entityCullingDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-entity-culling"
							:model-value="settings.entity_culling"
							@update:model-value="(v) => (settings.entity_culling = v ?? true)"
						/>
					</div>

					<div class="flex items-center justify-between gap-2 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.vsyncTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.vsyncDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-vsync"
							:model-value="settings.vsync"
							@update:model-value="(v) => (settings.vsync = v ?? true)"
						/>
					</div>
				</div>
			</section>

			<!-- Section 2: Controller & Haptics -->
			<section class="glass-surface glass-surface--strong flex flex-col gap-5 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5">
				<div class="flex items-start gap-3">
					<div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-blue-500/10 text-blue-400">
						<GameIcon class="size-5" />
					</div>
					<div>
						<h2 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.controllerSection) }}
						</h2>
						<p class="m-0 mt-1 text-sm text-secondary">
							{{ formatMessage(messages.controllerEnabledDesc) }}
						</p>
					</div>
				</div>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
					<div class="flex items-center justify-between gap-3 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.controllerEnabledTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.controllerEnabledDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-controller"
							:model-value="settings.controller_enabled"
							@update:model-value="(v) => (settings.controller_enabled = v ?? true)"
						/>
					</div>

					<div class="flex items-center justify-between gap-3 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.vibrationTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.vibrationDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-vibration"
							:model-value="settings.vibration"
							:disabled="!settings.controller_enabled"
							@update:model-value="(v) => (settings.vibration = v ?? true)"
						/>
					</div>
				</div>
			</section>

			<!-- Section 3: Gameplay & Utilities -->
			<section class="glass-surface glass-surface--strong flex flex-col gap-5 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5">
				<div class="flex items-start gap-3">
					<div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-emerald-500/10 text-emerald-400">
						<CompassIcon class="size-5" />
					</div>
					<div>
						<h2 class="m-0 text-lg font-semibold text-contrast">
							{{ formatMessage(messages.gameplaySection) }}
						</h2>
						<p class="m-0 mt-1 text-sm text-secondary">
							{{ formatMessage(messages.zoomDesc) }}
						</p>
					</div>
				</div>

				<div class="grid grid-cols-1 md:grid-cols-2 gap-3">
					<div class="flex items-center justify-between gap-3 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.zoomTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.zoomDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-zoom"
							:model-value="settings.zoom_enabled"
							@update:model-value="(v) => (settings.zoom_enabled = v ?? true)"
						/>
					</div>

					<div class="flex items-center justify-between gap-3 rounded-xl bg-surface-2 p-3">
						<div class="flex flex-col">
							<span class="text-xs font-bold text-contrast">
								{{ formatMessage(messages.minimapTitle) }}
							</span>
							<span class="text-[11px] text-secondary">
								{{ formatMessage(messages.minimapDesc) }}
							</span>
						</div>
						<Toggle
							id="instance-horizon-minimap"
							:model-value="settings.minimap_enabled"
							@update:model-value="(v) => (settings.minimap_enabled = v ?? false)"
						/>
					</div>
				</div>

				<!-- Performance Overlay -->
				<div class="flex flex-col gap-2">
					<label class="text-sm font-semibold text-contrast">
						{{ formatMessage(messages.performanceOverlayTitle) }}
					</label>
					<SegmentedControl
						:model-value="settings.performance_overlay"
						:items="[...overlayOptions]"
						:format-label="(o) => (o === 'none' ? formatMessage(messages.overlayNone) : o === 'fps' ? formatMessage(messages.overlayFps) : formatMessage(messages.overlayDetailed))"
						@update:model-value="(val) => val && (settings.performance_overlay = val)"
					/>
				</div>
			</section>

			<!-- Section 4: Resource Packs & Keybinds -->
			<section class="glass-surface glass-surface--strong flex flex-col gap-5 rounded-2xl border border-solid border-surface-5 bg-surface-3 p-5">
				<div class="flex items-start gap-3">
					<div class="flex size-10 shrink-0 items-center justify-center rounded-xl bg-purple-500/10 text-purple-400">
						<LayersIcon class="size-5" />
					</div>
					<div>
						<h2 class="m-0 text-lg font-semibold text-contrast">
							Resource Packs & Keybinds
						</h2>
						<p class="m-0 mt-1 text-sm text-secondary">
							Configured directly into the Horizon options.txt manifest for native loading.
						</p>
					</div>
				</div>

				<!-- Active Resource Packs -->
				<div class="flex flex-col gap-2">
					<label class="text-xs font-semibold text-contrast">
						Active Resource Packs
					</label>
					<div class="flex flex-wrap gap-2 rounded-xl bg-surface-2 p-3">
						<span
							v-for="pack in (settings.resource_packs || ['vanilla'])"
							:key="pack"
							class="rounded-lg bg-surface-3 px-3 py-1.5 text-xs font-mono font-medium text-contrast border border-surface-4"
						>
							{{ pack }}
						</span>
					</div>
				</div>

				<!-- Keybinds mapping grid -->
				<div class="flex flex-col gap-2">
					<div class="flex items-center justify-between">
						<label class="text-xs font-semibold text-contrast">
							Keybinds
						</label>
						<button
							type="button"
							class="text-xs text-purple-400 hover:text-purple-300 transition-colors"
							@click="resetKeybinds"
						>
							Reset Defaults
						</button>
					</div>

					<div class="grid grid-cols-2 sm:grid-cols-3 gap-2">
						<div
							v-for="(keyVal, action) in (settings.keybinds || {})"
							:key="action"
							class="flex items-center justify-between rounded-lg bg-surface-2 px-3 py-2 text-xs"
						>
							<span class="text-secondary truncate">{{ String(action).replace('key.', '') }}</span>
							<kbd class="rounded bg-surface-4 px-2 py-0.5 font-mono text-[10px] text-contrast font-bold border border-surface-5">
								{{ String(keyVal).replace('key.keyboard.', '').replace('key.mouse.', 'Mouse ').toUpperCase() }}
							</kbd>
						</div>
					</div>
				</div>
			</section>

			<!-- Save Action Bar -->
			<div class="flex items-center justify-end gap-3 pt-2">
				<Button
					color="brand"
					:disabled="saving"
					class="flex items-center gap-2"
					@click="saveSettings"
				>
					<SpinnerIcon v-if="saving" class="size-4 animate-spin" />
					<CheckIcon v-else-if="savedJustNow" class="size-4 text-white" />
					<span>{{ formatMessage(messages.saveButton) }}</span>
				</Button>
			</div>
		</template>
	</div>
</template>
