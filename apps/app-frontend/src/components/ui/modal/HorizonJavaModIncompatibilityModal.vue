<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="true"
		max-width="580px"
		width="580px"
	>
		<div class="flex w-full flex-col gap-5">
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

			<!-- Incompatible Mods List -->
			<div class="flex flex-col gap-2">
				<div class="flex items-center justify-between text-xs">
					<span class="font-semibold text-secondary uppercase tracking-wider">
						{{ formatMessage(messages.modsListTitle, { count: incompatibleMods.length }) }}
					</span>
					<span class="text-secondary">
						{{ formatMessage(messages.jarModsNotice) }}
					</span>
				</div>

				<div class="max-h-56 flex flex-col gap-1.5 overflow-y-auto rounded-xl border border-solid border-surface-5 bg-surface-1 p-2">
					<div
						v-for="mod in incompatibleMods"
						:key="mod.file_path"
						class="flex items-center justify-between rounded-lg bg-surface-2 px-3 py-2 text-xs"
					>
						<div class="flex items-center gap-2.5 truncate max-w-[340px]">
							<Avatar
								:src="mod.project?.icon_url"
								size="24px"
								class="shrink-0 rounded-md"
								alt=""
							/>
							<div class="flex flex-col truncate">
								<span class="font-medium text-contrast truncate">
									{{ mod.project?.title || mod.file_name }}
								</span>
								<span class="text-[10px] text-secondary truncate font-mono">
									{{ mod.file_name }}
								</span>
							</div>
						</div>
						<span class="shrink-0 rounded bg-amber-500/15 px-2 py-0.5 text-[10px] font-semibold text-amber-300 border border-amber-500/30">
							Java JAR
						</span>
					</div>
				</div>
			</div>

			<!-- Notice that data packs and resource packs still run -->
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
						type="outlined"
						size="lg"
						@click="handleContinueHorizon"
					>
						{{ formatMessage(messages.launchHorizonAnyway) }}
					</Button>
					<Button
						type="colored"
						color="brand"
						size="lg"
						@click="handlePlayWithJava"
					>
						<PlayIcon class="size-4" />
						{{ formatMessage(messages.playWithJavaInstead) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { PlayIcon, TriangleAlertIcon } from '@modrinth/assets'
import { Avatar, Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { ref } from 'vue'

import type { ContentItem } from '@/helpers/types'

const messages = defineMessages({
	title: {
		id: 'app.horizon.compat.title',
		defaultMessage: 'Java Mods Incompatible with Horizon',
	},
	bannerTitle: {
		id: 'app.horizon.compat.banner-title',
		defaultMessage: 'Java mod jars cannot execute on Horizon',
	},
	bannerDescription: {
		id: 'app.horizon.compat.banner-desc',
		defaultMessage:
			'Horizon is a native Rust engine for Minecraft 26.3. Standard Java bytecode mods (Forge, NeoForge, Fabric, Quilt) require the Java Virtual Machine. Horizon runs native WebAssembly mods (.wasm, .hmod), shaders, resource packs, and data packs.',
	},
	modsListTitle: {
		id: 'app.horizon.compat.mods-title',
		defaultMessage: '{count} Java mod(s) detected:',
	},
	jarModsNotice: {
		id: 'app.horizon.compat.jar-notice',
		defaultMessage: 'Will not load in Horizon',
	},
	compatiblePacksNote: {
		id: 'app.horizon.compat.packs-note',
		defaultMessage:
			'Your resource packs, shaders, and data packs will continue to work normally in Horizon. To play with Java mod jars, choose "Play with Java instead".',
	},
	cancel: {
		id: 'common.cancel',
		defaultMessage: 'Cancel',
	},
	launchHorizonAnyway: {
		id: 'app.horizon.compat.launch-anyway',
		defaultMessage: 'Launch Horizon (Skip JARs)',
	},
	playWithJavaInstead: {
		id: 'app.horizon.compat.play-java',
		defaultMessage: 'Play with Java instead',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal> | null>(null)
const incompatibleMods = ref<ContentItem[]>([])

let onPlayJava: (() => void) | null = null
let onContinueHorizon: (() => void) | null = null
let onCancelLaunch: (() => void) | null = null

function show(
	mods: ContentItem[],
	handlers: {
		playJava: () => void
		continueHorizon: () => void
		cancel: () => void
	},
) {
	incompatibleMods.value = mods
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
	hide: () => modalRef.value?.hide(),
})
</script>
