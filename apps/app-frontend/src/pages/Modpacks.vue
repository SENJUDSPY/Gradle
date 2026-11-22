<template>
	<div class="flex flex-col w-full h-full p-6 max-w-7xl mx-auto gap-6 overflow-y-auto">
		<!-- Page Header -->
		<div class="flex flex-col gap-1 border-b border-solid border-surface-5 pb-4">
			<div class="flex items-center gap-2">
				<PackageIcon class="size-6 text-brand" />
				<h1 class="m-0 text-2xl font-bold text-contrast">
					{{ formatMessage(messages.title) }}
				</h1>
			</div>
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>
		</div>

		<!-- Error or Success Alert -->
		<div v-if="creationError" class="rounded-xl bg-red-500/10 p-3 text-xs text-red-400">
			{{ creationError }}
		</div>

		<div
			v-if="creationSuccess"
			class="rounded-xl bg-green-500/10 p-3 text-xs font-semibold text-green-400"
		>
			{{ creationSuccess }}
		</div>

		<!-- View 1: Dropzone / Select Modpack File -->
		<div
			v-if="!analysis && !analyzing"
			class="flex flex-col items-center justify-center rounded-2xl border-2 border-dashed border-surface-5 bg-surface-2 p-12 text-center transition-colors hover:border-brand cursor-pointer"
			@click="handleSelectFile"
		>
			<div class="flex size-16 items-center justify-center rounded-2xl bg-brand/10 text-brand mb-4">
				<UploadIcon class="size-8" />
			</div>
			<h2 class="m-0 text-lg font-bold text-contrast">
				{{ formatMessage(messages.dropzoneTitle) }}
			</h2>
			<p class="m-0 mt-2 max-w-lg text-sm text-secondary">
				{{ formatMessage(messages.dropzoneDesc) }}
			</p>
			<Button type="colored" color="brand" size="lg" class="mt-6 pointer-events-none">
				<FolderOpenIcon class="size-4" />
				{{ formatMessage(messages.selectButton) }}
			</Button>
		</div>

		<!-- View 2: Analyzing Loading -->
		<div
			v-else-if="analyzing"
			class="flex flex-col items-center justify-center rounded-2xl border border-solid border-surface-5 bg-surface-2 p-12 text-center"
		>
			<SpinnerIcon class="size-8 text-brand animate-spin mb-3" />
			<h3 class="m-0 text-base font-bold text-contrast">
				{{ formatMessage(messages.analyzingTitle) }}
			</h3>
			<p class="m-0 mt-1 text-sm text-secondary">
				{{ formatMessage(messages.analyzingDesc) }}
			</p>
		</div>

		<!-- View 3: Analysis Results & Profile Creation -->
		<div v-else-if="analysis" class="flex flex-col gap-6">
			<!-- Pack Overview Banner -->
			<div
				class="flex flex-col gap-4 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-5"
			>
				<div class="flex items-start justify-between">
					<div class="flex flex-col gap-1">
						<div class="flex items-center gap-2">
							<span class="text-xl font-bold text-contrast">{{ analysis.pack_name }}</span>
							<span class="rounded bg-brand/10 px-2.5 py-0.5 text-xs font-semibold text-brand">
								{{ analysis.pack_format }}
							</span>
							<span
								:class="[
									'rounded-full px-2.5 py-0.5 text-xs font-semibold border',
									analysis.compatibility_level === 'Supported'
										? 'bg-green-500/20 text-green-300 border-green-500/40'
										: analysis.compatibility_level === 'PartiallySupported'
											? 'bg-amber-500/20 text-amber-300 border-amber-500/40'
											: 'bg-red-500/20 text-red-300 border-red-500/40',
								]"
							>
								{{
									analysis.compatibility_level === 'Supported'
										? 'Fully Supported'
										: analysis.compatibility_level === 'PartiallySupported'
											? 'Partially Supported'
											: 'Unsupported'
								}}
							</span>
						</div>
						<p v-if="analysis.summary" class="m-0 text-xs text-secondary max-w-2xl">
							{{ analysis.summary }}
						</p>
					</div>

					<div class="flex items-center gap-2">
						<Button size="sm" type="outlined" @click="compatModalRef?.show(analysis)">
							Compatibility Report
						</Button>
						<Button size="sm" type="outlined" @click="handleClear">
							{{ formatMessage(messages.chooseAnother) }}
						</Button>
					</div>
				</div>

				<!-- Stats bar -->
				<div
					class="grid grid-cols-1 md:grid-cols-3 gap-3 pt-2 border-t border-solid border-surface-4"
				>
					<div class="flex flex-col rounded-xl bg-surface-1 p-3">
						<span class="text-xs text-secondary">{{ formatMessage(messages.totalItems) }}</span>
						<span class="text-lg font-bold text-contrast">{{ analysis.total_items }}</span>
					</div>

					<div class="flex flex-col rounded-xl bg-green-500/10 p-3">
						<span class="text-xs text-green-400 font-medium">
							{{ formatMessage(messages.compatibleItems) }}
						</span>
						<div class="flex items-center gap-1.5 text-lg font-bold text-green-400">
							<CheckIcon class="size-5" />
							<span>{{ analysis.compatible_count }}</span>
						</div>
					</div>

					<div class="flex flex-col rounded-xl bg-surface-1 p-3">
						<span class="text-xs text-secondary">{{
							formatMessage(messages.incompatibleItems)
						}}</span>
						<span class="text-lg font-bold text-secondary font-mono">
							{{ analysis.incompatible_count }}
						</span>
					</div>
				</div>

				<!-- Horizon Compatibility Notice -->
				<div
					:class="[
						'flex items-start gap-2.5 rounded-xl p-3 text-xs font-medium border',
						analysis.compatibility_level === 'Supported'
							? 'bg-green-500/10 border-green-500/30 text-green-400'
							: 'bg-amber-500/10 border-amber-500/30 text-amber-300',
					]"
				>
					<SparklesIcon class="size-4 shrink-0 mt-0.5" />
					<div class="flex flex-col gap-0.5">
						<span class="font-bold">
							Compatibility: {{ analysis.compatibility_level === 'Supported' ? 'Fully Supported' : analysis.compatibility_level === 'PartiallySupported' ? 'Partially Supported' : 'Unsupported' }}
						</span>
						<span class="leading-relaxed">
							{{ analysis.compatibility_summary || formatMessage(messages.compatibilityExplainer) }}
						</span>
					</div>
				</div>
			</div>

			<!-- Items Inspection Section -->
			<div
				class="flex flex-col gap-3 rounded-2xl border border-solid border-surface-5 bg-surface-2 p-5"
			>
				<div class="flex flex-col md:flex-row md:items-center justify-between gap-3">
					<!-- Filter tabs -->
					<div class="flex items-center rounded-xl bg-surface-3 p-1 text-xs font-semibold">
						<button
							type="button"
							:class="[
								'flex items-center gap-1 rounded-lg px-2.5 py-1 transition-all',
								filter === 'all'
									? 'bg-surface-5 text-contrast shadow-sm'
									: 'text-secondary hover:text-contrast',
							]"
							@click="filter = 'all'"
						>
							{{ formatMessage(messages.filterAll) }} ({{ analysis.total_items }})
						</button>
						<button
							type="button"
							:class="[
								'flex items-center gap-1 rounded-lg px-2.5 py-1 transition-all',
								filter === 'compatible'
									? 'bg-green-500/20 text-green-400 shadow-sm'
									: 'text-secondary hover:text-contrast',
							]"
							@click="filter = 'compatible'"
						>
							{{ formatMessage(messages.filterCompatible) }} ({{ analysis.compatible_count }})
						</button>
						<button
							type="button"
							:class="[
								'flex items-center gap-1 rounded-lg px-2.5 py-1 transition-all',
								filter === 'incompatible'
									? 'bg-surface-5 text-secondary shadow-sm'
									: 'text-secondary hover:text-contrast',
							]"
							@click="filter = 'incompatible'"
						>
							{{ formatMessage(messages.filterJavaMods) }} ({{ analysis.incompatible_count }})
						</button>
					</div>

					<!-- Search input -->
					<div class="w-full md:w-64">
						<Input
							v-model="searchQuery"
							:placeholder="formatMessage(messages.searchPlaceholder)"
							size="sm"
						/>
					</div>
				</div>

				<!-- Items Table -->
				<div class="flex flex-col gap-1.5 max-h-80 overflow-y-auto pr-1">
					<div
						v-for="item in filteredItems"
						:key="item.path"
						class="flex items-center justify-between rounded-xl bg-surface-1 px-3.5 py-2.5 text-xs transition-colors hover:bg-surface-3"
					>
						<div class="flex items-center gap-2.5 min-w-0 pr-3">
							<LayersIcon class="size-4 shrink-0 text-secondary" />
							<div class="flex flex-col min-w-0">
								<span class="font-semibold text-contrast truncate">{{ item.name }}</span>
								<span class="text-[11px] text-secondary truncate font-mono">{{ item.path }}</span>
							</div>
						</div>

						<div class="flex items-center gap-2 shrink-0">
							<span
								:class="[
									'rounded px-2 py-0.5 text-[11px] font-semibold',
									item.is_compatible
										? 'bg-green-500/10 text-green-400'
										: 'bg-surface-4 text-secondary',
								]"
							>
								{{
									item.is_compatible
										? formatMessage(messages.statusCarriedOver)
										: formatMessage(messages.statusSkipped)
								}}
							</span>
						</div>
					</div>
				</div>

				<!-- Profile Creation CTA -->
				<div
					class="mt-4 flex flex-col md:flex-row md:items-center justify-between gap-3 border-t border-solid border-surface-4 pt-4"
				>
					<div class="flex flex-col gap-1 w-full md:w-80">
						<span class="text-xs font-semibold text-contrast">
							{{ formatMessage(messages.profileNameLabel) }}
						</span>
						<Input v-model="profileName" placeholder="Gradle Profile Name" />
					</div>

					<Button
						size="lg"
						type="colored"
						color="brand"
						:disabled="!profileName.trim() || creatingProfile"
						@click="handleCreateProfile"
					>
						<SpinnerIcon v-if="creatingProfile" class="size-4 animate-spin" />
						<PlayIcon v-else class="size-4" />
						{{
							creatingProfile
								? formatMessage(messages.creatingProfileButton)
								: formatMessage(messages.createProfileButton)
						}}
					</Button>
				</div>
			</div>
		</div>

		<!-- Horizon Profiles section -->
		<div v-if="horizonProfiles.length > 0" class="flex flex-col gap-3 mt-4">
			<h2 class="m-0 text-lg font-bold text-contrast">
				{{ formatMessage(messages.yourProfiles) }}
			</h2>
			<div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3">
				<router-link
					v-for="prof in horizonProfiles"
					:key="prof.id"
					:to="`/instance/${encodeURIComponent(prof.id)}`"
					class="flex items-center justify-between rounded-2xl border border-solid border-surface-5 bg-surface-2 p-4 transition-all hover:border-brand no-underline"
				>
					<div class="flex items-center gap-3">
						<div class="flex size-10 items-center justify-center rounded-xl bg-brand/10 text-brand">
							<PackageIcon class="size-5" />
						</div>
						<div class="flex flex-col">
							<span class="text-sm font-bold text-contrast">{{ prof.name }}</span>
							<span class="text-xs text-secondary">Gradle · 26.3</span>
						</div>
					</div>
					<PlayIcon class="size-4 text-brand" />
				</router-link>
			</div>
		</div>

		<!-- Modpack Compatibility Modal -->
		<HorizonModpackCompatibilityModal
			ref="compatModalRef"
			@import-rust="onModalImportRust"
			@import-java="onModalImportJava"
		/>
	</div>
</template>

<script setup lang="ts">
import {
	CheckIcon,
	FolderOpenIcon,
	LayersIcon,
	PackageIcon,
	PlayIcon,
	SparklesIcon,
	SpinnerIcon,
	UploadIcon,
} from '@modrinth/assets'
import { Button, defineMessages, Input, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { computed, onMounted, ref } from 'vue'
import { useRouter } from 'vue-router'

import { analyzeModpackFile, extractModpackAssets, type ModpackAnalysis } from '@/helpers/horizon'
import { install_create_instance, install_create_modpack_instance } from '@/helpers/install'
import { get, list } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'
import { useRootBreadcrumb } from '@/shared/breadcrumbs'
import HorizonModpackCompatibilityModal from '@/components/ui/modal/HorizonModpackCompatibilityModal.vue'

const { formatMessage } = useVIntl()
const router = useRouter()

useRootBreadcrumb({
	slot: 'root',
	id: 'modpacks',
	label: 'Modpacks',
	to: '/modpacks',
	visual: { type: 'icon', component: PackageIcon },
})

const selectedPackPath = ref<string | null>(null)
const compatModalRef = ref<InstanceType<typeof HorizonModpackCompatibilityModal> | null>(null)
const analyzing = ref(false)
const analysis = ref<ModpackAnalysis | null>(null)
const filter = ref<'all' | 'compatible' | 'incompatible'>('all')
const searchQuery = ref('')
const profileName = ref('')
const creatingProfile = ref(false)
const creationError = ref<string | null>(null)
const creationSuccess = ref<string | null>(null)
const instances = ref<GameInstance[]>([])

const messages = defineMessages({
	title: {
		id: 'modpacks.page.title',
		defaultMessage: 'Modpack Porter for Gradle',
	},
	subtitle: {
		id: 'modpacks.page.subtitle',
		defaultMessage:
			'Import Modrinth (.mrpack) and CurseForge (.zip) modpacks. Gradle carries over compatible data packs, resource packs, shaders, configs, and assets into an instant native profile.',
	},
	dropzoneTitle: {
		id: 'modpacks.page.dropzone-title',
		defaultMessage: 'Select or Drop a Modpack File',
	},
	dropzoneDesc: {
		id: 'modpacks.page.dropzone-desc',
		defaultMessage:
			'Supports Modrinth (.mrpack) and CurseForge (.zip) archives. Analysis runs 100% locally on your machine.',
	},
	selectButton: {
		id: 'modpacks.page.select-button',
		defaultMessage: 'Choose Modpack File',
	},
	analyzingTitle: {
		id: 'modpacks.page.analyzing-title',
		defaultMessage: 'Analyzing Modpack Components...',
	},
	analyzingDesc: {
		id: 'modpacks.page.analyzing-desc',
		defaultMessage:
			'Classifying resource packs, data packs, shaders, configs, and Java bytecode mods.',
	},
	chooseAnother: {
		id: 'modpacks.page.choose-another',
		defaultMessage: 'Choose Another File',
	},
	totalItems: {
		id: 'modpacks.page.total-items',
		defaultMessage: 'Total Pack Items',
	},
	compatibleItems: {
		id: 'modpacks.page.compatible-items',
		defaultMessage: 'Compatible with Gradle',
	},
	incompatibleItems: {
		id: 'modpacks.page.incompatible-items',
		defaultMessage: 'Java Bytecode (Skipped)',
	},
	compatibilityExplainer: {
		id: 'modpacks.page.compatibility-explainer',
		defaultMessage:
			'Gradle runs Minecraft 26.3 natively in Rust with wgpu/Vulkan shaders. Data packs, resource packs, shaders, and configs run natively. Java code mods (.jar) require JVM and are excluded.',
	},
	filterAll: {
		id: 'modpacks.page.filter-all',
		defaultMessage: 'All Components',
	},
	filterCompatible: {
		id: 'modpacks.page.filter-compatible',
		defaultMessage: 'Data & Assets',
	},
	filterJavaMods: {
		id: 'modpacks.page.filter-java-mods',
		defaultMessage: 'Java Mods',
	},
	searchPlaceholder: {
		id: 'modpacks.page.search-placeholder',
		defaultMessage: 'Filter mods by name...',
	},
	statusCarriedOver: {
		id: 'modpacks.page.status-carried-over',
		defaultMessage: 'Carried Over',
	},
	statusSkipped: {
		id: 'modpacks.page.status-skipped',
		defaultMessage: 'Skipped (Java)',
	},
	profileNameLabel: {
		id: 'modpacks.page.profile-name-label',
		defaultMessage: 'Gradle Profile Name',
	},
	createProfileButton: {
		id: 'modpacks.page.create-profile-button',
		defaultMessage: 'Create & Play Gradle Profile',
	},
	creatingProfileButton: {
		id: 'modpacks.page.creating-profile-button',
		defaultMessage: 'Creating Profile...',
	},
	yourProfiles: {
		id: 'modpacks.page.your-profiles',
		defaultMessage: 'Your Gradle Profiles',
	},
})

const horizonProfiles = computed(() => {
	return instances.value.filter((i) => i.loader === 'horizon')
})

const filteredItems = computed(() => {
	if (!analysis.value) return []
	let list = analysis.value.items

	if (filter.value === 'compatible') {
		list = list.filter((i) => i.is_compatible)
	} else if (filter.value === 'incompatible') {
		list = list.filter((i) => !i.is_compatible)
	}

	if (searchQuery.value.trim()) {
		const q = searchQuery.value.trim().toLowerCase()
		list = list.filter((i) => i.name.toLowerCase().includes(q) || i.path.toLowerCase().includes(q))
	}

	return list
})

async function fetchInstances() {
	try {
		instances.value = await list()
	} catch {
		instances.value = []
	}
}

async function handleSelectFile() {
	creationError.value = null
	creationSuccess.value = null

	try {
		const file = await open({
			multiple: false,
			filters: [
				{
					name: 'Modpacks (*.mrpack, *.zip)',
					extensions: ['mrpack', 'zip'],
				},
			],
		})

		if (!file || typeof file !== 'string') return

		selectedPackPath.value = file
		analyzing.value = true

		const report = await analyzeModpackFile(file)
		analysis.value = report
		profileName.value = `${report.pack_name} (Gradle)`
		if (report.compatibility_level !== 'Supported') {
			compatModalRef.value?.show(report)
		}
	} catch (e: unknown) {
		creationError.value = e instanceof Error ? e.message : String(e)
	} finally {
		analyzing.value = false
	}
}

async function onModalImportJava(report: ModpackAnalysis) {
	if (!selectedPackPath.value) return
	creatingProfile.value = true
	creationError.value = null
	try {
		await install_create_modpack_instance({
			type: 'fromFile',
			path: selectedPackPath.value,
		})
		creationSuccess.value = `Modpack "${report.pack_name}" imported successfully as standard Java Edition instance!`
		await fetchInstances()
	} catch (e: unknown) {
		creationError.value = e instanceof Error ? e.message : String(e)
	} finally {
		creatingProfile.value = false
	}
}

function onModalImportRust(report: ModpackAnalysis) {
	analysis.value = report
	profileName.value = `${report.pack_name} (Gradle)`
}

function handleClear() {
	selectedPackPath.value = null
	analysis.value = null
	profileName.value = ''
	searchQuery.value = ''
	filter.value = 'all'
	creationError.value = null
	creationSuccess.value = null
}

async function handleCreateProfile() {
	if (!selectedPackPath.value || !profileName.value.trim()) return

	creatingProfile.value = true
	creationError.value = null

	try {
		// 1. Create the Horizon instance
		const job = await install_create_instance({
			name: profileName.value.trim(),
			gameVersion: '26.3',
			loader: 'horizon',
			loaderVersion: null,
			iconPath: null,
		})

		// Await instance resolution
		const instancesList = await list()
		const createdInstance =
			instancesList.find((i) => i.name === profileName.value.trim()) ??
			(job ? await get(job.job_id).catch(() => null) : null)

		if (!createdInstance) {
			throw new Error('Failed to locate newly created Horizon instance directory.')
		}

		// 2. Extract compatible assets (resourcepacks, datapacks, configs, shaders)
		const extracted = await extractModpackAssets(selectedPackPath.value, createdInstance.path)

		creationSuccess.value = `Profile created with ${extracted} compatible data and asset files!`

		// 3. Navigate to instance page
		setTimeout(() => {
			router.push(`/instance/${encodeURIComponent(createdInstance.id)}`)
		}, 800)
	} catch (e: unknown) {
		creationError.value = e instanceof Error ? e.message : String(e)
	} finally {
		creatingProfile.value = false
	}
}

onMounted(() => {
	fetchInstances()
})
</script>
