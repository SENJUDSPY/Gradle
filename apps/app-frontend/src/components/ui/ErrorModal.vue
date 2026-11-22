<script setup>
import {
	CheckIcon,
	CopyIcon,
	DropdownIcon,
	FolderOpenIcon,
	HammerIcon,
	LogInIcon,
	UpdatedIcon,
	WrenchIcon,
	XIcon,
} from '@modrinth/assets'
import {
	Button,
	ButtonLink,
	Collapsible,
	defineMessages,
	IconButton,
	injectNotificationManager,
	useVIntl,
} from '@modrinth/ui'
import { computed, ref } from 'vue'

import { ChatIcon } from '@/assets/icons'
import ConfirmModalWrapper from '@/components/ui/modal/ConfirmModalWrapper.vue'
import ModalWrapper from '@/components/ui/modal/ModalWrapper.vue'
import {
	extractDeniedCommand,
	extractErrorMessage,
	handleSevereError,
	isAclDenialMessage,
} from '@/composables/use-error.js'
import { trackEvent } from '@/helpers/analytics'
import { login as login_flow, set_default_user } from '@/helpers/auth.js'
import { install_existing_instance } from '@/helpers/install'
import { cancel_directory_change } from '@/helpers/settings.ts'
import {
	resetAppDatabase,
	restoreLatestDbBackup,
	showAppDbBackupsFolder,
} from '@/helpers/utils.js'

const { handleError } = injectNotificationManager()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	openBackupsFolder: {
		id: 'app.error.state-init.open-backups-folder',
		defaultMessage: 'Open backups folder',
	},
	restoreLatestBackup: {
		id: 'app.error.state-init.restore-backup',
		defaultMessage: 'Restore latest backup',
	},
	resetAppDatabase: {
		id: 'app.error.state-init.reset-database',
		defaultMessage: 'Reset app database (backs up first)',
	},
	restoreConfirmTitle: {
		id: 'app.error.state-init.restore-confirm-title',
		defaultMessage: 'Restore latest database backup?',
	},
	restoreConfirmDesc: {
		id: 'app.error.state-init.restore-confirm-desc',
		defaultMessage:
			'The launcher will replace the current database with the most recent backup and reload. A safety copy of your current database will be saved first.',
	},
	resetConfirmTitle: {
		id: 'app.error.state-init.reset-confirm-title',
		defaultMessage: 'Reset app database?',
	},
	resetConfirmDesc: {
		id: 'app.error.state-init.reset-confirm-desc',
		defaultMessage:
			'A complete backup of your current database will be created first. The launcher will then start with a fresh database. Your game instances, worlds, and files will NOT be deleted.',
	},
})

const errorModal = ref()
const error = ref()
const closable = ref(true)
const errorCollapsed = ref(false)

const title = ref('An error occurred')
const errorType = ref('unknown')
const supportLink = ref('https://xorison.dev/product/gradle/support')
const metadata = ref({})

const confirmRestoreModal = ref()
const confirmResetModal = ref()
const actionLoading = ref(false)
const recoveryStatus = ref('')

defineExpose({
	async show(errorVal, context, canClose = true, source = null) {
		console.log(errorVal, context, canClose, source)
		closable.value = canClose

		const rawErrMsg = extractErrorMessage(errorVal)
		const isAclDenial =
			source === 'acl_denial' ||
			Boolean(errorVal?.isAclDenial) ||
			isAclDenialMessage(rawErrMsg)
		const deniedCommand = errorVal?.deniedCommand || extractDeniedCommand(rawErrMsg)

		if (isAclDenial) {
			title.value = 'Security Rule Blocked Internal Action'
			errorType.value = 'acl_denial'
			supportLink.value = 'https://xorison.dev/product/gradle/support'
			metadata.value = { aclMessage: rawErrMsg, deniedCommand }
			errorCollapsed.value = false
		} else if (errorVal?.code?.startsWith('minecraft_auth_')) {
			title.value = 'Unable to sign in to Minecraft'
			errorType.value = 'minecraft_auth'
			supportLink.value =
				'https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues'

			if (errorVal.code === 'minecraft_auth_network') {
				metadata.value.network = true
			}
			if (errorVal.code === 'minecraft_auth_connection_refused') {
				metadata.value.hostsFile = true
			}
		} else if (errorVal?.code?.startsWith('directory_move_')) {
			title.value = 'Could not change app directory'
			errorType.value = 'directory_move'
			supportLink.value = 'https://support.modrinth.com'

			if (errorVal.code === 'directory_move_permission_denied') {
				metadata.value.readOnly = true
			}

			if (errorVal.code === 'directory_move_no_space') {
				metadata.value.notEnoughSpace = true
			}
		} else if (errorVal?.code === 'no_loader_version') {
			title.value = 'No loader selected'
			errorType.value = 'no_loader_version'
			supportLink.value = 'https://support.modrinth.com'
			metadata.value.instanceId = context?.instanceId
		} else if (errorVal?.code === 'external_auth_library_not_installed') {
			title.value = 'Authentication library required'
			errorType.value = 'external_auth_library_not_installed'
			supportLink.value = 'https://xorison.dev/product/gradle/support'
			metadata.value = {}
		} else if (source === 'state_init') {
			title.value = 'Error initializing Gradle'
			errorType.value = 'state_init'
			supportLink.value = 'https://xorison.dev/product/gradle/support'
		} else {
			title.value = 'An error occurred'
			errorType.value = 'unknown'
			supportLink.value = 'https://xorison.dev/product/gradle/support'
			metadata.value = {}
		}

		error.value = errorVal
		errorModal.value.show()
	},
})

const loadingMinecraft = ref(false)
async function loginMinecraft() {
	try {
		loadingMinecraft.value = true
		const loggedIn = await login_flow()

		if (loggedIn) {
			await set_default_user(loggedIn.profile.id).catch(handleError)
		}

		await trackEvent('AccountLogIn', { source: 'ErrorModal' })
		loadingMinecraft.value = false
		errorModal.value.hide()
	} catch (err) {
		loadingMinecraft.value = false
		handleSevereError(err)
	}
}

async function cancelDirectoryChange() {
	try {
		await cancel_directory_change()
		window.location.reload()
	} catch (err) {
		handleError(err)
	}
}

function retryDirectoryChange() {
	window.location.reload()
}

async function openDbBackupsFolder() {
	await showAppDbBackupsFolder().catch(handleError)
}

function openRestoreConfirm() {
	confirmRestoreModal.value?.show()
}

function openResetConfirm() {
	confirmResetModal.value?.show()
}

async function handleRestoreBackup() {
	try {
		actionLoading.value = true
		recoveryStatus.value = 'Restoring database backup...'
		await restoreLatestDbBackup()
		window.location.reload()
	} catch (err) {
		actionLoading.value = false
		recoveryStatus.value = `Restore failed: ${err?.message || err}`
		handleError(err)
	}
}

async function handleResetDatabase() {
	try {
		actionLoading.value = true
		recoveryStatus.value = 'Creating backup and resetting database...'
		await resetAppDatabase()
		window.location.reload()
	} catch (err) {
		actionLoading.value = false
		recoveryStatus.value = `Reset failed: ${err?.message || err}`
		handleError(err)
	}
}

const loadingRepair = ref(false)
async function repairInstance() {
	loadingRepair.value = true
	try {
		await install_existing_instance(metadata.value.instanceId, false)
		errorModal.value.hide()
	} catch (err) {
		handleSevereError(err)
	}
	loadingRepair.value = false
}

const hasDebugInfo = computed(
	() =>
		errorType.value === 'acl_denial' ||
		errorType.value === 'directory_move' ||
		errorType.value === 'minecraft_auth' ||
		errorType.value === 'state_init' ||
		errorType.value === 'no_loader_version' ||
		errorType.value === 'external_auth_library_not_installed',
)

const debugInfo = computed(() => {
	if (errorType.value === 'acl_denial') {
		const raw =
			extractErrorMessage(error.value) ||
			metadata.value?.aclMessage ||
			'Command denied by application security rules (ACL).'
		const cmd = metadata.value?.deniedCommand || extractDeniedCommand(raw)
		return cmd ? `Denied command: ${cmd}\nDetails: ${raw}` : raw
	}
	return error.value?.message ?? error.value ?? 'No error message.'
})

const copied = ref(false)

async function copyToClipboard(text) {
	await navigator.clipboard.writeText(text)
	copied.value = true
	setTimeout(() => {
		copied.value = false
	}, 3000)
}
</script>

<template>
	<ModalWrapper ref="errorModal" :header="title" :closable="closable">
		<div class="error-modal-content flex flex-col gap-4 text-contrast">
			<div class="markdown-body p-0 border-0">
				<template v-if="errorType === 'minecraft_auth'">
					<template v-if="metadata.network">
						<h3 class="text-base font-bold text-contrast mb-2">Network issues</h3>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							It looks like there were issues with the Gradle App connecting to Microsoft's
							servers. This is often the result of a poor connection, so we recommend trying again
							to see if it works. If issues continue to persist, follow the steps in
							<a
								href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_e71a5f805f"
								class="text-brand hover:underline font-medium"
							>
								our support article
							</a>
							to troubleshoot.
						</p>
					</template>
					<template v-else-if="metadata.hostsFile">
						<h3 class="text-base font-bold text-contrast mb-2">Network issues</h3>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							The Gradle App tried to connect to Microsoft / Xbox / Minecraft services, but the
							remote server rejected the connection. This may indicate that these services are
							blocked by the hosts file. Please visit
							<a
								href="https://support.modrinth.com/en/articles/9038231-minecraft-sign-in-issues#h_d694a29256"
								class="text-brand hover:underline font-medium"
							>
								our support article
							</a>
							for steps on how to fix the issue.
						</p>
					</template>
					<template v-else>
						<h3 class="text-base font-bold text-contrast mb-2">Try another Microsoft account</h3>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							Double check you've signed in with the right account. You may own Minecraft on a
							different Microsoft account.
						</p>
						<div class="cta-button">
							<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
								<LogInIcon /> Try another account
							</button>
						</div>
						<h3 class="text-base font-bold text-contrast mb-2">
							Using PC Game Pass, coming from Bedrock, or just bought the game?
						</h3>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							Try signing in with the
							<a
								href="https://www.minecraft.net/en-us/download"
								class="text-brand hover:underline font-medium"
							>
								official Minecraft Launcher
							</a>
							first. Once you're done, come back here and sign in!
						</p>
					</template>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingMinecraft" @click="loginMinecraft">
							<LogInIcon /> Try signing in again
						</button>
					</div>
				</template>
				<template v-else-if="errorType === 'directory_move'">
					<template v-if="metadata.readOnly">
						<h3 class="text-base font-bold text-contrast mb-2">Change directory permissions</h3>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							It looks like the Gradle App is unable to write to the directory you selected.
							Please adjust the permissions of the directory and try again or cancel the directory
							change.
						</p>
					</template>
					<template v-else-if="metadata.notEnoughSpace">
						<h3 class="text-base font-bold text-contrast mb-2">Not enough space</h3>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							It looks like there is not enough space on the disk containing the directory you
							selected. Please free up some space and try again or cancel the directory change.
						</p>
					</template>
					<template v-else>
						<p class="text-secondary text-sm leading-relaxed mb-3">
							The Gradle App is unable to migrate to the new directory you selected. Please
							contact support for help or cancel the directory change.
						</p>
					</template>

					<div class="cta-button">
						<button class="btn" @click="retryDirectoryChange">
							<UpdatedIcon /> Retry directory change
						</button>
						<button class="btn btn-danger" @click="cancelDirectoryChange">
							<XIcon /> Cancel directory change
						</button>
					</div>
				</template>
				<template v-else-if="errorType === 'acl_denial'">
					<p class="text-sm leading-relaxed text-secondary mb-3">
						A security rule blocked an internal action.
					</p>
					<p class="text-sm leading-relaxed text-secondary mb-3">
						An internal permission was denied by the application security policy. Details of the blocked command are shown below.
					</p>
				</template>
				<template v-else-if="errorType === 'state_init'">
					<p class="text-sm leading-relaxed text-secondary mb-3">
						Gradle failed to initialize its local database or load crucial startup state.
					</p>
					<p class="text-sm leading-relaxed text-secondary mb-3">
						You can restore a pre-migration backup, or reset the app database to start fresh with a safe backup copy:
					</p>
					<div class="recovery-actions flex flex-col gap-2 p-3 bg-surface-2 rounded-xl border-0 mb-3">
						<span class="text-xs font-bold text-contrast uppercase tracking-wider mb-1">Database Recovery Options</span>
						<div class="flex flex-wrap items-center gap-2">
							<button class="btn btn-primary" :disabled="actionLoading" @click="openRestoreConfirm">
								<UpdatedIcon aria-hidden="true" />
								{{ formatMessage(messages.restoreLatestBackup) }}
							</button>
							<button class="btn btn-danger" :disabled="actionLoading" @click="openResetConfirm">
								<HammerIcon aria-hidden="true" />
								{{ formatMessage(messages.resetAppDatabase) }}
							</button>
							<button class="btn" @click="openDbBackupsFolder">
								<FolderOpenIcon aria-hidden="true" />
								{{ formatMessage(messages.openBackupsFolder) }}
							</button>
						</div>
						<span v-if="recoveryStatus" class="text-xs text-brand font-medium mt-1">{{ recoveryStatus }}</span>
					</div>
				</template>
				<template v-else-if="errorType === 'no_loader_version'">
					<p class="text-secondary text-sm leading-relaxed mb-3">The Gradle App failed to find the loader version for this instance.</p>
					<p class="text-secondary text-sm leading-relaxed mb-3">To resolve this, you need to repair the instance. Click the button below to do so.</p>
					<div class="cta-button">
						<button class="btn btn-primary" :disabled="loadingRepair" @click="repairInstance">
							<HammerIcon /> Repair instance
						</button>
					</div>
				</template>
				<template v-else-if="errorType === 'external_auth_library_not_installed'">
					<p class="text-secondary text-sm leading-relaxed mb-2">{{ debugInfo }}</p>
					<p class="text-secondary text-sm leading-relaxed mb-3">
						Choose and install a version under Settings → Gradle → Authentication libraries,
						then launch the instance again.
					</p>
				</template>
				<template v-else>
					<p class="text-secondary text-sm leading-relaxed mb-3">{{ debugInfo }}</p>
				</template>
				<template v-if="hasDebugInfo">
					<p class="text-secondary text-sm leading-relaxed mb-3">
						If you need help, please visit
						<a :href="supportLink" target="_blank" rel="noopener noreferrer" class="text-brand hover:underline font-medium">our support page</a>
						and we will be glad to assist! Make sure to provide the debug information below:
					</p>
				</template>
			</div>
			<div class="flex items-center gap-2 pt-1">
				<ButtonLink :href="supportLink" @click="errorModal.hide()"
					><ChatIcon /> Get support</ButtonLink
				>
				<Button v-if="closable" @click="errorModal.hide()"><XIcon /> Close</Button>
			</div>
			<template v-if="hasDebugInfo">
				<div class="flex flex-col gap-2 mt-2">
					<div class="overflow-clip">
						<button
							class="flex items-center justify-between w-full bg-transparent border-0 py-2 cursor-pointer"
							@click="errorCollapsed = !errorCollapsed"
						>
							<span class="flex items-center gap-2 text-contrast font-bold text-sm m-0">
								<WrenchIcon class="h-4 w-4" />
								Debug information
							</span>
							<DropdownIcon
								class="h-5 w-5 text-secondary transition-transform"
								:class="{ 'rotate-180': !errorCollapsed }"
							/>
						</button>
						<Collapsible :collapsed="errorCollapsed">
							<div
								class="p-4 bg-surface-2 rounded-xl text-xs flex flex-col gap-2 max-w-full items-start border-0"
							>
								<div class="flex items-start justify-between gap-3 w-full mb-2">
									<pre
										class="m-0 p-0 rounded-none bg-transparent text-xs font-mono text-secondary whitespace-pre-wrap break-all select-text overflow-auto max-h-48 flex-1 leading-relaxed"
									>{{ debugInfo }}</pre>
									<IconButton
										v-tooltip="'Copy debug info'"
										:label="'Copy debug info'"
										:disabled="copied"
										class="shrink-0"
										@click="copyToClipboard(debugInfo)"
									>
										<template v-if="copied"> <CheckIcon class="text-green" /> </template>
										<template v-else> <CopyIcon /> </template>
									</IconButton>
								</div>
							</div>
						</Collapsible>
					</div>
				</div>
			</template>
		</div>

		<ConfirmModalWrapper
			ref="confirmRestoreModal"
			:title="formatMessage(messages.restoreConfirmTitle)"
			:description="formatMessage(messages.restoreConfirmDesc)"
			proceed-label="Restore Backup"
			:danger="false"
			@proceed="handleRestoreBackup"
		/>
		<ConfirmModalWrapper
			ref="confirmResetModal"
			:title="formatMessage(messages.resetConfirmTitle)"
			:description="formatMessage(messages.resetConfirmDesc)"
			proceed-label="Reset Database"
			:danger="true"
			@proceed="handleResetDatabase"
		/>
	</ModalWrapper>
</template>

<style scoped lang="scss">
.error-modal-content {
	padding: 1.5rem;
	max-width: 580px;
	background-color: var(--surface-3);
}

.markdown-body {
	border: none !important;

	p {
		border: none !important;
		padding: 0 !important;
	}
}

.cta-button {
	display: flex;
	align-items: center;
	justify-content: center;
	padding: 0.5rem;
	gap: 0.5rem;
}

.recovery-actions {
	background-color: var(--surface-2);
}
</style>
