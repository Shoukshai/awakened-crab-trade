#!/usr/bin/env node
// Preserve upstream rule arrays verbatim through TypeScript's AST.
const fs=require('node:fs'),path=require('node:path'),vm=require('node:vm'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const host=require('./upstream-core.cjs'),ts=require(path.join(host.root,'renderer/node_modules/typescript'));
const sources=['renderer/src/web/price-check/filters/pseudo/atzoatl-rules.ts','renderer/src/web/price-check/filters/pseudo/reflection-rules.ts','renderer/public/data/en/client_strings.js'];
function rules(file){
 const source=fs.readFileSync(path.join(host.root,file),'utf8');
 const ast=ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true);
 const nodes=ast.statements.filter(n=>ts.isVariableStatement(n)&&n.declarationList.declarations.some(d=>['RULES','HIDDEN_GREEDY'].includes(d.name.getText())));
 const code=ts.transpileModule(nodes.map(n=>n.getText()).join('\n')+'\nresult=RULES;', {compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText;
 const context={stat:ref=>ref};vm.runInNewContext(code,context);return context.result;
}
const strings=host.load(sources[2]).default;
const data={upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')])),open_rooms:strings.INCURSION_OPEN,obstructed_rooms:strings.INCURSION_OBSTRUCTED,atzoatl:rules(sources[0]),reflections:rules(sources[1])};
const output=path.join(host.root,'rust/src/data/special-rules.json');
if(process.argv.includes('--check'))assert.deepEqual(JSON.parse(fs.readFileSync(output,'utf8')),JSON.parse(JSON.stringify(data)));
else fs.writeFileSync(output,JSON.stringify(data,null,2)+'\n');
console.log(`${data.atzoatl.length} room rules and ${data.reflections.length} reflection rules ${process.argv.includes('--check')?'verified':'exported'}`);
