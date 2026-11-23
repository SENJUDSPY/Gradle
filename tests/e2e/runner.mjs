#!/usr/bin/env node
/**
 * Master E2E Test Runner for AstralRinth.
 * Executes all 4 tiers of E2E tests, measures execution duration,
 * formats ANSI tables, and enforces pass/fail exit code semantics.
 */
import { readdirSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { performance } from 'node:perf_hooks'
import { renderTable, renderSummary } from './helpers/reporter-table.mjs'

const TIERS = [
  { tier: 1, name: 'Tier 1: Feature Coverage', dir: 'tier1-feature-coverage' },
  { tier: 2, name: 'Tier 2: Boundary & Corner Cases', dir: 'tier2-boundary-corners' },
  { tier: 3, name: 'Tier 3: Pairwise Combinations', dir: 'tier3-pairwise-combinations' },
  { tier: 4, name: 'Tier 4: Application Scenarios', dir: 'tier4-application-scenarios' }
]

function printHelp() {
  console.log(`
ASTRALRINTH E2E TEST RUNNER

Usage:
  node tests/e2e/runner.mjs [options]

Options:
  --tier=<1,2,3,4>        Filter execution by tier (comma-separated, e.g. --tier=1,2)
  --grep, -g <regex>      Filter test cases or suites by name/ID regex
  --feature, -f <F1-F12>  Filter tests exercising a specific feature (e.g. -f F7)
  --reporter=<mode>       Output reporter: 'table' (default ANSI table), 'json', 'spec'
  --bail                  Stop execution immediately upon first failed test
  --list                  List all discovered test suites without executing
  --no-color              Disable ANSI colors
  --help, -h              Display CLI documentation and exit
`)
}

async function main() {
  const args = process.argv.slice(2)

  if (args.includes('--help') || args.includes('-h')) {
    printHelp()
    process.exit(0)
  }

  const noColor = args.includes('--no-color') || !process.stdout.isTTY || !!process.env.NO_COLOR
  const bail = args.includes('--bail')
  const listOnly = args.includes('--list')

  const tierArg = args.find((a) => a.startsWith('--tier='))?.split('=')[1]
  const targetTiers = tierArg
    ? tierArg.split(',').map((t) => parseInt(t.trim(), 10)).filter(Boolean)
    : null

  let grepPattern = null
  const grepIndex = args.findIndex((a) => a === '--grep' || a === '-g')
  if (grepIndex !== -1 && args[grepIndex + 1]) {
    grepPattern = new RegExp(args[grepIndex + 1], 'i')
  } else {
    const grepFlag = args.find((a) => a.startsWith('--grep='))
    if (grepFlag) {
      grepPattern = new RegExp(grepFlag.split('=')[1], 'i')
    }
  }

  let featureFilter = null
  const featIndex = args.findIndex((a) => a === '--feature' || a === '-f')
  if (featIndex !== -1 && args[featIndex + 1]) {
    featureFilter = args[featIndex + 1].toUpperCase()
  } else {
    const featFlag = args.find((a) => a.startsWith('--feature='))
    if (featFlag) {
      featureFilter = featFlag.split('=')[1].toUpperCase()
    }
  }

  const reporterArg = args.find((a) => a.startsWith('--reporter='))?.split('=')[1] || 'table'

  const activeTiers = TIERS.filter((t) => !targetTiers || targetTiers.includes(t.tier))

  if (listOnly) {
    console.log('Discovered E2E Test Suites:')
    for (const t of activeTiers) {
      const fullDir = join(import.meta.dirname, t.dir)
      if (!existsSync(fullDir)) continue
      const files = readdirSync(fullDir).filter((f) => f.endsWith('.test.mjs')).sort()
      console.log(`\n[Tier ${t.tier}] ${t.name} (${files.length} suites)`)
      for (const f of files) {
        console.log(`  - ${f}`)
      }
    }
    process.exit(0)
  }

  if (reporterArg !== 'json') {
    console.log('\n========================================================================================')
    console.log('                          ASTRALRINTH E2E TEST RUNNER HARNESS                           ')
    console.log('========================================================================================\n')
  }

  const allResults = []
  let totalFailed = 0
  const stats = {
    tier1Total: 0,
    tier1Passed: 0,
    tier2Total: 0,
    tier2Passed: 0,
    tier3Total: 0,
    tier3Passed: 0,
    tier4Total: 0,
    tier4Passed: 0,
    total: 0,
    passed: 0,
    failed: 0,
    skipped: 0,
    elapsedMs: 0
  }

  const overallStartTime = performance.now()

  for (const t of activeTiers) {
    const fullDir = join(import.meta.dirname, t.dir)
    if (!existsSync(fullDir)) continue

    const files = readdirSync(fullDir).filter((f) => f.endsWith('.test.mjs')).sort()

    if (reporterArg !== 'json') {
      console.log(` [TIER ${t.tier}] Executing ${files.length} test suites in ${t.dir}...`)
    }

    for (const file of files) {
      const suitePath = join(fullDir, file)
      let mod
      try {
        mod = await import(suitePath)
      } catch (err) {
        console.error(`Failed to load suite ${file}:`, err)
        totalFailed++
        stats.failed++
        stats[`tier${t.tier}Total`]+= 1
        stats.total++
        allResults.push({
          tier: t.tier,
          id: file.replace('.test.mjs', ''),
          name: file,
          features: `Tier ${t.tier}`,
          passed: false,
          durationMs: 0,
          error: err
        })
        if (bail) break
        continue
      }

      // If module exports tests array: execute each test individually
      if (Array.isArray(mod.tests)) {
        for (const testCase of mod.tests) {
          const testId = testCase.id || file.replace('.test.mjs', '')
          const testName = testCase.name || 'unnamed'
          const features = testCase.features || `Tier ${t.tier}`

          if (grepPattern && !grepPattern.test(testId) && !grepPattern.test(testName)) {
            stats.skipped++
            continue
          }

          if (featureFilter && !features.toUpperCase().includes(featureFilter)) {
            stats.skipped++
            continue
          }

          stats[`tier${t.tier}Total`]+= 1
          stats.total++

          const t0 = performance.now()
          let passed = true
          let error = null

          try {
            await testCase.fn()
            stats[`tier${t.tier}Passed`]+= 1
            stats.passed++
          } catch (err) {
            passed = false
            error = err
            totalFailed++
            stats.failed++
          }

          const durationMs = performance.now() - t0
          allResults.push({
            tier: t.tier,
            id: testId,
            name: testName,
            features,
            passed,
            durationMs,
            error
          })

          if (!passed && bail) break
        }
      } else if (typeof mod.runSuite === 'function') {
        const testId = file.replace('.test.mjs', '')
        const testName = file
        const features = `Tier ${t.tier}`

        if (grepPattern && !grepPattern.test(testId) && !grepPattern.test(testName)) {
          stats.skipped++
          continue
        }

        stats[`tier${t.tier}Total`]+= 1
        stats.total++

        const t0 = performance.now()
        let passed = true
        let error = null

        try {
          await mod.runSuite()
          stats[`tier${t.tier}Passed`]+= 1
          stats.passed++
        } catch (err) {
          passed = false
          error = err
          totalFailed++
          stats.failed++
        }

        const durationMs = performance.now() - t0
        allResults.push({
          tier: t.tier,
          id: testId,
          name: testName,
          features,
          passed,
          durationMs,
          error
        })

        if (!passed && bail) break
      }

      if (totalFailed > 0 && bail) break
    }

    if (totalFailed > 0 && bail) break
  }

  stats.elapsedMs = performance.now() - overallStartTime

  if (reporterArg === 'json') {
    console.log(JSON.stringify({ stats, results: allResults }, null, 2))
  } else {
    // Print table
    console.log('\n' + renderTable(allResults, { color: !noColor }))
    console.log('\n' + renderSummary(stats, { color: !noColor }) + '\n')

    if (totalFailed > 0) {
      console.log('FAILURES ENCOUNTERED:')
      for (const r of allResults.filter((r) => !r.passed)) {
        console.log(`\n[${r.id}] ${r.name}:`)
        console.log(r.error?.stack || r.error?.message || r.error)
      }
      console.log('')
    }
  }

  process.exit(totalFailed === 0 ? 0 : 1)
}

main().catch((err) => {
  console.error('Fatal runner error:', err)
  process.exit(2)
})
