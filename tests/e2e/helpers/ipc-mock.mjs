/**
 * Tauri v2 IPC mock dispatcher and contract schema validator for headless E2E verification.
 */
import assert from 'node:assert/strict'

export class IpcMock {
  constructor() {
    this.handlers = new Map()
    this.history = []
    this.state = {
      instances: new Map([
        [
          'inst-1',
          {
            pid: 48291,
            cpu_percent: 34.5,
            rss_bytes: 2147483648,
            jvm_max_memory_bytes: 4294967296,
            uptime_seconds: 120
          }
        ],
        [
          'inst-running',
          {
            pid: 18420,
            cpu_percent: 42.8,
            rss_bytes: 3650722816,
            jvm_max_memory_bytes: 4294967296,
            uptime_seconds: 862
          }
        ]
      ]),
      system: {
        cpu_usage_percent: 18.5,
        total_memory_bytes: 17179869184,
        used_memory_bytes: 8589934592,
        available_memory_bytes: 8589934592,
        disk_total_bytes: 512110190592,
        disk_available_bytes: 274877906944,
        launcher_rss_bytes: 314572800,
        launcher_cpu_percent: 2.1
      }
    }

    this._installDefaultHandlers()
  }

  _installDefaultHandlers() {
    // 1. telemetry_get_system_snapshot
    this.register('telemetry_get_system_snapshot', () => {
      return { ...this.state.system }
    })

    // 2. telemetry_get_instance_process_metrics
    this.register('telemetry_get_instance_process_metrics', (args = {}) => {
      if (!args || typeof args !== 'object' || !('instance_id' in args)) {
        throw new Error("Missing required argument 'instance_id'")
      }
      if (typeof args.instance_id !== 'string') {
        throw new Error("Argument 'instance_id' must be a string")
      }
      const inst = this.state.instances.get(args.instance_id)
      return inst ? { ...inst } : null
    })

    // 3. telemetry_get_state_inspector_dump
    this.register('telemetry_get_state_inspector_dump', () => {
      const active_processes = []
      for (const [id, m] of this.state.instances.entries()) {
        active_processes.push({
          uuid: `uuid-${id}`,
          instance_id: id,
          instance_name: `Instance ${id}`,
          pid: m.pid,
          start_time: '2026-11-22T17:00:00Z'
        })
      }
      return {
        timestamp: new Date().toISOString(),
        os_info: 'Linux 6.8.0-x86_64',
        app_version: '0.22.01',
        active_processes,
        memory_summary: { ...this.state.system },
        store_usage: {
          cache_size_bytes: 4294967296,
          instance_size_bytes: 12884901888
        }
      }
    })

    // 4. file_extract_zip / horizon_extract_mod_assets / horizon_extract_modpack_assets
    this.register('horizon_extract_mod_assets', (args = {}) => {
      const { archive_path, destination } = args
      if (archive_path && archive_path.includes('traversal')) {
        throw new Error('ERR_PATH_TRAVERSAL: Illegal relative path component in archive entry')
      }
      return { success: true, textures_recovered: 1, recipes_recovered: 1 }
    })

    this.register('horizon_extract_modpack_assets', (args = {}) => {
      const { archive_path } = args
      if (archive_path && archive_path.includes('traversal')) {
        throw new Error('ERR_PATH_TRAVERSAL: Illegal relative path component in modpack entry')
      }
      return { success: true, extracted_count: 5 }
    })

    // 5. import_modpack
    this.register('import_modpack', (args = {}) => {
      const { path } = args
      if (path && (path.includes('malicious') || path.includes('traversal'))) {
        throw new Error('ERR_PATH_TRAVERSAL: Zip-Slip path traversal detected in archive entry')
      }
      return { success: true, instance_id: 'imported-1' }
    })

    // 6. backup_server
    this.register('backup_server', (args = {}) => {
      const { server_id } = args
      if (!server_id || server_id.includes('nonexistent')) {
        throw new Error('Server directory not found')
      }
      return { backup_path: `/servers/${server_id}/backups/backup_2026.zip` }
    })
  }

  register(command, handler) {
    this.handlers.set(command, handler)
  }

  async invoke(command, args = {}) {
    this.history.push({ command, args, timestamp: Date.now() })
    const handler = this.handlers.get(command)
    if (!handler) {
      throw new Error(`IPC command '${command}' not registered in mock`)
    }
    return typeof handler === 'function' ? await handler(args) : handler
  }

  assertCalled(command, times = 1) {
    const count = this.history.filter((h) => h.command === command).length
    assert.equal(count, times, `Expected '${command}' to be called ${times} times, was called ${count} times`)
  }

  getCallCount(command) {
    return this.history.filter((h) => h.command === command).length
  }

  clearHistory() {
    this.history = []
  }
}

/** Schema validator for SystemMetricsSnapshot */
export function validateSystemMetricsSnapshot(snapshot) {
  assert.ok(snapshot && typeof snapshot === 'object', 'Snapshot must be an object')
  const required = [
    'cpu_usage_percent',
    'total_memory_bytes',
    'used_memory_bytes',
    'available_memory_bytes',
    'disk_total_bytes',
    'disk_available_bytes',
    'launcher_rss_bytes',
    'launcher_cpu_percent'
  ]
  for (const f of required) {
    assert.ok(f in snapshot, `Missing required field: ${f}`)
    assert.equal(typeof snapshot[f], 'number', `Field ${f} must be a number`)
  }
  assert.ok(snapshot.cpu_usage_percent >= 0 && snapshot.cpu_usage_percent <= 100, 'cpu_usage_percent must be in [0, 100]')
  assert.ok(snapshot.total_memory_bytes > 0, 'total_memory_bytes must be positive')
  assert.ok(snapshot.used_memory_bytes <= snapshot.total_memory_bytes, 'used_memory_bytes <= total_memory_bytes')
  assert.ok(snapshot.disk_total_bytes > 0, 'disk_total_bytes must be positive')
  assert.ok(snapshot.disk_available_bytes <= snapshot.disk_total_bytes, 'disk_available_bytes <= disk_total_bytes')
  assert.ok(snapshot.launcher_rss_bytes > 0, 'launcher_rss_bytes must be positive')
  return true
}

/** Schema validator for ProcessMetricsSnapshot */
export function validateProcessMetricsSnapshot(metrics) {
  if (metrics === null) return true
  assert.ok(metrics && typeof metrics === 'object', 'Metrics must be an object')
  const required = ['pid', 'cpu_percent', 'rss_bytes', 'uptime_seconds']
  for (const f of required) {
    assert.ok(f in metrics, `Missing required field: ${f}`)
    assert.equal(typeof metrics[f], 'number', `Field ${f} must be a number`)
  }
  if (metrics.jvm_max_memory_bytes !== null && metrics.jvm_max_memory_bytes !== undefined) {
    assert.equal(typeof metrics.jvm_max_memory_bytes, 'number', 'jvm_max_memory_bytes must be number or null')
  }
  assert.ok(metrics.pid > 0, 'PID must be positive')
  assert.ok(metrics.cpu_percent >= 0, 'cpu_percent must be non-negative')
  assert.ok(metrics.rss_bytes > 0, 'rss_bytes must be positive')
  return true
}

/** Schema validator for StateInspectorDump */
export function validateStateInspectorDump(dump) {
  assert.ok(dump && typeof dump === 'object', 'Dump must be an object')
  assert.equal(typeof dump.timestamp, 'string', 'timestamp must be a string')
  assert.match(dump.timestamp, /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/, 'timestamp must be RFC 3339')
  assert.equal(typeof dump.os_info, 'string', 'os_info must be string')
  assert.equal(typeof dump.app_version, 'string', 'app_version must be string')
  assert.ok(Array.isArray(dump.active_processes), 'active_processes must be an array')
  assert.ok(dump.memory_summary && typeof dump.memory_summary === 'object', 'memory_summary must be an object')
  validateSystemMetricsSnapshot(dump.memory_summary)
  return true
}
