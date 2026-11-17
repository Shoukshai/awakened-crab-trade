#!/usr/bin/env node
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),vm=require('node:vm'),crypto=require('node:crypto');
const root=path.resolve(__dirname,'../..'),ts=require(path.join(root,'renderer/node_modules/typescript'));
const read=p=>fs.readFileSync(path.join(root,p),'utf8');
const gridFile='renderer/src/web/overlay/WidgetDelveGrid.vue',overlayFile='renderer/src/web/overlay/OverlayWindow.vue';
function script(file,setup=false){return read(file).match(setup?/<script setup lang="ts">([\s\S]*?)<\/script>/:/<script lang="ts">([\s\S]*?)<\/script>/)[1];}
function parsed(file,setup){const source=script(file,setup);return ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true);}
function find(ast,predicate){let result;function walk(n){if(predicate(n))result=n;ts.forEachChild(n,walk);}walk(ast);assert(result,'required unchanged AST node');return result.getText(ast);}
function evaluate(source){return vm.runInNewContext(ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText);}
const gridAst=parsed(gridFile,true),overlayAst=parsed(overlayFile,false);
const anchor=find(gridAst,n=>ts.isVariableDeclaration(n)&&n.name.getText(gridAst)==='anchor');
const anchorBody=ts.createSourceFile('anchor',anchor,ts.ScriptTarget.Latest,true).statements[0];
// Select the callback expression itself; its arithmetic is never rewritten.
let callback;function capture(n){if(ts.isVariableDeclaration(n)&&n.name.getText(gridAst)==='anchor')callback=n.initializer.arguments[0].getText(gridAst);ts.forEachChild(n,capture);}capture(gridAst);
const geometry=evaluate(`(function(wm){return (${callback})();})`);
function eventCallback(ast,name){let arrow;function walk(n){if(ts.isCallExpression(n)&&n.expression.getText(ast)==='Host.onEvent'&&ts.isStringLiteral(n.arguments[0])&&n.arguments[0].text===name)arrow=n.arguments[1].getText(ast);ts.forEachChild(n,walk);}walk(ast);assert(arrow,name);return arrow;}
const toggle=evaluate(`(function(props,wm,Host,e){return (${eventCallback(gridAst,'MAIN->CLIENT::widget-action')})(e);})`);
const focus=evaluate(`(function(active,gameFocused,widgets,hide,state){return (${eventCallback(overlayAst,'MAIN->OVERLAY::focus-change')})(state);})`);
let visibilityBody;function captureVisibility(n){if(ts.isVariableDeclaration(n)&&n.name.getText(overlayAst)==='visibilityState')visibilityBody=n.initializer.arguments[0].getText(overlayAst);ts.forEachChild(n,captureVisibility);}captureVisibility(overlayAst);assert(visibilityBody);
const visibility=evaluate(`(function(AppConfig,hideUI,active){return (${visibilityBody})();})`);
const showBody=find(overlayAst,n=>ts.isFunctionDeclaration(n)&&n.name.text==='show');
const show=evaluate(`(function(widgets,bringToTop,topmostOrExclusiveWidget,hide,id){${showBody};show(id);})`);
const viewports=[];
for(let h=1;h<=4320;h++)for(const ratio of [4/3,16/9,21/9])viewports.push([Math.max(1,Math.round(h*ratio)),h]);
for(const [w,h] of [[2560,1600],[1280,800],[1920,1080],[3840,2160],[1707,1067],[5120,1440]])viewports.push([w,h]);
const actions=['toggle','focus','blur','paint','hide','exclusive-on','exclusive-off'],sequences=[];
for(let seed=0;seed<128;seed++){let state=seed+1;const steps=[];for(let i=0;i<30;i++){state=(Math.imul(state,1664525)+1013904223)>>>0;steps.push(actions[state%actions.length]);}sequences.push(steps);}
const binary=path.resolve(process.argv[2]??path.join(root,'rust/target/debug/examples/delve-grid'));
const proc=cp.spawnSync(binary,{input:JSON.stringify({viewports,sequences}),encoding:'utf8',maxBuffer:32*1024*1024});assert.equal(proc.status,0,proc.stderr);const native=JSON.parse(proc.stdout);assert.equal(native.geometry.length,viewports.length);
let mismatch_count=0;const failures=[];
for(let i=0;i<viewports.length;i++)try{
 const [w,h]=viewports[i],a=geometry({size:{value:{width:w,height:h}}}),g=native.geometry[i];
 assert.equal(g.width,a.width);assert.equal(g.height,a.height);assert.equal(g.cell,a.cell);assert.equal(g.rows,Number(read(gridFile).match(/v-for="i in (\d+)"/)[1]));assert.equal(g.columns,Number(read(gridFile).match(/v-for="j in (\d+)"/)[1]));
 assert.equal(g.left,w*a.x/100-a.width/2);assert(Math.abs(g.top-h*a.y/100)<1e-8);
 assert.equal(g.grid_left,g.left+(a.width-a.cell*11)/2);assert(Math.abs(g.grid_top-(g.top+(a.height-a.cell*9)/2))<1e-8);
}catch(e){mismatch_count++;if(failures.length<10)failures.push({viewport:viewports[i],error:e.message});}
for(let i=0;i<sequences.length;i++)try{
 const config={wmId:1,wmWants:'hide',wmZorder:null,wmFlags:['hide-on-focus','menu::skip']},exclusive={wmId:2,wmWants:'hide',wmZorder:'exclusive',wmFlags:[]},props={config},wm={show(){show(widgets,()=>{}, {value:exclusive.wmWants==='show'?exclusive:config},id=>{widgets.value.find(w=>w.wmId===id).wmWants='hide';},1);},hide(){config.wmWants='hide';}},active={value:false},gameFocused={value:true},widgets={value:[config,exclusive]},expected=[];
 for(const action of sequences[i]){
  if(action==='toggle'){if(active.value)focus(active,gameFocused,widgets,()=>wm.hide(),{overlay:false,game:true});toggle(props,wm,{isElectron:true},{target:'delve-grid'});}
  else if(action==='focus'||action==='blur')focus(active,gameFocused,widgets,()=>wm.hide(),{overlay:action==='focus',game:action==='blur'});
  else if(action==='hide')wm.hide();
  else if(action==='exclusive-on')exclusive.wmWants='show';
  else if(action==='exclusive-off')exclusive.wmWants='hide';
  expected.push({wanted:config.wmWants==='show',visible:visibility(()=>({widgets:widgets.value}),{value:false},active).find(w=>w.wmId===1).isVisible});
 }
 assert.deepEqual(native.sequences[i],expected);
}catch(e){mismatch_count++;if(failures.length<10)failures.push({sequence:i,error:e.message});}
const color=require(path.join(root,'renderer/tailwind.config.js')).theme.colors.gray[600];assert.deepEqual(native.border_rgb,[1,3,5].map(offset=>parseInt(color.slice(offset,offset+2),16)));
assert(read(gridFile).includes("background: 'rgba(255,255,255,0.1)'"));
const sources=[gridFile,overlayFile,'renderer/tailwind.config.js'];
const report={geometry_cases:viewports.length,visibility_sequences:sequences.length,visibility_steps:sequences.reduce((s,v)=>s+v.length,0),mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Unchanged WidgetDelveGrid anchor callback, palette and toggle handler; unchanged overlay show function, focus-change and visibilityState callbacks for hide-on-focus and generic exclusive-widget suppression. Toggle events occur in game context. Geometry formulas, wanted-state and visible-state parity only, no raster or desktop/backend equivalence claimed. Extra Linux context-loss guards tested separately.',source_sha256:Object.fromEntries(sources.map(p=>[p,crypto.createHash('sha256').update(read(p)).digest('hex')]))};
if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
