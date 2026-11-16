#!/usr/bin/env node
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const root=path.resolve(__dirname,'../..'),ts=require(path.join(root,'renderer/node_modules/typescript'));
const name='renderer/src/web/item-check/WidgetItemCheck.vue';
const source=fs.readFileSync(path.join(root,name),'utf8');
const script=source.match(/<script lang="ts">([\s\S]*?)<\/script>/)[1];
const js=ts.transpileModule(script,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
const out={};vm.runInThisContext(`(function(exports){${js}\n})`)(out);
const data={source:name,source_sha256:crypto.createHash('sha256').update(source).digest('hex'),config:out.default.widget.initInstance().maps};
const file=path.join(root,'rust/src/data/map-check-defaults.json');
if(process.argv.includes('--check'))assert.deepEqual(JSON.parse(fs.readFileSync(file,'utf8')),data);
else fs.writeFileSync(file,JSON.stringify(data,null,2)+'\n');
console.log('Upstream map-check defaults '+(process.argv.includes('--check')?'verified':'exported'));
