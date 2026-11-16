#!/usr/bin/env node
// Full English parser + unchanged resolver callback + makeIdentifiedUnique.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),vm=require('node:vm'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs'),ts=require(path.join(host.root,'renderer/node_modules/typescript'));
const read=p=>fs.readFileSync(path.join(host.root,p),'utf8');
const file='renderer/src/web/price-check/unidentified-resolver/UnidentifiedResolver.vue';
function resolverCallback(name){
 const text=read(file).match(/<script[^>]*>([\s\S]*?)<\/script>/)[1],src=ts.createSourceFile(file,text,ts.ScriptTarget.Latest,true);let arrow;
 function walk(n){if(ts.isVariableDeclaration(n)&&n.name.getText(src)===name)arrow=n.initializer.arguments[0].getText(src);ts.forEachChild(n,walk);}walk(src);assert(arrow,name);
 const source=`result=(props,select,ITEMS_ITERATOR,ItemRarity)=>(${arrow})();`;
 const context={};vm.runInNewContext(ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText,context);return context.result;
}
const clean=x=>JSON.parse(JSON.stringify(x));
const query=q=>{q=clean(q);q.query.stats=q.query.stats.filter(g=>!g.disabled).map(({disabled,...g})=>({...g,filters:g.filters.filter(f=>!f.disabled).map(({disabled,...f})=>f)}));return q;};
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts'),presets=host.load('renderer/src/web/price-check/filters/create-presets.ts'),trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts'),details=host.load('renderer/src/web/price-check/trends/getDetailsId.ts');
 const resolver=resolverCallback('identifiedVariants'),show=resolverCallback('show');
 const entries=read('renderer/public/data/en/items.ndjson').trim().split('\n').map(JSON.parse),groups=new Map();
 for(const e of entries.filter(e=>e.namespace==='UNIQUE')){const group=groups.get(e.unique.base)??[];if(!group.some(u=>u.refName===e.refName))group.push(e);groups.set(e.unique.base,group);}
 const cases=[],rejected=[];let automatic_groups=0;
 const add=(text,label,select=null)=>{const p=parser.parseClipboard(text);if(p.isErr()){rejected.push({label,error:String(p.error)});return;}cases.push({text,label,select});};
 const text=(base,level=84,ending='',props='',rarity='Unique')=>`Item Class: ${base.craftable?.category==='Map'?'Maps':'Synthetic Fixtures'}\nRarity: ${rarity}\n${rarity==='Rare'?'Synthetic Fixture\n':''}${base.name}\n--------\n${props?props+'\n--------\n':''}Item Level: ${level}\n--------\nUnidentified\n${ending?"--------\n"+ending:""}`;
 for(const [baseName,unique] of groups){
  const base=entries.find(e=>e.namespace==='ITEM'&&e.refName===baseName);assert(base,baseName);
  const raw=text(base);add(raw,'choices '+baseName);if(unique.length===1)automatic_groups++;
  for(const u of unique)for(const level of [1,84])for(const ending of ['', 'Corrupted'])add(text(base,level,ending),'select '+u.refName+' '+level+' '+ending,u.refName);
 }
 for(const [baseName,name] of [['Prismatic Jewel',"Watcher's Eye"],['Imperial Staff','Agnerod East'],['Driftwood Wand','Lifesprig'],['Steelscale Gauntlets','Tombfist']]){
  const base=entries.find(e=>e.namespace==='ITEM'&&e.refName===baseName);assert(base);
  for(const level of [68,74,75,78,80,81,82,83,85,86,100])for(const ending of ['Foil Unique','Mirrored','Split','Shaper Item'])add(text(base,level,ending,'Sockets: R-R-R-R-R-R'),'special '+name+' '+level+' '+ending,name);
 }
 for(const fixture of ['driftwood-wand.txt','scroll-of-wisdom.txt','small-life-flask.txt','synthetic-sacrifice-fragment.txt'])add(read('rust/src/fixtures/'+fixture),'ineligible '+fixture);
 for(const [fixture,identity] of [['synthetic-unidentified-belt.txt','Mageblood'],['synthetic-unidentified-wand.txt','Lifesprig']]){const raw=read('rust/src/fixtures/'+fixture);add(raw,'fixture choices '+fixture);add(raw,'fixture selected '+fixture,identity);}
 const noUnique=entries.find(e=>e.namespace==='ITEM'&&e.refName==='Accumulator Wand');add(text(noUnique),'no candidate');
 const belt=entries.find(e=>e.namespace==='ITEM'&&e.refName==='Heavy Belt');
 for(const rarity of ['Normal','Magic','Rare'])add(text(belt,84,'','',rarity),'ineligible '+rarity);
 add(text(belt),'wrong base','Headhunter');
 add(text(belt),'unknown name','Unknown Unique');
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/unidentified'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:256*1024*1024});assert.equal(native.status,0,native.stderr);const output=JSON.parse(native.stdout);assert.equal(output.length,cases.length);
 let mismatch_count=0,selected_count=0,source_details_errors=0;const failures=[];
 for(let i=0;i<cases.length;i++)try{
  const c=cases[i],original=parser.parseClipboard(c.text).value,props={item:original},automatic=[];
  const eligible=show(props,()=>{},data.ITEMS_ITERATOR,parser.ItemRarity);assert.equal(output[i].needs_identity,eligible);
  const choices=eligible?resolver(props,u=>automatic.push(u.refName),data.ITEMS_ITERATOR,parser.ItemRarity):[];
  assert.deepEqual(output[i].choices,clean(choices.map(e=>({reference:e.refName,name:e.name}))));
  assert.deepEqual(automatic,choices.length===1?[choices[0].refName]:[]);
  if(c.select){
   const choice=choices.find(e=>e.refName===c.select);
   if(!choice){assert(output[i].selected.Err,'wrong identity must be rejected');continue;}
   const expected=parser.makeIdentifiedUnique(choice,original),actual=output[i].selected.Ok;assert(actual,output[i].selected.Err);selected_count++;
   assert.equal(actual.item.namespace,expected.info.namespace);assert.equal(actual.item.reference,expected.info.refName);assert.equal(actual.item.market.name,expected.info.name);assert.equal(actual.item.market.unique_base,expected.uniqueBase.refName);
   assert.equal(actual.item.unidentified,true);assert.equal(actual.item.raw,c.text);assert.equal(actual.item.item_level,expected.itemLevel??null);assert.equal(actual.item.corrupted,expected.isCorrupted);assert.equal(actual.item.mirrored,!!expected.isMirrored);assert.equal(actual.item.market.foil,!!expected.isFoil);assert.equal(actual.dust,expected.dustEquivalent??null);
   let key=null;try{key=details.getDetailsId(expected);}catch{source_details_errors++;}
   assert.deepEqual(actual.key,key?{namespace:key.ns,name:key.name,variant:key.variant??null}:null);
   const p=presets.createPresets(expected,{searchStatRange:10,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,useEn:true});
   const modes=id=>({'filters.preset_exact':'exact','filters.preset_base_item':'base','filters.preset_pseudo':'calculated'}[id]??id);
   assert.deepEqual(actual.presets.map(p=>p.mode),p.presets.map(p=>modes(p.id)));
   for(let j=0;j<actual.presets.length;j++){const a=actual.presets[j],e=p.presets[j];assert.deepEqual(a.plan,clean(e.filters));assert.deepEqual(query(a.query),query(trade.createTradeRequest(e.filters,e.stats.filter(f=>!f.disabled))));assert.equal(a.query.query.filters.misc_filters.filters.identified.option,'false');}
  }else assert.equal(output[i].selected,null);
 }catch(e){mismatch_count++;if(failures.length<12)failures.push({case:i,label:cases[i].label,error:e.message,native:output[i]});}
 const sources=[file,'renderer/src/parser/Parser.ts','renderer/src/web/price-check/filters/create-presets.ts','renderer/src/web/price-check/filters/create-item-filters.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts'];
 const report={cases:cases.length,base_groups:groups.size,automatic_groups,selected_count,source_details_errors,source_parse_rejections:rejected,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Unchanged English parser, computed resolver callbacks, makeIdentifiedUnique, dust, poe.ninja identities, preset/item-filter models and complete effective Trade bodies; all catalogue base groups and deduplicated unique identities; synthetic copies. Original unidentified flag and raw text retained. Missing required unique-variant stats fail closed where upstream throws; no hidden mods inferred; no icon or live-copy parity.',source_sha256:Object.fromEntries(sources.map(p=>[p,crypto.createHash('sha256').update(read(p)).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
