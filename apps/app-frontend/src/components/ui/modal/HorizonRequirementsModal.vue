<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="true"
		max-width="560px"
		width="560px"
	>
		<div class="flex w-full flex-col gap-5">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.subtitle) }}
			</p>

			<div v-if="loading" class="flex items-center justify-center py-8">
				<SpinnerIcon class="size-6 animate-spin text-brand" />
			</div>

			<div v-else-if="report" class="flex flex-col gap-3">
				<!-- Minecraft 26.3 Copy Check -->
				<div
					class="flex flex-col gap-1.5 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
				>
					<div class="flex items-center justify-between">
						<div class="flex items-center gap-2">
							<CheckCircleIcon v-if="report.minecraft_26_3.passed" class="size-5 text-green-500" />
							<CircleAlertIcon v-else class="size-5 text-red-500" />
							<span class="font-semibold text-contrast">{{ report.minecraft_26_3.title }}</span>
						</div>
						<Button
							v-if="!report.minecraft_26_3.passed"
							size="sm"
							type="outlined"
							@click="pickCustomJar"
						>
							<FolderOpenIcon />
							{{ formatMessage(messages.chooseJar) }}
						</Button>
					</div>
					<p class="m-0 text-xs text-secondary">{{ report.minecraft_26_3.details }}</p>
					<div
						v-if="report.minecraft_26_3.path"
						class="break-all rounded bg-surface-1 px-2 py-1 font-mono text-[11px] text-secondary"
					>
						{{ report.minecraft_26_3.path }}
					</div>
					<div v-if="report.minecraft_26_3.error" class="text-xs text-red-400">
						{{ report.minecraft_26_3.error }}
					</div>
				</div>

				<!-- Java 25 Check -->
				<div
					class="flex flex-col gap-1.5 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
				>
					<div class="flex items-center gap-2">
						<CheckCircleIcon v-if="report.java_25.passed" class="size-5 text-green-500" />
						<CircleAlertIcon v-else class="size-5 text-red-500" />
						<span class="font-semibold text-contrast">{{ report.java_25.title }}</span>
					</div>
					<p class="m-0 text-xs text-secondary">{{ report.java_25.details }}</p>
					<div
						v-if="report.java_25.path"
						class="break-all rounded bg-surface-1 px-2 py-1 font-mono text-[11px] text-secondary"
					>
						{{ report.java_25.path }}
					</div>
					<div v-if="report.java_25.error" class="text-xs text-red-400">
						{{ report.java_25.error }}
					</div>
				</div>

				<!-- GPU Vulkan / Metal Check -->
				<div
					class="flex flex-col gap-1.5 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
				>
					<div class="flex items-center gap-2">
						<CheckCircleIcon v-if="report.gpu_vulkan.passed" class="size-5 text-green-500" />
						<CircleAlertIcon v-else class="size-5 text-red-500" />
						<span class="font-semibold text-contrast">{{ report.gpu_vulkan.title }}</span>
					</div>
					<p class="m-0 text-xs text-secondary">{{ report.gpu_vulkan.details }}</p>
					<div v-if="report.gpu_vulkan.error" class="text-xs text-red-400">
						{{ report.gpu_vulkan.error }}
					</div>
				</div>

				<!-- Overall Status -->
				<div
					:class="[
						'mt-2 flex items-center gap-2 rounded-xl p-3 text-sm font-medium',
						report.all_passed ? 'bg-green-500/10 text-green-400' : 'bg-orange/10 text-orange',
					]"
				>
					<CheckCircleIcon v-if="report.all_passed" class="size-5 shrink-0" />
					<TriangleAlertIcon v-else class="size-5 shrink-0" />
					<span>
						{{
							report.all_passed
								? formatMessage(messages.allPassed)
								: formatMessage(messages.someMissing)
						}}
					</span>
				</div>
			</div>

			<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
				<Button type="outlined" :disabled="loading" @click="runCheck(selectedCustomJar)">
					<RefreshCwIcon :class="{ 'animate-spin': loading }" />
					{{ formatMessage(messages.refresh) }}
				</Button>
				<div class="flex items-center gap-2">
					<Button
						v-if="report?.minecraft_26_3.passed"
						type="colored"
						color="brand"
						@click="emit('prepare', selectedCustomJar)"
					>
						<WrenchIcon />
						{{ formatMessage(messages.prepareFiles) }}
					</Button>
					<Button type="quiet" @click="hide">
						{{ formatMessage(commonMessages.closeButton) }}
					</Button>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckCircleIcon,
	CircleAlertIcon,
	FolderOpenIcon,
	RefreshCwIcon,
	SpinnerIcon,
	TriangleAlertIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { ref } from 'vue'

import { checkHorizonRequirements, type SystemRequirementsReport } from '@/helpers/horizon'

const emit = defineEmits<{
	(e: 'prepare', jar: string | null): void
}>()

const messages = defineMessages({
	title: {
		id: 'app.horizon.requirements.title',
		defaultMessage: 'Horizon System Requirements',
	},
	subtitle: {
		id: 'app.horizon.native-rust',
		defaultMessage: 'Minecraft Java 26.3 rewritten in native Rust (no JVM)',
	},
	chooseJar: {
		id: 'app.horizon.requirements.choose-jar',
		defaultMessage: 'Choose 26.3.jar...',
	},
	allPassed: {
		id: 'app.horizon.requirements.all-passed',
		defaultMessage: 'All requirements met. Horizon is ready to play.',
	},
	someMissing: {
		id: 'app.horizon.requirements.missing',
		defaultMessage: 'One or more requirements are missing.',
	},
	refresh: {
		id: 'app.horizon.requirements.check',
		defaultMessage: 'Check Requirements',
	},
	prepareFiles: {
		id: 'instance.action.prepare-resources',
		defaultMessage: 'Prepare Game Files',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal>>()
const loading = ref(false)
const report = ref<SystemRequirementsReport | null>(null)
const selectedCustomJar = ref<string | null>(null)

async function runCheck(customJar?: string | null) {
	loading.value = true
	try {
		report.value = await checkHorizonRequirements(customJar)
	} catch (e) {
		console.error('Failed to check Horizon requirements:', e)
	} finally {
		loading.value = false
	}
}

async function pickCustomJar() {
	const file = await open({
		multiple: false,
		filters: [{ name: 'Minecraft Client Jar', extensions: ['jar'] }],
	})
	if (typeof file === 'string') {
		selectedCustomJar.value = file
		await runCheck(file)
	}
}

function show(customJar?: string | null) {
	selectedCustomJar.value = customJar ?? null
	modalRef.value?.show()
	void runCheck(customJar)
}

function hide() {
	modalRef.value?.hide()
}

defineExpose({
	show,
	hide,
	runCheck,
	report,
})
</script>
