#!/usr/bin/env node
// Exports the unchanged upstream rule table, without loading the web UI.
const fs=require('node:fs'), path=require('node:path'), vm=require('node:vm'), crypto=require('node:crypto'), assert=require('node:assert/strict');
const root=path.resolve(__dirname,'../..');
const ts=require(path.join(root,'renderer/node_modules/typescript'));
const relative='renderer/src/web/price-check/filters/pseudo/index.ts';
const source=fs.readFileSync(path.join(root,relative),'utf8');
const js=ts.transpileModule(source+'\nexport { PSEUDO_RULES };',{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
const exportsObject={};
vm.runInThisContext(`(function(exports,require){${js}\n})`,{filename:relative})(exportsObject,name=>{
  if(name==='@/assets/data') return {stat:s=>s};
  if(name==='@/parser/ParsedItem')return {ItemRarity:{Unique:'Unique'}};
  if(name==='@/parser/modifiers')return {ModifierType:{Explicit:'explicit',Crafted:'crafted',Pseudo:'pseudo'}};
  if(name==='../create-stat-filters')return {};
  throw new Error('Unexpected rule-table import: '+name);
});
const rules=exportsObject.PSEUDO_RULES.map(rule=>{
  if(!rule.mutate)return rule;
  assert.equal(rule.pseudo,'+#% total to Chaos Resistance');
  return {...rule,mutate:'chaos_visibility'};
});
const data={source:relative,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',source_sha256:crypto.createHash('sha256').update(source).digest('hex'),rules};
const output=path.join(root,'rust/src/data/pseudo-rules.json');
if(process.argv.includes('--check'))assert.deepEqual(JSON.parse(fs.readFileSync(output,'utf8')),JSON.parse(JSON.stringify(data)));
else fs.writeFileSync(output,JSON.stringify(data,null,2)+'\n');
console.log(`${rules.length} upstream pseudo rules ${process.argv.includes('--check')?'verified':'exported'}`);
