#!/usr/bin/env node
const fs = require('node:fs'), path = require('node:path'), cp = require('node:child_process'), vm = require('node:vm'), assert = require('node:assert/strict'), crypto = require('node:crypto');
const root = path.resolve(__dirname, '../..'), ts = require(root + '/renderer/node_modules/typescript');
const source = 'renderer/src/web/overlay/OverlayWindow.vue';
const text = fs.readFileSync(root + '/' + source, 'utf8').match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
const ast = ts.createSourceFile(source, text, ts.ScriptTarget.Latest, true);
let bring, topmost;
function walk(n) {
  if (ts.isFunctionDeclaration(n) && n.name?.text === 'bringToTop') bring = n.getText(ast);
  if (ts.isVariableDeclaration(n) && n.name.getText(ast) === 'topmostWidget' && ts.isCallExpression(n.initializer)) topmost = n.initializer.arguments[0].getText(ast);
  ts.forEachChild(n, walk);
}
walk(ast); assert(bring && topmost);
const js = ts.transpileModule(`(function(AppConfig){const topmostWidget={get value(){return (${topmost})()}};${bring};return bringToTop;})`, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
const make = vm.runInNewContext(js);
let seed = 17019;
function random(n) { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed % n; }
const cases = [], expected = [];
for (let i = 0; i < 256; i++) {
  const order = Array.from({ length: 2 + random(47) }, (_, n) => ({ type: n % 2 ? 'image' : 'stopwatch', id: n + 1 }));
  const widgets = order.map((key, n) => ({ wmId: n + 1, wmZorder: n + 1, key }));
  const raise = make(() => ({ widgets })), raises = [], states = [];
  for (let j = 0; j < 128; j++) {
    const target = random(widgets.length); raises.push(order[target]); raise(target + 1);
    states.push(widgets.slice().sort((a,b)=>a.wmZorder-b.wmZorder).map(w=>w.key));
  }
  cases.push({ order, raises }); expected.push(states);
}
const p = cp.spawnSync(process.argv[2] || root + '/rust/target/debug/examples/widget-order', { input: JSON.stringify(cases), encoding: 'utf8', maxBuffer: 128 * 1024 * 1024 });
assert.equal(p.status, 0, p.stderr); assert.deepEqual(JSON.parse(p.stdout), expected);
const report = { sequences: cases.length, steps: 32768, mismatch_count: 0, scope: 'Unchanged upstream bringToTop and topmostWidget callback; relative ordering of nonexclusive, initialized widgets. Native drawing, clicks, persistence, and exclusive views tested separately.', source_sha256: { [source]: crypto.createHash('sha256').update(fs.readFileSync(root + '/' + source)).digest('hex') } };
if (process.argv[3]) fs.writeFileSync(process.argv[3], JSON.stringify(report, null, 2) + '\n'); console.log(JSON.stringify(report, null, 2));
