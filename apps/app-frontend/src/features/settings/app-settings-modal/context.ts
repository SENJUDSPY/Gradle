import { createContext } from '@modrinth/ui'

export type UnsavedChangesController = {
	hasChanges: () => boolean
	getOriginal: () => object
	getModified: () => object
	isSaving: () => boolean
	reset: () => void
	save: () => void | Promise<void>
}

export interface AppSettingsModalContext {
	close: () => boolean
	registerUnsavedChangesController: (controller: UnsavedChangesController | null) => void
}

export const [injectAppSettingsModal, provideAppSettingsModal] =
	createContext<AppSettingsModalContext>('AppSettingsModal', 'appSettingsModal')
