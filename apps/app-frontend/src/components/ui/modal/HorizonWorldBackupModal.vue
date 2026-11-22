<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="!isBackingUp"
		max-width="560px"
		width="560px"
	>
		<div class="flex w-full flex-col gap-5">
			<!-- Warning Banner -->
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

			<!-- Worlds to be backed up -->
			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold text-secondary uppercase tracking-wider">
					{{ formatMessage(messages.worldsListTitle, { count: worldsToBackup.length }) }}
				</span>
				<div class="max-h-48 flex flex-col gap-1.5 overflow-y-auto rounded-xl border border-solid border-surface-5 bg-surface-1 p-2">
					<div
						v-for="world in worldsToBackup"
						:key="world.path"
						class="flex items-center justify-between rounded-lg bg-surface-2 px-3 py-2 text-xs"
					>
						<span class="font-medium text-contrast truncate max-w-[320px]">
							{{ world.name }}
						</span>
						<span class="text-xs text-secondary font-mono">
							{{ world.path }}
						</span>
					</div>
				</div>
			</div>

			<!-- Backing up progress -->
			<div v-if="isBackingUp" class="flex items-center gap-3 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3 text-xs">
				<SpinnerIcon class="size-4 animate-spin text-brand" />
				<span class="text-contrast font-medium">
					{{ formatMessage(messages.backingUpStatus) }}
				</span>
			</div>

			<!-- Actions -->
			<div class="flex items-center justify-between gap-3 pt-2">
				<Button
					type="quiet"
					size="lg"
					:disabled="isBackingUp"
					@click="handleCancel"
				>
					{{ formatMessage(messages.cancel) }}
				</Button>

				<div class="flex items-center gap-2">
					<Button
						type="outlined"
						size="lg"
						:disabled="isBackingUp"
						@click="handlePlayWithJava"
					>
						{{ formatMessage(messages.playWithJavaInstead) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						size="lg"
						:disabled="isBackingUp"
						@click="handleConfirmBackupAndLaunch"
					>
						<ArchiveIcon class="size-4" />
						{{ formatMessage(messages.backupAndLaunch) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { ArchiveIcon, SpinnerIcon, TriangleAlertIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import type { SingleplayerWorld } from '@/helpers/worlds'

const messages = defineMessages({
	title: {
		id: 'app.horizon.world-backup.title',
		defaultMessage: 'World Save Upgrade Warning',
	},
	bannerTitle: {
		id: 'app.horizon.world-backup.banner-title',
		defaultMessage: 'Gradle upgrades saves permanently (one-way)',
	},
	bannerDescription: {
		id: 'app.horizon.world-backup.banner-desc',
		defaultMessage:
			'Gradle upgrades world saves to the Minecraft 26.3 format, which cannot be loaded in older Minecraft versions without backup. Gradle will create an automatic backup now before launch.',
	},
	worldsListTitle: {
		id: 'app.horizon.world-backup.worlds-title',
		defaultMessage: '{count} World(s) to be backed up:',
	},
	backingUpStatus: {
		id: 'app.horizon.world-backup.backing-up',
		defaultMessage: 'Creating automatic world backup...',
	},
	cancel: {
		id: 'common.cancel',
		defaultMessage: 'Cancel',
	},
	playWithJavaInstead: {
		id: 'app.horizon.world-backup.play-java',
		defaultMessage: 'Play with Java instead',
	},
	backupAndLaunch: {
		id: 'app.horizon.world-backup.confirm',
		defaultMessage: 'Backup & Launch Gradle',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal> | null>(null)
const worldsToBackup = ref<SingleplayerWorld[]>([])
const isBackingUp = ref(false)

let onConfirmLaunch: (() => Promise<void>) | null = null
let onPlayJava: (() => void) | null = null
let onCancel: (() => void) | null = null

function show(
	worlds: SingleplayerWorld[],
	handlers: {
		confirm: () => Promise<void>
		playJava: () => void
		cancel: () => void
	},
) {
	worldsToBackup.value = worlds
	onConfirmLaunch = handlers.confirm
	onPlayJava = handlers.playJava
	onCancel = handlers.cancel
	modalRef.value?.show()
}

async function handleConfirmBackupAndLaunch() {
	isBackingUp.value = true
	try {
		if (onConfirmLaunch) {
			await onConfirmLaunch()
		}
		modalRef.value?.hide()
	} finally {
		isBackingUp.value = false
	}
}

function handlePlayWithJava() {
	modalRef.value?.hide()
	onPlayJava?.()
}

function handleCancel() {
	modalRef.value?.hide()
	onCancel?.()
}

defineExpose({
	show,
	hide: () => modalRef.value?.hide(),
})
</script>
