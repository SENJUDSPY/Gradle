import { reactive } from 'vue'

export function extractErrorMessage(error) {
	if (error instanceof Error) {
		return error.message || String(error)
	}
	if (typeof error === 'string') {
		return error
	}
	if (error && typeof error === 'object') {
		const raw =
			error.rawAclMessage ||
			error.message ||
			error.error ||
			error.description ||
			error.details
		if (typeof raw === 'string' && raw.length > 0) {
			return raw
		}
		try {
			return JSON.stringify(error)
		} catch {
			return String(error)
		}
	}
	return error != null ? String(error) : ''
}

export function isAclDenialMessage(msg) {
	if (!msg || typeof msg !== 'string') return false
	return (
		msg.includes('not allowed by ACL') ||
		msg.includes('not permitted by ACL') ||
		msg.includes('denied by ACL') ||
		msg.includes('blocked by ACL') ||
		msg.includes('Permissions associated with this command:') ||
		msg.includes('explicitly denied on origin') ||
		msg.includes('not allowed on any window') ||
		msg.includes('restricted by application security rules') ||
		msg.includes('application security rules') ||
		/\b[a-z0-9_-]+(?:\|[a-z0-9_]+|\.[a-z0-9_]+)\s+not allowed(?:\.| on\b)/i.test(msg)
	)
}

export function extractDeniedCommand(msg) {
	if (!msg || typeof msg !== 'string') return null
	const quotedMatch = msg.match(/Command\s+['"]([^'"]+)['"]\s+not allowed/i)
	if (quotedMatch) {
		return quotedMatch[1]
	}
	// Release format: "Command plugin:horizon|horizon_list_servers not allowed by ACL"
	// or "Command initialize_state not allowed by ACL"
	const releasePluginMatch = msg.match(/\bplugin:([a-z0-9_-]+\|[a-z0-9_]+)/i)
	if (releasePluginMatch) {
		return releasePluginMatch[1]
	}
	const releaseAppMatch = msg.match(/\bCommand\s+([a-z0-9_]+)\s+not allowed/i)
	if (releaseAppMatch) {
		return releaseAppMatch[1]
	}
	// Debug format: "window.is_maximized not allowed on window ..." or "horizon.horizon_list_servers not allowed."
	const debugPluginMatch = msg.match(/\b([a-z0-9_-]+)\.([a-z0-9_]+)\s+(?:not allowed|explicitly denied)/i)
	if (debugPluginMatch) {
		return `${debugPluginMatch[1]}|${debugPluginMatch[2]}`
	}
	const debugAppMatch = msg.match(/^([a-z0-9_]+)\s+(?:not allowed|explicitly denied)/i)
	if (debugAppMatch) {
		return debugAppMatch[1]
	}
	return null
}

const errorState = reactive({
	errorModal: null,
	minecraftAuthErrorModal: null,
	minecraftRequiredModal: null,
	pendingError: null,
	setErrorModal(ref) {
		this.errorModal = ref
		if (ref && this.pendingError) {
			const pending = this.pendingError
			this.pendingError = null
			ref.show(pending.error, pending.context, pending.closable, pending.source)
		}
	},
	setMinecraftAuthErrorModal(ref) {
		this.minecraftAuthErrorModal = ref
	},
	setMinecraftRequiredModal(ref) {
		this.minecraftRequiredModal = ref
	},
	showError(error, context, closable = true, source = null) {
		console.error('[useError.showError]', error, context)
		if (error && typeof error === 'object') {
			if (error.code === 'not_logged_in' && this.minecraftRequiredModal) {
				this.minecraftRequiredModal.show()
				return
			}
			if (error.code?.startsWith('minecraft_auth_') && this.minecraftAuthErrorModal) {
				this.minecraftAuthErrorModal.show(error)
				return
			}
		}
		const errMsg = extractErrorMessage(error)
		const isAclDenial = isAclDenialMessage(errMsg)
		const deniedCommand = isAclDenial ? extractDeniedCommand(errMsg) : null
		let normalizedError = error
		if (isAclDenial) {
			console.error('[useError] ACL denial detected:', { deniedCommand, errMsg })
			if (normalizedError && typeof normalizedError === 'object') {
				normalizedError.isAclDenial = true
				normalizedError.rawAclMessage = normalizedError.rawAclMessage || errMsg
				normalizedError.deniedCommand = normalizedError.deniedCommand || deniedCommand
			} else {
				normalizedError = {
					message: errMsg,
					isAclDenial: true,
					rawAclMessage: errMsg,
					deniedCommand,
				}
			}
		}
		const resolvedSource = isAclDenial ? 'acl_denial' : source
		if (this.errorModal) {
			this.errorModal.show(normalizedError, context, closable, resolvedSource)
		} else {
			this.pendingError = {
				error: normalizedError,
				context,
				closable,
				source: resolvedSource,
			}
		}
	},
})

export function useError() {
	return errorState
}

export const handleSevereError = (error, context) => {
	console.error('[handleSevereError]', error, context)
	useError().showError(error, context)
}
