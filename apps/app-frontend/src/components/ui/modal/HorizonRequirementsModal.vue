<template>
	<NewModal
		ref="modalRef"
		:header="formatMessage(messages.title)"
		:closable="true"
		max-width="640px"
		width="640px"
	>
		<div class="flex w-full flex-col gap-4">
			<!-- Tab Switcher -->
			<div class="flex border-b border-solid border-surface-5 pb-2">
				<div class="flex gap-2">
					<button
						type="button"
						:class="[
							'cursor-pointer rounded-lg px-3 py-1.5 text-sm font-medium transition-colors border-0',
							activeTab === 'checks'
								? 'bg-brand text-white'
								: 'bg-surface-3 text-secondary hover:text-contrast',
						]"
						@click="activeTab = 'checks'"
					>
						{{ formatMessage(messages.tabChecks) }}
					</button>
					<button
						type="button"
						:class="[
							'cursor-pointer rounded-lg px-3 py-1.5 text-sm font-medium transition-colors border-0',
							activeTab === 'diagnostics'
								? 'bg-brand text-white'
								: 'bg-surface-3 text-secondary hover:text-contrast',
						]"
						@click="switchToDiagnostics"
					>
						{{ formatMessage(messages.tabDiagnostics) }}
					</button>
				</div>
			</div>

			<!-- TAB 1: PREFLIGHT CHECKS -->
			<div v-if="activeTab === 'checks'" class="flex flex-col gap-4">
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.subtitle) }}
				</p>

				<div v-if="loading" class="flex items-center justify-center py-8">
					<SpinnerIcon class="size-6 animate-spin text-brand" />
				</div>

				<div v-else-if="report" class="flex max-h-[460px] flex-col gap-3 overflow-y-auto pr-1">
					<!-- Check List -->
					<div
						v-for="item in checkList"
						:key="item.key"
						class="flex flex-col gap-1.5 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3.5"
					>
						<div class="flex items-center justify-between gap-2">
							<div class="flex items-center gap-2">
								<CheckCircleIcon v-if="item.check.passed" class="size-5 shrink-0 text-green-500" />
								<CircleAlertIcon v-else class="size-5 shrink-0 text-red-500" />
								<span class="font-semibold text-contrast">{{ item.check.title }}</span>
								<span
									v-if="item.optional"
									class="rounded bg-surface-4 px-1.5 py-0.5 text-[10px] uppercase font-semibold text-secondary"
								>
									{{ formatMessage(messages.optionalBadge) }}
								</span>
							</div>

							<!-- Actionable Fix Button -->
							<div v-if="!item.check.passed && item.check.fix_action" class="shrink-0">
								<Button
									v-if="item.check.fix_action === 'choose_jar'"
									size="sm"
									type="outlined"
									@click="pickCustomJar"
								>
									<FolderOpenIcon />
									{{ formatMessage(messages.chooseJar) }}
								</Button>
								<Button
									v-else-if="item.check.fix_action === 'open_url' && item.check.fix_url"
									size="sm"
									type="outlined"
									@click="openUrl(item.check.fix_url)"
								>
									<ExternalIcon />
									{{ formatMessage(messages.installFix) }}
								</Button>
								<Button
									v-else-if="item.check.fix_action === 'prepare_resources'"
									size="sm"
									type="colored"
									color="brand"
									@click="emit('prepare', selectedCustomJar)"
								>
									<WrenchIcon />
									{{ formatMessage(messages.prepareFiles) }}
								</Button>
								<Button
									v-else-if="item.check.fix_action === 'install_runtime'"
									size="sm"
									type="colored"
									color="brand"
									@click="emit('install-runtime')"
								>
									<DownloadIcon />
									{{ formatMessage(messages.installRuntime) }}
								</Button>
								<Button
									v-else-if="item.check.fix_action === 'login'"
									size="sm"
									type="outlined"
									@click="emit('login')"
								>
									{{ formatMessage(messages.signIn) }}
								</Button>
								<Button
									v-else-if="item.check.fix_action === 'open_dir' && item.check.fix_url"
									size="sm"
									type="outlined"
									@click="openFolder(item.check.fix_url)"
								>
									<FolderOpenIcon />
									{{ formatMessage(messages.openFolder) }}
								</Button>
							</div>
						</div>

						<p class="m-0 text-xs text-secondary">{{ item.check.details }}</p>

						<div
							v-if="item.check.path"
							class="break-all rounded bg-surface-1 px-2 py-1 font-mono text-[11px] text-secondary"
						>
							{{ item.check.path }}
						</div>

						<div v-if="item.check.error" class="text-xs text-red-400">
							{{ item.check.error }}
						</div>
					</div>

					<!-- Overall Status Banner -->
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

				<!-- Actions Bar -->
				<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
					<Button type="outlined" :disabled="loading" @click="runCheck(selectedCustomJar)">
						<RefreshCwIcon :class="{ 'animate-spin': loading }" />
						{{ formatMessage(messages.refresh) }}
					</Button>
					<div class="flex items-center gap-2">
						<Button
							v-if="!report?.resources_prepared.passed && (report?.minecraft_26_3.passed || selectedCustomJar)"
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

			<!-- TAB 2: DIAGNOSTICS PAGE -->
			<div v-else-if="activeTab === 'diagnostics'" class="flex flex-col gap-4">
				<p class="m-0 text-sm text-secondary">
					{{ formatMessage(messages.diagnosticsSubtitle) }}
				</p>

				<div v-if="diagnosticsLoading" class="flex items-center justify-center py-8">
					<SpinnerIcon class="size-6 animate-spin text-brand" />
				</div>

				<div v-else-if="diagnostics" class="flex max-h-[460px] flex-col gap-3 overflow-y-auto pr-1">
					<!-- System & OS -->
					<div class="flex flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3">
						<span class="text-xs font-semibold uppercase tracking-wider text-secondary">Operating System</span>
						<span class="text-sm font-medium text-contrast">{{ diagnostics.os_info }}</span>
					</div>

					<!-- GPU & Vulkan -->
					<div class="flex flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3">
						<span class="text-xs font-semibold uppercase tracking-wider text-secondary">GPU & Graphics API</span>
						<p class="m-0 text-sm font-medium text-contrast">{{ diagnostics.requirements.gpu_vulkan.details }}</p>
						<div
							v-if="diagnostics.gpu_info && diagnostics.gpu_info !== 'vulkaninfo unavailable'"
							class="mt-1 max-h-32 overflow-y-auto rounded bg-surface-1 p-2 font-mono text-[11px] text-secondary"
						>
							<pre class="m-0 whitespace-pre-wrap">{{ diagnostics.gpu_info.slice(0, 1500) }}</pre>
						</div>
					</div>

					<!-- Java Runtime -->
					<div class="flex flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3">
						<span class="text-xs font-semibold uppercase tracking-wider text-secondary">Java 25 (Asset Prep)</span>
						<span class="text-sm font-medium text-contrast">{{ diagnostics.java_version }}</span>
					</div>

					<!-- Engine Releases -->
					<div class="flex flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3">
						<span class="text-xs font-semibold uppercase tracking-wider text-secondary">Gradle Engine Releases</span>
						<span class="text-sm font-medium text-contrast">
							{{ diagnostics.installed_horizon_versions.length > 0 ? diagnostics.installed_horizon_versions.join(', ') : 'None installed' }}
						</span>
					</div>

					<!-- Resources Manifest -->
					<div class="flex flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3">
						<span class="text-xs font-semibold uppercase tracking-wider text-secondary">Resources Manifest</span>
						<div
							v-if="diagnostics.resources_manifest"
							class="max-h-28 overflow-y-auto rounded bg-surface-1 p-2 font-mono text-[11px] text-secondary"
						>
							<pre class="m-0 whitespace-pre-wrap">{{ diagnostics.resources_manifest }}</pre>
						</div>
						<span v-else class="text-xs text-orange">Not prepared yet</span>
					</div>

					<!-- Data Directory -->
					<div class="flex flex-col gap-1 rounded-xl border border-solid border-surface-5 bg-surface-2 p-3">
						<span class="text-xs font-semibold uppercase tracking-wider text-secondary">Gradle Rust Data Directory</span>
						<span class="break-all font-mono text-xs text-secondary">{{ diagnostics.horizon_home }}</span>
					</div>
				</div>

				<!-- Diagnostics Footer Actions -->
				<div class="flex items-center justify-between border-t border-solid border-surface-5 pt-3">
					<Button type="outlined" :disabled="diagnosticsLoading" @click="fetchDiagnostics">
						<RefreshCwIcon :class="{ 'animate-spin': diagnosticsLoading }" />
						{{ formatMessage(messages.runDiagnostics) }}
					</Button>
					<div class="flex items-center gap-2">
						<Button type="outlined" :disabled="!diagnostics" @click="copyDiagnostics">
							<CheckIcon v-if="copied" class="text-green-500" />
							<CopyIcon v-else />
							{{ copied ? formatMessage(messages.copied) : formatMessage(messages.copyDiagnostics) }}
						</Button>
						<Button type="colored" color="brand" @click="exportDebug">
							<DownloadIcon />
							{{ formatMessage(messages.exportDebugInfo) }}
						</Button>
						<Button type="quiet" @click="hide">
							{{ formatMessage(commonMessages.closeButton) }}
						</Button>
					</div>
				</div>
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import {
	CheckCircleIcon,
	CheckIcon,
	CircleAlertIcon,
	CopyIcon,
	DownloadIcon,
	ExternalIcon,
	FolderOpenIcon,
	RefreshCwIcon,
	SpinnerIcon,
	TriangleAlertIcon,
	WrenchIcon,
} from '@modrinth/assets'
import { Button, commonMessages, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import { computed, ref } from 'vue'

import { exportDebugInfo } from '@/components/ui/download-manager/debug-info-export'
import {
	checkHorizonRequirements,
	runHorizonDiagnostics,
	type CheckItem,
	type DiagnosticsReport,
	type SystemRequirementsReport,
} from '@/helpers/horizon'
import { openPath } from '@/helpers/utils'

const emit = defineEmits<{
	(e: 'prepare', jar: string | null): void
	(e: 'install-runtime'): void
	(e: 'login'): void
}>()

const messages = defineMessages({
	title: {
		id: 'app.horizon.requirements.title',
		defaultMessage: 'Gradle System Requirements & Diagnostics',
	},
	subtitle: {
		id: 'app.horizon.native-rust',
		defaultMessage: 'Minecraft Java 26.3 rewritten in native Rust (no JVM)',
	},
	tabChecks: {
		id: 'app.horizon.requirements.tab.checks',
		defaultMessage: 'Preflight Checklist',
	},
	tabDiagnostics: {
		id: 'app.horizon.requirements.tab.diagnostics',
		defaultMessage: 'Diagnostics',
	},
	chooseJar: {
		id: 'app.horizon.requirements.choose-jar',
		defaultMessage: 'Choose 26.3.jar...',
	},
	installFix: {
		id: 'app.horizon.requirements.install-fix',
		defaultMessage: 'Install / View Fix',
	},
	installRuntime: {
		id: 'app.horizon.requirements.install-runtime',
		defaultMessage: 'Install Engine',
	},
	signIn: {
		id: 'app.horizon.requirements.sign-in',
		defaultMessage: 'Sign In',
	},
	openFolder: {
		id: 'app.horizon.requirements.open-folder',
		defaultMessage: 'Open Folder',
	},
	optionalBadge: {
		id: 'app.horizon.requirements.optional',
		defaultMessage: 'Optional',
	},
	allPassed: {
		id: 'app.horizon.requirements.all-passed',
		defaultMessage: 'All requirements met. Gradle Rust Edition is ready to play.',
	},
	someMissing: {
		id: 'app.horizon.requirements.missing',
		defaultMessage: 'One or more requirements are missing. Review the items above.',
	},
	refresh: {
		id: 'app.horizon.requirements.check',
		defaultMessage: 'Check Requirements',
	},
	prepareFiles: {
		id: 'instance.action.prepare-resources',
		defaultMessage: 'Prepare Game Files',
	},
	diagnosticsSubtitle: {
		id: 'app.horizon.diagnostics.subtitle',
		defaultMessage: 'System, hardware, and runtime diagnostics for Gradle Rust Edition.',
	},
	runDiagnostics: {
		id: 'app.horizon.diagnostics.run',
		defaultMessage: 'Run Diagnostics',
	},
	copyDiagnostics: {
		id: 'app.horizon.diagnostics.copy',
		defaultMessage: 'Copy Diagnostics',
	},
	copied: {
		id: 'app.horizon.diagnostics.copied',
		defaultMessage: 'Copied!',
	},
	exportDebugInfo: {
		id: 'app.horizon.diagnostics.export',
		defaultMessage: 'Export Debug Info',
	},
})

const { formatMessage } = useVIntl()
const modalRef = ref<InstanceType<typeof NewModal>>()
const activeTab = ref<'checks' | 'diagnostics'>('checks')
const loading = ref(false)
const report = ref<SystemRequirementsReport | null>(null)
const selectedCustomJar = ref<string | null>(null)

const diagnostics = ref<DiagnosticsReport | null>(null)
const diagnosticsLoading = ref(false)
const copied = ref(false)

interface CheckEntry {
	key: string
	check: CheckItem
	optional: boolean
}

const checkList = computed<CheckEntry[]>(() => {
	if (!report.value) return []
	const r = report.value
	const resourcesDone = r.resources_prepared.passed

	return [
		{ key: 'vulkan', check: r.gpu_vulkan, optional: false },
		{ key: 'disk', check: r.disk_space, optional: false },
		{ key: 'runtime', check: r.runtime_installed, optional: false },
		{ key: 'resources', check: r.resources_prepared, optional: false },
		{ key: 'account', check: r.valid_account, optional: false },
		{ key: 'java', check: r.java_25, optional: resourcesDone },
		{ key: 'minecraft', check: r.minecraft_26_3, optional: resourcesDone },
	]
})

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

async function fetchDiagnostics() {
	diagnosticsLoading.value = true
	try {
		diagnostics.value = await runHorizonDiagnostics(selectedCustomJar.value)
	} catch (e) {
		console.error('Failed to fetch Horizon diagnostics:', e)
	} finally {
		diagnosticsLoading.value = false
	}
}

function switchToDiagnostics() {
	activeTab.value = 'diagnostics'
	if (!diagnostics.value) {
		void fetchDiagnostics()
	}
}

async function copyDiagnostics() {
	if (!diagnostics.value) return
	const d = diagnostics.value
	const text = [
		'=== Gradle Rust Edition Diagnostics ===',
		`Generated At: ${d.generated_at}`,
		`OS: ${d.os_info}`,
		`GPU Status: ${d.requirements.gpu_vulkan.passed ? 'PASSED' : 'FAILED'} - ${d.requirements.gpu_vulkan.details}`,
		`Disk Space: ${d.requirements.disk_space.passed ? 'PASSED' : 'FAILED'} - ${d.requirements.disk_space.details}`,
		`Runtime Installed: ${d.requirements.runtime_installed.passed ? 'PASSED' : 'FAILED'} - ${d.requirements.runtime_installed.details}`,
		`Resources Prepared: ${d.requirements.resources_prepared.passed ? 'PASSED' : 'FAILED'} - ${d.requirements.resources_prepared.details}`,
		`Account: ${d.requirements.valid_account.passed ? 'PASSED' : 'FAILED'} - ${d.requirements.valid_account.details}`,
		`Java: ${d.java_version}`,
		`Horizon Home: ${d.horizon_home}`,
		`Installed Releases: ${d.installed_horizon_versions.join(', ') || 'None'}`,
		'=== GPU Summary ===',
		d.gpu_info,
	].join('\n')

	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 2000)
}

async function exportDebug() {
	try {
		await exportDebugInfo('Export Gradle Debug Info Bundle')
	} catch (e) {
		console.error('Failed to export debug info:', e)
	}
}

async function openFolder(path: string) {
	try {
		await openPath(path)
	} catch (e) {
		console.error('Failed to open directory:', e)
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
	activeTab.value = 'checks'
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
	switchToDiagnostics,
})
</script>
