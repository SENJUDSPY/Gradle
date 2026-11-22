import 'overlayscrollbars/overlayscrollbars.css'

import { installTooltipDirective } from '@modrinth/ui'
import { VueQueryPlugin } from '@tanstack/vue-query'
import { createApp } from 'vue'

import App from '@/App.vue'
import { overlayScrollbarsDirective } from '@/directives/overlayScrollbars'
import { setupErrorReporting } from '@/helpers/error-reporting'
import { debugStartup, traceStartupStep } from '@/helpers/startup-debug'
import i18nPlugin from '@/plugins/i18n'
import i18nDebugPlugin from '@/plugins/i18n-debug'
import router from '@/routes'

// Intercept Tauri invoke calls to capture exact command on ACL denials
if (typeof window !== 'undefined') {
	const installTauriInterceptor = () => {
		const internals = window.__TAURI_INTERNALS__
		if (internals && typeof internals.invoke === 'function' && !internals.__acl_intercepted) {
			const originalInvoke = internals.invoke
			internals.invoke = async function (cmd, args, options) {
				try {
					return await originalInvoke.call(this, cmd, args, options)
				} catch (err) {
					const rawMsg = err?.message || String(err)
					if (
						rawMsg.includes('restricted by application security rules') ||
						rawMsg.includes('not allowed by ACL') ||
						rawMsg.includes('denied by ACL')
					) {
						const aclError = new Error(`Command '${cmd}' not allowed by ACL: ${rawMsg}`)
						aclError.deniedCommand = cmd
						aclError.isAclDenial = true
						aclError.rawAclMessage = rawMsg
						throw aclError
					}
					throw err
				}
			}
			internals.__acl_intercepted = true
		}
	}
	installTauriInterceptor()
	window.addEventListener('DOMContentLoaded', installTauriInterceptor)
}

debugStartup('Frontend entry module evaluated')
const app = createApp(App)
setupErrorReporting(app, router)

app.use(VueQueryPlugin)
app.use(router)
if (import.meta.env.DEV) {
	void traceStartupStep('Initial router readiness', () => router.isReady()).catch(() => {})
}
app.use(i18nPlugin)
app.use(i18nDebugPlugin)
installTooltipDirective(app)
app.directive('overlay-scrollbars', overlayScrollbarsDirective)

async function mount() {
	if (import.meta.env.DEV && import.meta.env.VITE_VUE_SCAN === 'true') {
		const { VueScanPlugin } = await traceStartupStep(
			'Load Vue render tracker',
			() => import('@taijased/vue-render-tracker'),
		)
		app.use(new VueScanPlugin({ enabled: true, showOverlay: true, log: false, playSound: false }))
	}
	debugStartup('Vue mount started')
	app.mount('#app')
	debugStartup('Vue mount completed')
}

void mount()
