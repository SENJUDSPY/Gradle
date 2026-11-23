/**
 * CSS custom property and design token auditor for AstralRinth E2E verification.
 */
import assert from 'node:assert/strict'
import { readFileSync, existsSync, readdirSync, statSync } from 'node:fs'
import { join } from 'node:path'

const WORKSPACE_ROOT = join(import.meta.dirname, '../../../')

export function resolveWorkspacePath(relativePath) {
  if (relativePath.startsWith('/')) return relativePath
  return join(WORKSPACE_ROOT, relativePath)
}

export function inspectCssFile(filePath) {
  const absPath = resolveWorkspacePath(filePath)
  if (!existsSync(absPath)) {
    throw new Error(`CSS file not found at: ${absPath}`)
  }
  const content = readFileSync(absPath, 'utf8')

  return {
    path: absPath,
    content,

    hasToken(tokenName) {
      return content.includes(tokenName)
    },

    getTokenValue(tokenName) {
      const regex = new RegExp(`${tokenName}\\s*:\\s*([^;]+);`)
      const match = content.match(regex)
      return match ? match[1].trim() : null
    },

    findSlopNeonGlows() {
      // Slop: 24px blur glows, saturated neon purple (91, 91, 240, 0.35), cyan (34, 211, 238, 0.3)
      const neonRegex = /rgba\(\s*91,\s*91,\s*240,\s*0\.35\)|rgba\(\s*34,\s*211,\s*238,\s*0\.3\)|0\s+0\s+24px\s+-2px/gi
      return [...content.matchAll(neonRegex)].map((m) => m[0])
    },

    verifyHairlineBorders() {
      // Must contain --border-hairline with 0.08 alpha
      const match = content.match(/--border-hairline\s*:\s*1px solid rgba\(\s*(?:255,\s*255,\s*255|0,\s*0,\s*0),\s*0\.08\)/)
      return !!match
    },

    verifyMonochromaticBrandGradient() {
      // Monochromatic indigo: 180deg from #6366F1 to #4F46E5, or 135deg indigo
      const indigoMatch = content.match(/--brand-gradient\s*:\s*linear-gradient\([^)]*#6366F1[^)]*#4F46E5[^)]*\)/i)
      const hasRainbow = content.includes('#22d3ee') && content.includes('#3b82f6')
      return { isMonochromatic: !hasRainbow, match: indigoMatch }
    },

    verifyAppleSpringCurves() {
      // Must define --ease-spring with cubic-bezier(0.16, 1, 0.3, 1)
      const match = content.match(/--ease-spring\s*:\s*cubic-bezier\(\s*0\.16,\s*1,\s*0\.3,\s*1\s*\)/)
      return !!match
    }
  }
}

/**
 * Recursively scans a directory for files matching extension and checks tokens.
 */
export function auditDirectoryForSlop(dirPath, ext = ['.css', '.scss', '.vue']) {
  const absPath = resolveWorkspacePath(dirPath)
  const results = []

  function scan(current) {
    if (!existsSync(current)) return
    const entries = readdirSync(current)
    for (const e of entries) {
      const full = join(current, e)
      const s = statSync(full)
      if (s.isDirectory()) {
        if (e !== 'node_modules' && e !== '.git' && e !== 'target') {
          scan(full)
        }
      } else if (ext.some((x) => e.endsWith(x))) {
        const inspector = inspectCssFile(full)
        const glows = inspector.findSlopNeonGlows()
        if (glows.length > 0) {
          results.push({ file: full, glows })
        }
      }
    }
  }

  scan(absPath)
  return results
}
