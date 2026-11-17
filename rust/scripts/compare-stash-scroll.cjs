#!/usr/bin/env node
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'../..'),ts=require(root+'/renderer/node_modules/typescript');
const shortcuts='main/src/shortcuts/Shortcuts.ts',game='main/src/windowing/GameWindow.ts';
let area,callback,sidebar;
function scan(n,a){if(ts.isFunctionDeclaration(n)&&n.name?.text==='isStashArea')area=n.getText(a);if(ts.isCallExpression(n)&&n.expression.getText(a)==='uIOhook.on'&&n.arguments[0]?.getText(a)==="'wheel'")callback=n.arguments[1].body.getText(a);if(ts.isGetAccessor(n)&&n.name.getText(a)==='uiSidebarWidth')sidebar=n.body.getText(a);ts.forEachChild(n,x=>scan(x,a));}
for(const f of [shortcuts,game]){const a=ts.createSourceFile(f,fs.readFileSync(root+'/'+f,'utf8'),ts.ScriptTarget.Latest,true);scan(a,a);}assert(area&&callback&&sidebar);
const source=`(function(c){const actions=[];const UiohookKey={ArrowLeft:'left',ArrowRight:'right'};const uIOhook={keyTap:k=>actions.push(k)};${area};const poeWindow={bounds:c.bounds,isActive:c.active,get uiSidebarWidth()${sidebar}};const owner={poeWindow,stashScroll:c.enabled};(function(e)${callback}).call(owner,{ctrlKey:c.ctrl,x:c.x,y:c.y,rotation:c.rotation});return {inside:isStashArea(c,poeWindow),action:actions[0]??null};})`;
const oracle=vm.runInNewContext(ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022}}).outputText);
if(process.argv[2]==='--evaluate'){const input=JSON.parse(fs.readFileSync(0,'utf8'));console.log(JSON.stringify(input.map(oracle)));process.exit(0);}
const cases=[];
for(const [ox,oy] of [[0,0],[300,200],[-1920,-100]])for(const h of [150,300,600,800,1080,1200,1440,1600,2160]){
 const bounds={x:ox,y:oy,width:h*1.6,height:h},edge=ox+Math.round(h*(370/600));
 for(const x of [ox-1,ox,edge-0.01,edge,edge+0.01,ox+bounds.width])for(const y of [oy,oy+h*154/1600-0.01,oy+h*154/1600,oy+h*154/1600+0.01,oy+h*1192/1600-0.01,oy+h*1192/1600,oy+h*1192/1600+0.01,oy+h])for(const rotation of [-2,-1,0,1,2])cases.push({ctrl:true,active:true,enabled:true,x,y,rotation,bounds});
}
for(const ctrl of [false,true])for(const active of [false,true])for(const enabled of [false,true])for(const rotation of [-1,0,1])for(const bounds of [null,{x:0,y:0,width:2560,height:1600}])cases.push({ctrl,active,enabled,x:2000,y:600,rotation,bounds});
const p=cp.spawnSync(process.argv[2]||root+'/rust/target/debug/examples/stash-scroll',{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:16*1024*1024});assert.equal(p.status,0,p.stderr);const expected=JSON.parse(JSON.stringify(cases.map(oracle)));assert.deepEqual(JSON.parse(p.stdout),expected);
const report={cases:cases.length,mismatches:0,scope:'Unchanged isStashArea, wheel callback and uiSidebarWidth getter. Includes strict edges, sidebar rounding, offsets, wheel sign and activation/option guards. No drawing, XRecord availability or native key delivery claimed.',source_sha256:Object.fromEntries([shortcuts,game].map(f=>[f,crypto.createHash('sha256').update(fs.readFileSync(root+'/'+f)).digest('hex')]))};if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));
