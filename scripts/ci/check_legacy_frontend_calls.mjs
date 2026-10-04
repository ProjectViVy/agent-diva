#!/usr/bin/env node
// DN-M gate: no reachable legacy business path may exist in the migrated
// frontend. Allowlist-based — anything not explicitly permitted fails.
//
// Rules over agent-diva-gui/src/**/*.{ts,vue} (tests/fixtures excluded):
//   R1 '@tauri-apps/api/core' imports → only src/platform/desktop-host.ts
//   R2 any other '@tauri-apps/*' import → desktop-host.ts | utils/openExternal.ts
//   R3 invoke()/invokeCommand()/.invoke() call → only inside the allowed files;
//      a non-literal first argument is a dynamic business invoke → always denied
//   R4 new EventSource | WebSocket | XMLHttpRequest → denied (Manager HTTP/SSE)
//   R5 fetch( → denied outside the dormant pet feature
//   R6 provider endpoint URL literal → denied outside welcomeConfig.ts
//      (the wizard default endpoint is data written to backend settings,
//      not a call site)
//
// Dormant scope: src/features/diva-pet/** is a sealed native feature ruled
// dormant in DN-3/DN-6 — it is exempt wholesale; its own DN-6 dispositions
// cover its future.
//
// --selftest runs the negative fixtures in scripts/ci/fixtures/legacy-calls/
// through the same scanner and requires every rule to fire.
import { readFileSync, readdirSync, statSync } from 'node:fs'
import { join, relative } from 'node:path'
import { createRequire } from 'node:module'

const REPO_ROOT = process.argv.includes('--repo')
  ? process.argv[process.argv.indexOf('--repo') + 1]
  : new URL('../../', import.meta.url).pathname
const SRC = join(REPO_ROOT, 'agent-diva-gui', 'src')
const require = createRequire(join(REPO_ROOT, 'agent-diva-gui', 'package.json'))
const ts = require('typescript')

const INVOKE_FILE = 'src/platform/desktop-host.ts'
const NATIVE_FILES = new Set([INVOKE_FILE, 'src/utils/openExternal.ts'])
const PROVIDER_URL_FILES = new Set(['src/utils/welcomeConfig.ts'])
const DORMANT_PREFIX = 'src/features/diva-pet/'
const VIVY_CMD = 'vivy_call'
// DN-6B native speech commands: fixed literal names, allowed only inside
// the frozen desktop-host seam.
const NATIVE_CMDS = new Set([
  VIVY_CMD,
  'speech_config_get',
  'speech_config_update',
  'speech_credential_set',
  'speech_credential_delete',
  'speech_context_set',
  'speech_transcribe',
  'speech_synthesize',
  'speech_cancel',
  'voice_asset_import',
  'voice_asset_list',
  'voice_asset_read',
  'voice_asset_delete',
])

const PROVIDER_HOST_RE =
  /api\.deepseek\.com|api\.openai\.com|token\.sensenova\.cn|api\.siliconflow\.cn|api\.minimax|api\.anthropic\.com|generativelanguage\.googleapis|dashscope\.aliyuncs|api\.moonshot\.cn|api\.x\.ai/i

function* walk(dir) {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name)
    if (statSync(p).isDirectory()) yield* walk(p)
    else if (/\.(ts|vue)$/.test(name) && !/\.(test|spec|d)\.ts$/.test(name)) yield p
  }
}

function scriptBlocks(file, text) {
  if (!file.endsWith('.vue')) return [{ code: text, offset: 0 }]
  const blocks = []
  const re = /<script[^>]*>([\s\S]*?)<\/script>/gi
  let m
  while ((m = re.exec(text))) blocks.push({ code: m[1], offset: m.index })
  return blocks
}

function lineOf(code, pos) {
  return code.slice(0, pos).split('\n').length
}

function scanFile(absPath, rel) {
  const violations = []
  const dormant = rel.startsWith(DORMANT_PREFIX)
  if (dormant) return violations
  const text = readFileSync(absPath, 'utf8')
  for (const { code } of scriptBlocks(absPath, text)) {
    const sf = ts.createSourceFile(absPath, code, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX)
    const report = (pos, msg) => violations.push(`${rel}:${lineOf(code, pos)} ${msg}`)
    // Function declarations in this file (local helpers) are not tauri
    // invokes; variable bindings still count (`const invoke = tauri.invoke`).
    const locals = new Set()
    const collect = (node) => {
      if (ts.isFunctionDeclaration(node) && node.name) locals.add(node.name.text)
      node.forEachChild(collect)
    }
    collect(sf)
    const visit = (node) => {
      if (ts.isImportDeclaration(node) && ts.isStringLiteral(node.moduleSpecifier)) {
        const mod = node.moduleSpecifier.text
        if (mod === '@tauri-apps/api/core' && rel !== INVOKE_FILE) {
          report(node.pos, `imports @tauri-apps/api/core outside ${INVOKE_FILE}`)
        } else if (mod.startsWith('@tauri-apps/') && mod !== '@tauri-apps/api/core' && !NATIVE_FILES.has(rel)) {
          report(node.pos, `imports ${mod} outside the frozen native seam`)
        }
      }
      if (ts.isCallExpression(node)) {
        const callee = node.expression
        const isDynImport =
          callee.kind === ts.SyntaxKind.ImportKeyword &&
          node.arguments[0] && ts.isStringLiteral(node.arguments[0]) &&
          node.arguments[0].text === '@tauri-apps/api/core'
        if (isDynImport && rel !== INVOKE_FILE) {
          report(node.pos, `dynamic import of @tauri-apps/api/core outside ${INVOKE_FILE}`)
        }
        let invokeLike = false
        let name = null
        if (ts.isIdentifier(callee) && (callee.text === 'invoke' || callee.text === 'invokeCommand') && !locals.has(callee.text)) {
          invokeLike = true
        } else if (ts.isPropertyAccessExpression(callee) && callee.name.text === 'invoke') {
          invokeLike = true
        }
        if (invokeLike) {
          const arg = node.arguments[0]
          if (arg && ts.isStringLiteral(arg)) name = arg.text
          if (rel === INVOKE_FILE) {
            if (!NATIVE_CMDS.has(name)) report(node.pos, `invokes non-frozen command ${name ?? '<dynamic>'}`)
          } else {
            report(node.pos, name ? `legacy business invoke '${name}'` : 'dynamic/aliased business invoke')
          }
        }
        if (ts.isIdentifier(callee) && callee.text === 'fetch') {
          report(node.pos, 'direct fetch() — business traffic must go through vivy_call')
        }
      }
      if (ts.isNewExpression(node)) {
        const t = ts.isIdentifier(node.expression) ? node.expression.text : ''
        if (t === 'EventSource' || t === 'WebSocket' || t === 'XMLHttpRequest') {
          report(node.pos, `new ${t}() — Manager HTTP/SSE channels are retired`)
        }
      }
      node.forEachChild(visit)
    }
    visit(sf)
    if (!PROVIDER_URL_FILES.has(rel) && PROVIDER_HOST_RE.test(code)) {
      violations.push(`${rel}: provider endpoint literal outside ${[...PROVIDER_URL_FILES].join(', ')}`)
    }
  }
  return violations
}

const violations = []
for (const f of walk(SRC)) violations.push(...scanFile(f, relative(join(REPO_ROOT, 'agent-diva-gui'), f)))

if (process.argv.includes('--selftest')) {
  const fx = join(REPO_ROOT, 'scripts', 'ci', 'fixtures', 'legacy-calls')
  let fired = 0, expected = 0
  for (const name of readdirSync(fx)) {
    if (!name.endsWith('.ts')) continue
    // seam-* fixtures scan under the frozen INVOKE_FILE path so the
    // inside-seam deny rule (non-allowlisted literal commands) is exercised.
    const rel = name.startsWith('seam-') ? INVOKE_FILE : `scripts/ci/fixtures/legacy-calls/${name}`
    const v = scanFile(join(fx, name), rel)
    expected++
    if (v.length > 0) fired++
    else violations.push(`selftest fixture ${name} produced NO violation`)
  }
  console.log(`selftest: ${fired}/${expected} fixtures fired`)
}

if (violations.length) {
  console.error('LEGACY-CALL GATE VIOLATIONS:')
  for (const v of violations) console.error('  ' + v)
  process.exit(1)
}
console.log('legacy frontend calls gate clean')
