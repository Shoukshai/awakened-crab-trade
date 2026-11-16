#!/usr/bin/env node
// Offline differential test. Executes unchanged upstream TypeScript against its
// real binary matcher index, then compares the native batch diagnostic.
const fs = require('node:fs');
const path = require('node:path');
const vm = require('node:vm');
const cp = require('node:child_process');
const assert = require('node:assert/strict');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const ts = require(path.join(root, 'renderer/node_modules/typescript'));
const source = rel => fs.readFileSync(path.join(root, rel), 'utf8');
const data = source('renderer/public/data/en/stats.ndjson');
const binaryIndex = fs.readFileSync(path.join(root, 'renderer/public/data/en/stats-matcher.index.bin'));
function lookup(text) {
  let hash = 2166136261;
  for (const byte of Buffer.from(text)) hash = Math.imul(hash ^ byte, 16777619) >>> 0;
  let left = 0, right = binaryIndex.length / 8 - 1;
  while (left <= right) {
    const mid = Math.floor((left + right) / 2), h = binaryIndex.readUInt32LE(mid * 8);
    if (h < hash) left = mid + 1;
    else if (h > hash) right = mid - 1;
    else {
      const offset = binaryIndex.readUInt32LE(mid * 8 + 4);
      const record = JSON.parse(data.slice(offset, data.indexOf('\n', offset)));
      const stats = record.stats ?? [record];
      return stats.some(s => s.matchers.some(m => m.string === text || m.advanced === text)) ? record : undefined;
    }
  }
}
function moduleFrom(rel, imports = {}) {
  const js = ts.transpileModule(source(rel), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const exports = {};
  const run = vm.runInThisContext(`(function(exports,require){${js}\n})`, {filename: rel});
  run(exports, name => { assert(name in imports, `Unexpected upstream import: ${name}`); return imports[name]; });
  return exports;
}
const client = moduleFrom('renderer/public/data/en/client_strings.js').default;
const meta = moduleFrom('renderer/src/parser/meta.ts');
const translation = moduleFrom('renderer/src/parser/stat-translations.ts', {
  '@/assets/data': { CLIENT_STRINGS: client, STAT_BY_MATCH_STR_V2: lookup }, './meta': meta,
});
const advanced = moduleFrom('renderer/src/parser/advanced-mod-desc.ts', {
  '@/assets/data': {CLIENT_STRINGS: client}, './modifiers': {ModifierType:{Explicit:'explicit',Implicit:'implicit',Crafted:'crafted',Fractured:'fractured'},ModifierMechanic:{}},
});
function nativeFields(parsed) {
  if (!parsed) return null;
  const {stat, roll} = parsed;
  return {reference:stat.ref, ids:stat.trade.ids[currentKind], value:roll?.value ?? null,
    min:roll?.min ?? null, max:roll?.max ?? null, decimal:!!roll?.dp,
    legacy:roll?.generation==='legacy', option:!!stat.trade.option,
    inverted:!!stat.trade.inverted, better:stat.better};
}
let currentKind;
const cases = [], seen = new Set();
function add(text,kind,category) {
  const input = {text,kind,category:category ?? null};
  const key = JSON.stringify(input);
  if (!seen.has(key)) { seen.add(key); cases.push(input); }
}
const representative = c => ({WEAPON:'Wand',ARMOUR:'Helmet',HEIST_EQUIPMENT:'Heist Tool'})[c] ?? c;
for (const line of data.trimEnd().split('\n')) {
  const record = JSON.parse(line);
  for (const stat of record.stats ?? [record]) {
    for (const matcher of stat.matchers) {
      for (const template of new Set([matcher.string, matcher.advanced].filter(Boolean))) {
        const categories = new Set([null,...(record.resolve?.test ?? []).map(representative)]);
        for (const kind of Object.keys(stat.trade.ids)) {
          for (const category of categories) {
            let n=0;
            add(template.replace(/#/g, () => (++n%2 ? '7':'13')),kind,category);
            if (template.includes('#')) {
              n=0; add(template.replace(/#/g, () => (++n%2 ? '7(1-10)':'13(11-20)')),kind,category);
              n=0; add(template.replace(/#/g, () => (++n%2 ? '-7(-12--3)':'-13(-20--11)')),kind,category);
              n=0; add(template.replace(/#/g, () => (++n%2 ? '1.25(0.5-2.75)':'3.75(3-5)')),kind,category);
            }
          }
        }
      }
    }
  }
}
for (const text of ['-4(-8--2)% reduced Movement Speed','8(12-4)% increased Spell Damage','18(8-12)% increased Spell Damage','unrecognized 99','Passives in Radius of Wicked Ward() can be Allocated']) add(text,'explicit',null);
const baseCount = cases.length;
for(let i=0;i<baseCount;i+=97) {
  cases.push({...cases[i],effect:20});
  cases.push({...cases[i],effect:50,unscalable:true});
}
const binary = path.resolve(process.argv[2] ?? path.join(root, 'rust/target/debug/examples/stat-translation'));
const native = cp.spawnSync(binary, {input:JSON.stringify(cases),encoding:'utf8',maxBuffer:128*1024*1024});
assert.equal(native.status,0,native.stderr);
const actual = JSON.parse(native.stdout);
assert.equal(actual.length,cases.length);
const failures=[];
let translated=0, mismatch_count=0;
for (let i=0;i<cases.length;i++) {
  const input=cases[i]; currentKind=input.kind;
  const parsed = translation.tryParseTranslation({string:input.text,unscalable:!!input.unscalable},input.kind,input.category ?? undefined);
  const expected = nativeFields(parsed);
  if (parsed && input.effect) {
    const applied=advanced.applyIncr({rollIncr:input.effect},parsed);
    if(applied?.roll) Object.assign(expected,{value:applied.roll.value,min:applied.roll.min,max:applied.roll.max});
  }
  const got = actual[i] === null ? null : Object.fromEntries(Object.keys(expected ?? nativeFields({stat:{ref:'',trade:{ids:{},option:false},better:0}})).map(k=>[k,actual[i][k]]));
  try { assert.deepEqual(got,expected); if(expected) translated++; }
  catch { mismatch_count++; if(failures.length<12) failures.push({input,expected,actual:got}); }
}
const report={cases:cases.length,translated,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',
  effect_cases:cases.length-baseCount,
  source_sha256:crypto.createHash('sha256').update(source('renderer/src/parser/stat-translations.ts')).digest('hex'),
  scope:'Generated matcher strings and effect scaling, real upstream binary index and unchanged TypeScript; not full item parsing or UI parity'};
if(process.argv[3]) fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
if(failures.length) process.exitCode=1;
