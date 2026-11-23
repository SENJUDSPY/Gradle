/**
 * Vue SFC and TypeScript AST inspector helper for AstralRinth E2E verification.
 */
import assert from 'node:assert/strict'
import { readFileSync, existsSync } from 'node:fs'
import { join } from 'node:path'
import { runInNewContext } from 'node:vm'
import { parse } from '@vue/compiler-sfc'
import ts from '../../../apps/app-frontend/node_modules/typescript/lib/typescript.js'

const WORKSPACE_ROOT = join(import.meta.dirname, '../../../')

export function resolveWorkspacePath(relativePath) {
  if (relativePath.startsWith('/')) return relativePath
  return join(WORKSPACE_ROOT, relativePath)
}

/**
 * Parses a Vue Single File Component into SFC descriptor and TypeScript AST.
 */
export function inspectVueComponent(filePath) {
  const absPath = resolveWorkspacePath(filePath)
  if (!existsSync(absPath)) {
    throw new Error(`Component not found at: ${absPath}`)
  }
  const source = readFileSync(absPath, 'utf8')
  const { descriptor, errors } = parse(source)
  if (errors && errors.length > 0) {
    throw new Error(`Vue SFC parse error in ${filePath}: ${errors.map((e) => e.message).join(', ')}`)
  }

  const scriptContent = descriptor.scriptSetup?.content || descriptor.script?.content || ''
  const ast = scriptContent
    ? ts.createSourceFile(absPath, scriptContent, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
    : null

  const styles = descriptor.styles?.map((s) => s.content).join('\n') || ''
  const template = descriptor.template?.content || ''

  return {
    source,
    descriptor,
    ast,
    scriptContent,
    template,
    styles,
    hasScript: !!scriptContent,
    hasTemplate: !!template,
    hasStyles: descriptor.styles?.length > 0
  }
}

/**
 * Parses a TypeScript or JavaScript file into a TypeScript AST.
 */
export function inspectTsFile(filePath) {
  const absPath = resolveWorkspacePath(filePath)
  if (!existsSync(absPath)) {
    throw new Error(`File not found at: ${absPath}`)
  }
  const source = readFileSync(absPath, 'utf8')
  const ast = ts.createSourceFile(absPath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  return { source, ast }
}

/**
 * Searches a TS AST for a function declaration or variable statement with function initializer.
 */
export function findFunctionDeclaration(ast, name) {
  if (!ast) return null
  let found = null

  function visit(node) {
    if (found) return
    if (ts.isFunctionDeclaration(node) && node.name?.text === name) {
      found = node
      return
    }
    if (ts.isVariableStatement(node)) {
      for (const decl of node.declarationList.declarations) {
        if (decl.name?.getText(ast) === name && decl.initializer) {
          found = decl
          return
        }
      }
    }
    ts.forEachChild(node, visit)
  }

  visit(ast)
  return found
}

/**
 * Extracts and executes a component function within a secure vm context.
 */
export function extractComponentFunction(componentRelPath, functionName, context = {}) {
  const absPath = resolveWorkspacePath(componentRelPath)
  const source = readFileSync(absPath, 'utf8')
  const script = source.match(/<script\b[^>]*>([\s\S]*?)<\/script>/)?.[1]
  assert.ok(script, `${componentRelPath}: script block is required`)

  const ast = ts.createSourceFile(absPath, script, ts.ScriptTarget.Latest, true, ts.ScriptKind.TS)
  const declaration = findFunctionDeclaration(ast, functionName)
  assert.ok(declaration, `${componentRelPath}: function '${functionName}' is required`)

  const declText = declaration.getText(ast)
  const { outputText } = ts.transpileModule(declText, {
    compilerOptions: { target: ts.ScriptTarget.ES2022 }
  })

  return runInNewContext(`${outputText}\n${functionName}`, context)
}
