/**
 * Cargo subprocess runner and Rust source safety/lint auditor for AstralRinth E2E verification.
 */
import assert from 'node:assert/strict'
import { execSync } from 'node:child_process'
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const WORKSPACE_ROOT = join(import.meta.dirname, '../../../')

export function resolveWorkspacePath(relativePath) {
  if (relativePath.startsWith('/')) return relativePath
  return join(WORKSPACE_ROOT, relativePath)
}

const executionCache = new Map()

export function runCargoTest(packageName, options = {}) {
  const cacheKey = `test-${packageName}`
  if (!options.fresh && executionCache.has(cacheKey)) {
    return executionCache.get(cacheKey)
  }

  const timeoutMs = options.timeoutMs || 25000
  try {
    const stdout = execSync(`cargo test -p ${packageName} -- --nocapture`, {
      cwd: WORKSPACE_ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: timeoutMs
    })
    const res = { success: true, stdout, stderr: '', exitCode: 0 }
    executionCache.set(cacheKey, res)
    return res
  } catch (err) {
    const res = {
      success: false,
      stdout: err.stdout ? String(err.stdout) : '',
      stderr: err.stderr ? String(err.stderr) : '',
      exitCode: err.status || 1
    }
    executionCache.set(cacheKey, res)
    return res
  }
}

export function runCargoCheck(packageName, options = {}) {
  const cacheKey = `check-${packageName}-${options.allFeatures ? 'all' : 'default'}`
  if (!options.fresh && executionCache.has(cacheKey)) {
    return executionCache.get(cacheKey)
  }

  const timeoutMs = options.timeoutMs || 25000
  const flag = options.allFeatures ? '--all-features' : options.noDefaultFeatures ? '--no-default-features' : ''
  const cmd = packageName
    ? `cargo check -p ${packageName} ${flag} --message-format=json`
    : `cargo check --workspace ${flag} --message-format=json`

  try {
    const stdout = execSync(cmd, {
      cwd: WORKSPACE_ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: timeoutMs
    })

    const messages = stdout
      .split('\n')
      .filter((line) => line.trim().startsWith('{'))
      .map((line) => {
        try {
          return JSON.parse(line)
        } catch {
          return null
        }
      })
      .filter((m) => m && m.reason === 'compiler-message')

    const warnings = messages.filter((m) => m.message?.level === 'warning')
    const errors = messages.filter((m) => m.message?.level === 'error')

    const res = {
      success: errors.length === 0,
      exitCode: 0,
      warningsCount: warnings.length,
      errorsCount: errors.length,
      warnings,
      errors,
      stdout
    }
    executionCache.set(cacheKey, res)
    return res
  } catch (err) {
    const res = {
      success: false,
      exitCode: err.status || 1,
      warningsCount: 0,
      errorsCount: 1,
      warnings: [],
      errors: [err.message],
      stdout: err.stdout ? String(err.stdout) : '',
      stderr: err.stderr ? String(err.stderr) : ''
    }
    executionCache.set(cacheKey, res)
    return res
  }
}

export function runCargoClippy(packageName, options = {}) {
  const cacheKey = `clippy-${packageName || 'workspace'}`
  if (!options.fresh && executionCache.has(cacheKey)) {
    return executionCache.get(cacheKey)
  }

  const timeoutMs = options.timeoutMs || 30000
  const target = packageName ? `-p ${packageName}` : '--workspace'
  const cmd = `cargo clippy ${target} --all-targets --message-format=json -- -D warnings`

  try {
    const stdout = execSync(cmd, {
      cwd: WORKSPACE_ROOT,
      encoding: 'utf8',
      stdio: 'pipe',
      timeout: timeoutMs
    })

    const messages = stdout
      .split('\n')
      .filter((line) => line.trim().startsWith('{'))
      .map((line) => {
        try {
          return JSON.parse(line)
        } catch {
          return null
        }
      })
      .filter((m) => m && m.reason === 'compiler-message')

    const warnings = messages.filter((m) => m.message?.level === 'warning')
    const errors = messages.filter((m) => m.message?.level === 'error')

    const res = {
      clean: warnings.length === 0 && errors.length === 0,
      warningsCount: warnings.length,
      errorsCount: errors.length,
      warnings,
      errors,
      stdout
    }
    executionCache.set(cacheKey, res)
    return res
  } catch (err) {
    const res = {
      clean: false,
      warningsCount: 1,
      errorsCount: 0,
      warnings: [err.message],
      errors: [],
      stdout: err.stdout ? String(err.stdout) : '',
      stderr: err.stderr ? String(err.stderr) : ''
    }
    executionCache.set(cacheKey, res)
    return res
  }
}

/**
 * Scans Rust source files for blanket `#![allow(warnings)]` or `#![allow(clippy::all)]`.
 */
export function scanBlanketAllowWarnings(dirPath = 'packages') {
  const absPath = resolveWorkspacePath(dirPath)
  const violations = []

  function scan(current) {
    if (!existsSync(current)) return
    for (const e of readdirSync(current)) {
      const full = join(current, e)
      const s = statSync(full)
      if (s.isDirectory()) {
        if (e !== 'target' && e !== '.git') {
          scan(full)
        }
      } else if (e.endsWith('.rs')) {
        const content = readFileSync(full, 'utf8')
        const regex = /#!\[\s*allow\s*\(\s*(warnings|clippy::all)\s*\)\s*\]/g
        const match = content.match(regex)
        if (match) {
          violations.push({ file: full, matches: match })
        }
      }
    }
  }

  scan(absPath)
  return violations
}

/**
 * Scans Rust source files for unsafe blocks and asserts they have preceding `// SAFETY:` comments.
 */
export function scanSourceSafetyComments(dirPath = 'packages/horizon-runtime') {
  const absPath = resolveWorkspacePath(dirPath)
  const undocumentedUnsafe = []

  function scan(current) {
    if (!existsSync(current)) return
    for (const e of readdirSync(current)) {
      const full = join(current, e)
      const s = statSync(full)
      if (s.isDirectory()) {
        if (e !== 'target' && e !== '.git') {
          scan(full)
        }
      } else if (e.endsWith('.rs')) {
        const lines = readFileSync(full, 'utf8').split('\n')
        for (let i = 0; i < lines.length; i++) {
          const line = lines[i].trim()
          if (line.startsWith('unsafe ') || line === 'unsafe {') {
            // Check preceding 3 lines for // SAFETY:
            let hasSafety = false
            for (let j = Math.max(0, i - 3); j < i; j++) {
              if (lines[j].includes('SAFETY:')) {
                hasSafety = true
                break
              }
            }
            if (!hasSafety) {
              undocumentedUnsafe.push({ file: full, line: i + 1, code: line })
            }
          }
        }
      }
    }
  }

  scan(absPath)
  return undocumentedUnsafe
}
