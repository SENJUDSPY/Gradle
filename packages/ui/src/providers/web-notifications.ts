import { ModrinthApiError } from '@modrinth/api-client'
import { isCancelledError } from '@tanstack/vue-query'
import type { Component } from 'vue'

import { createContext } from '.'

export interface WebNotificationButton {
	label: string
	action: () => void | Promise<void>
	icon?: Component
	color?: 'brand' | 'red' | 'orange' | 'green' | 'blue' | 'standard'
	keepOpen?: boolean
}

export interface WebNotification {
	id: string | number
	title?: string
	text?: string
	type?: 'error' | 'warning' | 'success' | 'info' | 'neutral'
	errorCode?: string
	count?: number
	autoCloseMs?: number | null // null means do not dismiss automatically
	timer?: NodeJS.Timeout
	supportData?: Record<string, unknown>
	containerClass?: string
	noIcon?: boolean
	buttons?: WebNotificationButton[]
	dismissible?: boolean
	copyable?: boolean
}

export type NotificationPanelLocation = 'left' | 'right'

export abstract class AbstractWebNotificationManager {
	protected readonly DEFAULT_AUTO_DISMISS_DELAY_MS = 7 * 1000
	protected readonly ERROR_AUTO_DISMISS_DELAY_MS = 10 * 1000

	abstract getNotifications(): WebNotification[]
	abstract getNotificationLocation(): NotificationPanelLocation
	abstract setNotificationLocation(location: NotificationPanelLocation): void

	protected abstract addNotificationToStorage(notification: WebNotification): void
	protected abstract removeNotificationFromStorage(id: string | number): void
	protected abstract removeNotificationFromStorageByIndex(index: number): void
	protected abstract clearAllNotificationsFromStorage(): void

	addNotification = (notification: Partial<WebNotification>): WebNotification => {
		const existingNotif = this.findExistingNotification(notification)

		if (existingNotif) {
			this.refreshNotificationTimer(existingNotif)
			existingNotif.count = (existingNotif.count || 0) + 1
			return existingNotif
		}

		const newNotification = this.createNotification(notification)
		this.setNotificationTimer(newNotification)
		this.addNotificationToStorage(newNotification)
		return newNotification
	}

	/**
	 * Query cancellations (e.g. navigating away mid-fetch) are ignored, as they are not user-facing failures.
	 *
	 * @deprecated You should use `addNotification` instead to provide a more human-readable error message to the user.
	 */
	handleError = (error: unknown): void => {
		if (isCancelledError(error)) return

		console.error('[NotificationManager] Handled error:', error)

		let message: string
		if (error instanceof Error) {
			message = error.message
		} else if (typeof error === 'string') {
			message = error
		} else if (typeof error === 'object' && error !== null) {
			const obj = error as Record<string, unknown>
			message =
				(typeof obj.message === 'string' && obj.message) ||
				(typeof obj.error === 'string' && obj.error) ||
				(typeof obj.description === 'string' && obj.description) ||
				(typeof obj.details === 'string' && obj.details) ||
				(() => {
					try {
						return JSON.stringify(error)
					} catch {
						return String(error)
					}
				})()
		} else {
			message = String(error)
		}

		if (
			message.includes('not allowed by ACL') ||
			message.includes('not permitted by ACL') ||
			message.includes('denied by ACL') ||
			message.includes('blocked by ACL') ||
			message.includes('Permissions associated with this command:') ||
			/\b[a-z0-9_-]+(?:\|[a-z0-9_]+|\.[a-z0-9_]+)\s+not allowed(?:\.| on\b)/i.test(message)
		) {
			message = `A security rule blocked an internal action: ${message}`
		}

		const errorCode =
			typeof error === 'object' && error !== null && 'code' in error
				? String((error as { code: unknown }).code)
				: undefined

		this.addNotification({
			title: 'An error occurred',
			text: message,
			errorCode,
			type: 'error',
		})
	}

	removeNotification = (id: string | number): WebNotification | undefined => {
		const notifications = this.getNotifications()
		const notification = notifications.find((n) => n.id === id)

		if (notification) {
			this.clearNotificationTimer(notification)
			this.removeNotificationFromStorage(id)
		}

		return notification
	}

	removeNotificationByIndex = (index: number): WebNotification | null => {
		const notifications = this.getNotifications()

		if (index >= 0 && index < notifications.length) {
			const notification = notifications[index]
			this.clearNotificationTimer(notification)
			this.removeNotificationFromStorageByIndex(index)

			return notification
		}

		return null
	}

	clearAllNotifications = (): void => {
		const notifications = this.getNotifications()
		notifications.forEach((notification) => {
			this.clearNotificationTimer(notification)
		})
		this.clearAllNotificationsFromStorage()
	}

	setNotificationTimer = (notification: WebNotification): void => {
		if (!notification) return

		this.clearNotificationTimer(notification)

		if (notification.autoCloseMs === null) return

		const isError = notification.type === 'error'
		const delay =
			notification.autoCloseMs ??
			(isError ? this.ERROR_AUTO_DISMISS_DELAY_MS : this.DEFAULT_AUTO_DISMISS_DELAY_MS)

		notification.timer = setTimeout(() => {
			this.removeNotification(notification.id)
		}, delay)
	}

	stopNotificationTimer = (notification: WebNotification): void => {
		this.clearNotificationTimer(notification)
	}

	private refreshNotificationTimer(notification: WebNotification): void {
		this.setNotificationTimer(notification)
	}

	private clearNotificationTimer(notification: WebNotification): void {
		if (notification.timer) {
			clearTimeout(notification.timer)
			notification.timer = undefined
		}
	}

	private findExistingNotification(
		notification: Partial<WebNotification>,
	): WebNotification | undefined {
		return this.getNotifications().find(
			(existing) =>
				existing.text === notification.text &&
				existing.title === notification.title &&
				existing.type === notification.type,
		)
	}

	private createNotification(notification: Partial<WebNotification>): WebNotification {
		return {
			...notification,
			id: notification.id ?? new Date().getTime(),
			count: 1,
		} as WebNotification
	}
}

export const [injectNotificationManager, provideNotificationManager] =
	createContext<AbstractWebNotificationManager>('root', 'notificationManager')
