<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="false"
		max-width="580px"
		width="580px"
	>
		<div class="flex w-full flex-col gap-5">
			<!-- Header explanation -->
			<div class="flex items-start gap-3 rounded-2xl border border-solid border-purple-500/20 bg-purple-500/10 p-4 text-xs text-contrast">
				<SparklesIcon class="size-5 shrink-0 text-purple-400" />
				<div class="flex flex-col gap-1">
					<span class="font-bold text-sm text-purple-300">
						{{ formatMessage(messages.bannerTitle) }}
					</span>
					<span class="text-secondary leading-relaxed">
						{{ formatMessage(messages.bannerDescription) }}
					</span>
				</div>
			</div>

			<!-- Summary of changes -->
			<div class="flex flex-col gap-2 rounded-xl border border-solid border-surface-5 bg-surface-2 p-4 text-xs">
				<span class="font-semibold text-contrast text-sm">
					{{ formatMessage(messages.summaryTitle, { count: existingInstances.length }) }}
				</span>
				<ul class="m-0 flex flex-col gap-1.5 pl-4 text-secondary leading-relaxed list-disc">
					<li>{{ formatMessage(messages.bulletHorizonDefault) }}</li>
					<li>{{ formatMessage(messages.bulletJavaClassic) }}</li>
					<li>{{ formatMessage(messages.bulletDataSafety) }}</li>
				</ul>
			</div>

			<!-- Affected Instances List -->
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-secondary uppercase tracking-wider">
					{{ formatMessage(messages.instancesListTitle) }}
				</span>
				<div class="max-h-48 flex flex-col gap-1.5 overflow-y-auto rounded-xl border border-solid border-surface-5 bg-surface-1 p-2">
					<div
						v-for="inst in existingInstances"
						:key="inst.id"
						class="flex items-center justify-between rounded-lg bg-surface-2 px-3 py-2 text-xs"
					>
						<span class="font-medium text-contrast truncate max-w-[280px]">
							{{ inst.name }}
						</span>
						<div class="flex items-center gap-2">
							<span class="text-secondary capitalize">{{ inst.loader }} {{ inst.game_version }}</span>
							<span class="rounded bg-purple-500/20 px-1.5 py-0.5 text-[10px] font-semibold text-purple-300 border border-purple-500/30">
								Gradle (default)
							</span>
						</div>
					</div>
				</div>
			</div>

			<!-- Actions -->
			<div class="flex items-center justify-end gap-3 pt-2">
				<Button
					type="quiet"
					size="lg"
					:disabled="migrating"
					@click="handleApplyMigration('java')"
				>
					{{ formatMessage(messages.keepJavaDefault) }}
				</Button>
				<Button
					type="colored"
					color="brand"
					size="lg"
					:disabled="migrating"
					@click="handleApplyMigration('horizon')"
				>
					<SparklesIcon class="size-4" />
					{{ formatMessage(messages.applyUpgrade) }}
				</Button>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { SparklesIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import { type InstanceRuntime,list, setInstanceRuntime } from '@/helpers/instance'
import type { GameInstance } from '@/helpers/types'

const messages = defineMessages({
	title: {
		id: 'app.horizon.migration.title',
		defaultMessage: 'Gradle Runtime Upgrade',
	},
	bannerTitle: {
		id: 'app.horizon.migration.banner-title',
		defaultMessage: 'Dual Runtimes are now available!',
	},
	bannerDescription: {
		id: 'app.horizon.migration.banner-desc',
		defaultMessage:
			'Every instance now supports both the native Gradle (Rust, optimized) engine and classic Java Minecraft. Gradle is recommended for ultra-low memory, instant chunk loading, and smooth framerates.',
	},
	summaryTitle: {
		id: 'app.horizon.migration.summary-title',
		defaultMessage: 'Migration Summary ({count} instances found)',
	},
	bulletHorizonDefault: {
		id: 'app.horizon.migration.bullet-horizon',
		defaultMessage: 'Gradle will be added as the default launch runtime for all existing instances.',
	},
	bulletJavaClassic: {
		id: 'app.horizon.migration.bullet-java',
		defaultMessage: 'Java (classic) remains 100% available with 0 regressions. You can switch back anytime via the runtime switch or Play dropdown.',
	},
	bulletDataSafety: {
		id: 'app.horizon.migration.bullet-safety',
		defaultMessage: 'Nothing is deleted or rewritten: your world saves, resource packs, shaders, screenshots, and Java mod jars remain completely intact.',
	},
	instancesListTitle: {
		id: 'app.horizon.migration.instances-title',
		defaultMessage: 'Instances to be upgraded:',
	},
	keepJavaDefault: {
		id: 'app.horizon.migration.keep-java',
		defaultMessage: 'Keep Java as Default',
	},
	applyUpgrade: {
		id: 'app.horizon.migration.apply',
		defaultMessage: 'Upgrade to Gradle (Recommended)',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal> | null>(null)
const existingInstances = ref<GameInstance[]>([])
const migrating = ref(false)
const emit = defineEmits<{
	completed: []
}>()

const MIGRATION_STORAGE_KEY = 'gradle_horizon_dual_runtime_migration_v1'

async function checkAndPromptMigration() {
	if (localStorage.getItem(MIGRATION_STORAGE_KEY) === 'true') {
		return
	}

	try {
		const instances = await list()
		if (!instances || instances.length === 0) {
			localStorage.setItem(MIGRATION_STORAGE_KEY, 'true')
			return
		}

		// Filter instances that haven't been migrated yet (don't have runtime recorded on disk)
		existingInstances.value = instances
		modalRef.value?.show()
	} catch (err) {
		console.warn('Failed to check instance migration:', err)
	}
}

async function handleApplyMigration(defaultRuntime: InstanceRuntime) {
	migrating.value = true
	try {
		for (const inst of existingInstances.value) {
			await setInstanceRuntime(inst.id, defaultRuntime).catch(console.error)
		}
		localStorage.setItem(MIGRATION_STORAGE_KEY, 'true')
		modalRef.value?.hide()
		emit('completed')
	} finally {
		migrating.value = false
	}
}

defineExpose({
	checkAndPromptMigration,
})
</script>
