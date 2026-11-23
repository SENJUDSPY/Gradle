/**
 * ANSI Unicode / ASCII execution summary table generator for AstralRinth E2E runner.
 */

export function renderTable(results, options = {}) {
  const useColor = options.color ?? (process.stdout.isTTY && !process.env.NO_COLOR)

  const green = (str) => (useColor ? `\x1b[32m${str}\x1b[0m` : str)
  const red = (str) => (useColor ? `\x1b[31m${str}\x1b[0m` : str)
  const yellow = (str) => (useColor ? `\x1b[33m${str}\x1b[0m` : str)
  const cyan = (str) => (useColor ? `\x1b[36m${str}\x1b[0m` : str)

  const pad = (str, len) => String(str).padEnd(len).slice(0, len)
  const padNum = (num, len) => String(num).padStart(len)

  const border = '+' + '-'.repeat(14) + '+' + '-'.repeat(46) + '+' + '-'.repeat(16) + '+' + '-'.repeat(10) + '+' + '-'.repeat(10) + '+'
  const header = '| ' + pad('Test ID', 12) + ' | ' + pad('Scenario / Test Name', 44) + ' | ' + pad('Features', 14) + ' | ' + pad('Status', 8) + ' | ' + pad('Time', 8) + ' |'

  const lines = [border, header, border]
  for (const r of results) {
    const statusText = r.passed ? 'PASS' : 'FAIL'
    const statusFormatted = r.passed ? green(statusText) : red(statusText)
    const timeFormatted = padNum(r.durationMs.toFixed(1) + 'ms', 8)
    const line = '| ' + pad(r.id, 12) + ' | ' + pad(r.name, 44) + ' | ' + pad(r.features || '', 14) + ' | ' + (useColor ? (r.passed ? ' ' + green('PASS  ') : ' ' + red('FAIL  ')) : pad(statusText, 8)) + ' | ' + timeFormatted + ' |'
    lines.push(line)
  }
  lines.push(border)
  return lines.join('\n')
}

export function renderSummary(stats, options = {}) {
  const useColor = options.color ?? (process.stdout.isTTY && !process.env.NO_COLOR)
  const green = (str) => (useColor ? `\x1b[32m${str}\x1b[0m` : str)
  const red = (str) => (useColor ? `\x1b[31m${str}\x1b[0m` : str)
  const bold = (str) => (useColor ? `\x1b[1m${str}\x1b[0m` : str)

  const t1Pct = stats.tier1Total ? Math.round((stats.tier1Passed / stats.tier1Total) * 100) : 100
  const t2Pct = stats.tier2Total ? Math.round((stats.tier2Passed / stats.tier2Total) * 100) : 100
  const t3Pct = stats.tier3Total ? Math.round((stats.tier3Passed / stats.tier3Total) * 100) : 100
  const t4Pct = stats.tier4Total ? Math.round((stats.tier4Passed / stats.tier4Total) * 100) : 100

  const statusStr = stats.failed === 0 ? green('SUCCESS (Exit Code 0)') : red(`FAILURE (Exit Code 1, ${stats.failed} failed)`)

  const summary = [
    '========================================================================================',
    '                                     EXECUTION SUMMARY                                  ',
    '========================================================================================',
    `  Tier 1 (Feature Coverage):        ${String(stats.tier1Passed).padStart(2)} / ${String(stats.tier1Total).padStart(2)} passed (${t1Pct}%)`,
    `  Tier 2 (Boundary & Corner Cases): ${String(stats.tier2Passed).padStart(2)} / ${String(stats.tier2Total).padStart(2)} passed (${t2Pct}%)`,
    `  Tier 3 (Pairwise Combinations):   ${String(stats.tier3Passed).padStart(2)} / ${String(stats.tier3Total).padStart(2)} passed (${t3Pct}%)`,
    `  Tier 4 (Application Scenarios):    ${String(stats.tier4Passed).padStart(2)} / ${String(stats.tier4Total).padStart(2)} passed (${t4Pct}%)`,
    '----------------------------------------------------------------------------------------',
    `  Total Tests Executed:             ${stats.total}`,
    `  Passed:                           ${stats.passed}`,
    `  Failed:                           ${stats.failed}`,
    `  Skipped:                          ${stats.skipped || 0}`,
    `  Total Elapsed Time:               ${(stats.elapsedMs / 1000).toFixed(2)}s`,
    `  Status:                           ${statusStr}`,
    '========================================================================================'
  ]

  return summary.join('\n')
}
