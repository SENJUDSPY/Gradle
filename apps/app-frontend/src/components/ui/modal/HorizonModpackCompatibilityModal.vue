<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="true"
		max-width="640px"
		width="640px"
	>
		<div v-if="analysis" class="flex w-full flex-col gap-4">
			<!-- Pack summary header -->
			<div class="flex items-center justify-between rounded-xl bg-surface-2 p-3 text-xs">
				<div class="flex flex-col gap-0.5">
					<span class="text-sm font-bold text-contrast">{{ analysis.pack_name }}</span>
					<span class="text-secondary">
						Version {{ analysis.pack_version }} &bull; {{ analysis.original_loader }} (MC {{ analysis.original_game_version }})
					</span>
				</div>
				<div class="flex items-center gap-1.5">
					<span
						:class="[
							'rounded-full px-2.5 py-1 text-xs font-semibold border',
							levelBadgeClass,
						]"
					>
						{{ levelBadgeText }}
					</span>
				</div>
			</div>

			<!-- Compatibility Level Banner -->
			<div :class="['flex items-start gap-3 rounded-xl border p-3.5 text-xs', levelBannerClass]">
				<TriangleAlertIcon v-if="analysis.compatibility_level !== 'Supported'" class="size-5 shrink-0 mt-0.5" />
				<CheckIcon v-else class="size-5 shrink-0 mt-0.5 text-green-400" />
				<div class="flex flex-col gap-1">
					<span class="font-bold text-sm">{{ levelTitleText }}</span>
					<span class="text-secondary leading-relaxed">{{ analysis.compatibility_summary }}</span>
				</div>
			</div>

			<!-- Counts summary cards -->
			<div class="grid grid-cols-3 gap-2">
				<button
					type="button"
					:class="[
						'flex flex-col items-center justify-center rounded-xl border p-2.5 transition-all text-center',
						filter === 'all'
							? 'border-purple-500 bg-purple-500/10'
							: 'border-surface-5 bg-surface-2 hover:bg-surface-3',
					]"
					@click="filter = 'all'"
				>
					<span class="text-lg font-bold text-contrast">{{ analysis.total_items }}</span>
					<span class="text-[11px] text-secondary">Total Content</span>
				</button>
				<button
					type="button"
					:class="[
						'flex flex-col items-center justify-center rounded-xl border p-2.5 transition-all text-center',
						filter === 'compatible'
							? 'border-green-500 bg-green-500/10'
							: 'border-surface-5 bg-surface-2 hover:bg-surface-3',
					]"
					@click="filter = 'compatible'"
				>
					<span class="text-lg font-bold text-green-400">{{ analysis.compatible_count }}</span>
					<span class="text-[11px] text-secondary">Compatible Assets</span>
				</button>
				<button
					type="button"
					:class="[
						'flex flex-col items-center justify-center rounded-xl border p-2.5 transition-all text-center',
						filter === 'incompatible'
							? 'border-amber-500 bg-amber-500/10'
							: 'border-surface-5 bg-surface-2 hover:bg-surface-3',
					]"
					@click="filter = 'incompatible'"
				>
					<span class="text-lg font-bold text-amber-400">{{ analysis.incompatible_count }}</span>
					<span class="text-[11px] text-secondary">Skipped Java Mods</span>
				</button>
			</div>

			<!-- Items List -->
			<div class="flex flex-col gap-1.5">
				<div class="flex items-center justify-between text-xs text-secondary px-1">
					<span>Content Breakdown ({{ filteredItems.length }} items)</span>
					<span class="text-[10px] text-secondary/80">Java .jar mods require JVM</span>
				</div>

				<div class="max-h-60 flex flex-col gap-1.5 overflow-y-auto rounded-xl border border-surface-5 bg-surface-1 p-2">
					<div
						v-for="item in filteredItems"
						:key="item.path"
						class="flex items-center justify-between rounded-lg bg-surface-2 px-3 py-2 text-xs"
					>
						<div class="flex items-center gap-2.5 truncate max-w-[380px]">
							<span
								:class="[
									'size-2 rounded-full shrink-0',
									item.is_compatible ? 'bg-green-400' : 'bg-amber-400',
								]"
							/>
							<div class="flex flex-col truncate">
								<span class="font-medium text-contrast truncate">{{ item.name }}</span>
								<span class="text-[10px] text-secondary truncate font-mono">{{ item.path }}</span>
							</div>
						</div>

						<div class="flex items-center gap-2 shrink-0">
							<span
								v-if="item.is_compatible"
								class="rounded bg-green-500/15 px-2 py-0.5 text-[10px] font-semibold text-green-300 border border-green-500/30"
							>
								{{ formatCategory(item.category) }}
							</span>
							<span
								v-else
								class="rounded bg-amber-500/15 px-2 py-0.5 text-[10px] font-semibold text-amber-300 border border-amber-500/30"
							>
								Java Bytecode &bull; Skipped
							</span>
						</div>
					</div>

					<div v-if="filteredItems.length === 0" class="py-6 text-center text-xs text-secondary">
						No items in this category.
					</div>
				</div>
			</div>

			<!-- Action buttons -->
			<div class="flex items-center justify-between gap-3 pt-2">
				<Button type="quiet" size="lg" @click="handleCancel">
					Cancel
				</Button>

				<div class="flex items-center gap-2">
					<Button
						v-if="analysis.compatible_count > 0"
						type="outlined"
						size="lg"
						@click="handleImportRust"
					>
						Import into Rust Edition
					</Button>
					<Button
						type="colored"
						color="brand"
						size="lg"
						@click="handleImportJava"
					>
						<PlayIcon class="size-4" />
						Import into Java Edition
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, PlayIcon, TriangleAlertIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { computed, ref } from 'vue'

import type { ModpackAnalysis } from '@/helpers/horizon'

const emit = defineEmits<{
	(e: 'import-rust', analysis: ModpackAnalysis): void
	(e: 'import-java', analysis: ModpackAnalysis): void
	(e: 'cancel'): void
}>()

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal> | null>(null)
const analysis = ref<ModpackAnalysis | null>(null)
const filter = ref<'all' | 'compatible' | 'incompatible'>('all')

const messages = defineMessages({
	title: {
		id: 'modpack.compatibility.modal.title',
		defaultMessage: 'Modpack Compatibility Report',
	},
})

const levelBadgeClass = computed(() => {
	switch (analysis.value?.compatibility_level) {
		case 'Supported':
			return 'bg-green-500/20 text-green-300 border-green-500/40'
		case 'PartiallySupported':
			return 'bg-amber-500/20 text-amber-300 border-amber-500/40'
		default:
			return 'bg-red-500/20 text-red-300 border-red-500/40'
	}
})

const levelBadgeText = computed(() => {
	switch (analysis.value?.compatibility_level) {
		case 'Supported':
			return 'Fully Supported'
		case 'PartiallySupported':
			return 'Partially Supported'
		default:
			return 'Unsupported on Rust'
	}
})

const levelBannerClass = computed(() => {
	switch (analysis.value?.compatibility_level) {
		case 'Supported':
			return 'border-green-500/30 bg-green-500/10 text-green-300'
		case 'PartiallySupported':
			return 'border-amber-500/30 bg-amber-500/10 text-amber-300'
		default:
			return 'border-red-500/30 bg-red-500/10 text-red-300'
	}
})

const levelTitleText = computed(() => {
	switch (analysis.value?.compatibility_level) {
		case 'Supported':
			return 'Compatible with Gradle Rust Edition'
		case 'PartiallySupported':
			return 'Compatible with skipped Java mods'
		default:
			return 'Requires Java Virtual Machine (JVM)'
	}
})

const filteredItems = computed(() => {
	if (!analysis.value) return []
	switch (filter.value) {
		case 'compatible':
			return analysis.value.items.filter((i) => i.is_compatible)
		case 'incompatible':
			return analysis.value.items.filter((i) => !i.is_compatible)
		default:
			return analysis.value.items
	}
})

function formatCategory(category: string): string {
	switch (category) {
		case 'ResourcePack':
			return 'Resource Pack'
		case 'DataPack':
			return 'Data Pack'
		case 'ShaderPack':
			return 'Shaders'
		case 'Configuration':
			return 'Config'
		case 'AssetMod':
			return 'Asset Pack'
		case 'JavaBytecodeMod':
			return 'Java Bytecode'
		default:
			return category
	}
}

function show(report: ModpackAnalysis) {
	analysis.value = report
	filter.value = 'all'
	modalRef.value?.show()
}

function hide() {
	modalRef.value?.hide()
}

function handleImportRust() {
	if (analysis.value) {
		emit('import-rust', analysis.value)
		hide()
	}
}

function handleImportJava() {
	if (analysis.value) {
		emit('import-java', analysis.value)
		hide()
	}
}

function handleCancel() {
	emit('cancel')
	hide()
}

defineExpose({ show, hide })
</script>
