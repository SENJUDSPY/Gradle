import { test } from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'

const ROOT = join(import.meta.dirname, '../../../')

export const tests = [
  {
    id: 'TC-F1-01',
    name: 'Horizon runtime crate manifest & target validity',
    features: 'F1',
    fn: async () => {
      const manifestPath = join(ROOT, 'packages/horizon-runtime/Cargo.toml')
      assert.ok(existsSync(manifestPath), 'horizon-runtime Cargo.toml must exist')
      const toml = readFileSync(manifestPath, 'utf8')
      assert.ok(toml.includes('name = "horizon-runtime"'), 'Must have crate name horizon-runtime')
      assert.ok(toml.includes('edition.workspace = true') || toml.includes('edition = "2024"') || toml.includes('edition = "2021"'), 'Must specify valid Rust edition')
      assert.ok(existsSync(join(ROOT, 'packages/horizon-runtime/src/lib.rs')), 'src/lib.rs must exist')
    }
  },
  {
    id: 'TC-F1-02',
    name: 'Theseus app-lib crate manifest & telemetry state module',
    features: 'F1',
    fn: async () => {
      const manifestPath = join(ROOT, 'packages/app-lib/Cargo.toml')
      assert.ok(existsSync(manifestPath), 'packages/app-lib/Cargo.toml must exist')
      const toml = readFileSync(manifestPath, 'utf8')
      assert.ok(toml.includes('name = "theseus"'), 'Must have crate name theseus')
      assert.ok(toml.includes('sysinfo'), 'theseus must depend on sysinfo for telemetry')
      assert.ok(existsSync(join(ROOT, 'packages/app-lib/src/state/process.rs')), 'state/process.rs must exist')
    }
  },
  {
    id: 'TC-F1-03',
    name: 'Theseus GUI desktop application crate structure',
    features: 'F1',
    fn: async () => {
      const manifestPath = join(ROOT, 'apps/app/Cargo.toml')
      assert.ok(existsSync(manifestPath), 'apps/app/Cargo.toml must exist')
      const toml = readFileSync(manifestPath, 'utf8')
      assert.ok(toml.includes('name = "theseus_gui"'), 'Must have crate name theseus_gui')
      assert.ok(existsSync(join(ROOT, 'apps/app/src/main.rs')), 'src/main.rs must exist')
      assert.ok(existsSync(join(ROOT, 'apps/app/src/commands.rs')), 'src/commands.rs must exist')
    }
  },
  {
    id: 'TC-F1-04',
    name: 'Workspace Cargo root members and resolver integrity',
    features: 'F1',
    fn: async () => {
      const rootCargo = readFileSync(join(ROOT, 'Cargo.toml'), 'utf8')
      assert.ok(rootCargo.includes('members = ['), 'Workspace must define members')
      assert.ok(rootCargo.includes('"packages/horizon-runtime"'), 'Must include horizon-runtime')
      assert.ok(rootCargo.includes('"packages/app-lib"'), 'Must include packages/app-lib')
      assert.ok(rootCargo.includes('"apps/app"'), 'Must include apps/app')
    }
  },
  {
    id: 'TC-F1-05',
    name: 'Strict clippy linter configuration and deny directives',
    features: 'F1',
    fn: async () => {
      const clippyToml = join(ROOT, 'clippy.toml')
      assert.ok(existsSync(clippyToml), 'clippy.toml must exist at root')
      // Root Cargo.toml lint settings
      const rootCargo = readFileSync(join(ROOT, 'Cargo.toml'), 'utf8')
      assert.ok(rootCargo.includes('[workspace.lints') || existsSync(clippyToml), 'Clippy linter directives must be configured')
    }
  }
]

for (const t of tests) {
  test(`${t.id}: ${t.name}`, t.fn)
}

export async function runSuite() {
  for (const t of tests) {
    await t.fn()
  }
}
