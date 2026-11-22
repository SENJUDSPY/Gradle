<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="true"
		max-width="640px"
		width="640px"
	>
		<div class="flex w-full flex-col gap-4">
			<!-- Incompatibility Warning Banner -->
			<div class="flex items-start gap-3 rounded-2xl border border-solid border-amber-500/30 bg-amber-500/10 p-4 text-xs text-contrast">
				<TriangleAlertIcon class="size-5 shrink-0 text-amber-400" />
				<div class="flex flex-col gap-1">
					<span class="font-bold text-sm text-amber-300">
						{{ formatMessage(messages.bannerTitle) }}
					</span>
					<span class="text-secondary leading-relaxed">
						{{ formatMessage(messages.bannerDescription) }}
					</span>
				</div>
			</div>

			<!-- Compatibility Summary Banner -->
			<div class="flex items-center justify-between rounded-xl bg-surface-2 p-3 text-xs border border-solid border-surface-5">
				<div class="flex items-center gap-2">
					<SparklesIcon class="size-4 text-brand" />
					<span class="font-bold text-sm text-contrast">
						{{ compatibilitySummaryText }}
					</span>
				</div>
				<span
					class="rounded px-2.5 py-1 text-xs font-semibold border"
					:class="skippedCount > 0 ? 'bg-amber-500/20 text-amber-300 border-amber-500/30' : 'bg-brand/20 text-brand border-brand/30'"
				>
					{{ skippedCount > 0 ? 'Confirmation Required' : '100% Native Ready' }}
				</span>
			</div>

			<!-- Tab switch: Skipped vs Supported -->
			<div class="flex items-center gap-2 border-b border-solid border-surface-5 pb-2">
				<button
					type="button"
					class="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer"
					:class="activeTab === 'skipped' ? 'bg-surface-3 text-contrast' : 'text-secondary hover:text-contrast'"
					@click="activeTab = 'skipped'"
				>
					<span class="size-2 rounded-full bg-amber-400" />
					Skipped / Needs Java ({{ skippedCount }})
				</button>
				<button
					type="button"
					class="flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-semibold transition-colors cursor-pointer"
					:class="activeTab === 'supported' ? 'bg-surface-3 text-contrast' : 'text-secondary hover:text-contrast'"
					@click="activeTab = 'supported'"
				>
					<span class="size-2 rounded-full bg-brand" />
					Supported Natively ({{ supportedCount }})
				</button>
			</div>

			<!-- Tab 1: Skipped / Incompatible Mods List -->
			<div v-if="activeTab === 'skipped'" class="flex flex-col gap-2">
				<div class="flex items-center justify-between text-xs text-secondary">
					<span>Java bytecode mods requiring a JVM runtime</span>
					<span>Will be skipped</span>
				</div>

				<div class="max-h-60 flex flex-col gap-1.5 overflow-y-auto rounded-xl border border-solid border-surface-5 bg-surface-1 p-2">
					<div
						v-for="mod in skippedMods"
						:key="mod.file_name"
						class="flex flex-col gap-1 rounded-lg bg-surface-2 p-2.5 text-xs"
					>
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-2 truncate max-w-[400px]">
								<span class="font-medium text-contrast truncate">
									{{ mod.name }}
								</span>
								<span class="text-[10px] text-secondary font-mono truncate">
									{{ mod.file_name }}
								</span>
							</div>
							<div class="flex items-center gap-1.5 shrink-0">
								<span
									v-if="mod.loader"
									class="rounded bg-surface-4 px-1.5 py-0.5 text-[10px] font-medium text-secondary"
								>
									{{ mod.loader }}
								</span>
								<span class="rounded bg-amber-500/15 px-2 py-0.5 text-[10px] font-semibold text-amber-300 border border-amber-500/30">
									Needs JVM
								</span>
							</div>
						</div>
						<p class="m-0 text-[11px] text-secondary leading-snug">
							{{ mod.reason }}
						</p>
					</div>
					<div
						v-if="skippedMods.length === 0"
						class="p-4 text-center text-xs text-secondary"
					>
						No skipped mods. All mods run natively on Gradle Rust Edition!
					</div>
				</div>
			</div>

			<!-- Tab 2: Supported Mods List -->
			<div v-else class="flex flex-col gap-2">
				<div class="flex items-center justify-between text-xs text-secondary">
					<span>Mods running natively in Gradle</span>
					<span>Built-in / WASM / Asset</span>
				</div>

				<div class="max-h-60 flex flex-col gap-1.5 overflow-y-auto rounded-xl border border-solid border-surface-5 bg-surface-1 p-2">
					<div
						v-for="mod in supportedModsList"
						:key="mod.file_name"
						class="flex flex-col gap-1 rounded-lg bg-surface-2 p-2.5 text-xs"
					>
						<div class="flex items-center justify-between">
							<div class="flex items-center gap-2 truncate max-w-[400px]">
								<span class="font-medium text-contrast truncate">
									{{ mod.name }}
								</span>
								<span class="text-[10px] text-secondary font-mono truncate">
									{{ mod.file_name }}
								</span>
							</div>
							<span
								class="shrink-0 rounded px-2 py-0.5 text-[10px] font-semibold border"
								:class="getCategoryBadgeClass(mod.category)"
							>
								{{ mod.category }}
							</span>
						</div>
						<p class="m-0 text-[11px] text-brand leading-snug">
							{{ mod.details }}
						</p>
					</div>
					<div
						v-if="supportedModsList.length === 0"
						class="p-4 text-center text-xs text-secondary"
					>
						No native mods found in this instance yet.
					</div>
				</div>
			</div>

			<!-- Pack note -->
			<p class="m-0 text-xs text-secondary leading-relaxed">
				{{ formatMessage(messages.compatiblePacksNote) }}
			</p>

			<!-- Action buttons -->
			<div class="flex items-center justify-between gap-3 pt-2">
				<Button
					type="quiet"
					size="lg"
					@click="handleCancel"
				>
					{{ formatMessage(messages.cancel) }}
				</Button>

				<div class="flex items-center gap-2">
					<Button
						type="colored"
						color="brand"
						size="lg"
						@click="handleContinueHorizon"
					>
						<PlayIcon class="size-4" />
						Confirm & Launch (Skip {{ skippedCount }} JARs)
					</Button>
					<Button
						type="outlined"
						size="lg"
						@click="handlePlayWithJava"
					>
						{{ formatMessage(messages.playWithJavaInstead) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { PlayIcon, SparklesIcon, TriangleAlertIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { InstanceModCompatibilityReport, ModJarAnalysis } from '@/helpers/horizon'
import type { ContentItem } from '@/helpers/types'

const messages = defineMessages({
	title: {
		id: 'app.horizon.compat.title',
		defaultMessage: 'Java Mods Incompatible with Gradle',
	},
	bannerTitle: {
		id: 'app.horizon.compat.banner-title',
		defaultMessage: 'Java mod jars cannot execute on Gradle',
	},
	bannerDescription: {
		id: 'app.horizon.compat.banner-desc',
		defaultMessage:
			'Gradle is a native Rust engine for Minecraft 26.3. Standard Java bytecode mods (Forge, NeoForge, Fabric, Quilt) require the Java Virtual Machine. Gradle runs native WebAssembly mods (.wasm, .hmod), shaders, resource packs, and data packs.',
	},
	compatiblePacksNote: {
		id: 'app.horizon.compat.packs-note',
		defaultMessage:
			'Your resource packs, shaders, and data packs will continue to work normally in Gradle. To play with Java mod jars, choose "Play with Java instead".',
	},
	cancel: {
		id: 'common.cancel',
		defaultMessage: 'Cancel',
	},
	playWithJavaInstead: {
		id: 'app.horizon.compat.play-java',
		defaultMessage: 'Play with Java instead',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal> | null>(null)
const activeTab = ref<'skipped' | 'supported'>('skipped')

interface DisplayModItem {
	name: string
	file_name: string
	category: string
	reason: string
	details: string
	loader?: string
	is_supported: boolean
}

const skippedMods = ref<DisplayModItem[]>([])
const supportedModsList = ref<DisplayModItem[]>([])
const supportedCount = ref(0)
const totalModsCount = ref(0)
const skippedCount = ref(0)

const compatibilitySummaryText = computed(() => {
	return `${supportedCount.value} of ${totalModsCount.value} mods supported, ${skippedCount.value} skipped`
})

let onPlayJava: (() => void) | null = null
let onContinueHorizon: (() => void) | null = null
let onCancelLaunch: (() => void) | null = null

function getCategoryBadgeClass(category: string): string {
	switch (category) {
		case 'Built-in Equivalent':
			return 'bg-emerald-500/15 text-emerald-400 border-emerald-500/30'
		case 'Resource Only':
			return 'bg-sky-500/15 text-sky-400 border-sky-500/30'
		case 'WASM Native':
			return 'bg-purple-500/15 text-purple-400 border-purple-500/30'
		default:
			return 'bg-brand/15 text-brand border-brand/30'
	}
}

/**
 * Show compatibility report generated by the Rust classifier
 */
function showReport(
	report: InstanceModCompatibilityReport,
	handlers: {
		playJava: () => void
		continueHorizon: () => void
		cancel: () => void
	},
) {
	totalModsCount.value = report.total_mods
	supportedCount.value = report.supported_count
	skippedCount.value = report.skipped_count

	const skipped: DisplayModItem[] = []
	const supported: DisplayModItem[] = []

	for (const mod of report.mods) {
		const type = mod.classification.type
		const details = mod.classification.details

		if (type === 'UNSUPPORTED') {
			skipped.push({
				name: mod.name,
				file_name: mod.file_name,
				category: 'Needs Java (JVM)',
				reason: details.reason,
				details: details.reason,
				loader: mod.loader !== 'unknown' ? mod.loader.toUpperCase() : undefined,
				is_supported: false,
			})
		} else {
			let cat = 'Supported'
			let desc = details.reason
			if (type === 'BUILT_IN_EQUIVALENT') {
				cat = 'Built-in Equivalent'
				desc = details.feature ? `${details.feature} · ${details.reason}` : details.reason
			} else if (type === 'RESOURCE_ONLY') {
				cat = 'Resource Only'
			} else if (type === 'WASM_AVAILABLE') {
				cat = 'WASM Native'
			}

			supported.push({
				name: mod.name,
				file_name: mod.file_name,
				category: cat,
				reason: details.reason,
				details: desc,
				loader: mod.loader !== 'unknown' ? mod.loader.toUpperCase() : undefined,
				is_supported: true,
			})
		}
	}

	skippedMods.value = skipped
	supportedModsList.value = supported
	activeTab.value = skipped.length > 0 ? 'skipped' : 'supported'

	onPlayJava = handlers.playJava
	onContinueHorizon = handlers.continueHorizon
	onCancelLaunch = handlers.cancel
	modalRef.value?.show()
}

/**
 * Backward compatibility signature for raw ContentItem[]
 */
function show(
	mods: ContentItem[],
	handlers: {
		playJava: () => void
		continueHorizon: () => void
		cancel: () => void
	},
	counts?: { supported: number; total: number; skipped: number },
) {
	if (counts) {
		supportedCount.value = counts.supported
		totalModsCount.value = counts.total
		skippedCount.value = counts.skipped
	} else {
		skippedCount.value = mods.length
		totalModsCount.value = mods.length
		supportedCount.value = 0
	}

	skippedMods.value = mods.map((m) => ({
		name: m.project?.title || m.file_name,
		file_name: m.file_name,
		category: 'Java JAR',
		reason: 'Requires JVM runtime; will not execute on Gradle native Rust engine.',
		details: 'Java bytecode jar',
		is_supported: false,
	}))
	supportedModsList.value = []
	activeTab.value = 'skipped'

	onPlayJava = handlers.playJava
	onContinueHorizon = handlers.continueHorizon
	onCancelLaunch = handlers.cancel
	modalRef.value?.show()
}

function handlePlayWithJava() {
	modalRef.value?.hide()
	onPlayJava?.()
}

function handleContinueHorizon() {
	modalRef.value?.hide()
	onContinueHorizon?.()
}

function handleCancel() {
	modalRef.value?.hide()
	onCancelLaunch?.()
}

defineExpose({
	show,
	showReport,
	hide: () => modalRef.value?.hide(),
})
</script>
