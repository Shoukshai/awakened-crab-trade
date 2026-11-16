#!/usr/bin/env node
// Execute unchanged findItems/useSelectedItems/createVirtualItem and Trade rules.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),vm=require('node:vm'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs'),ts=require(path.join(host.root,'renderer/node_modules/typescript'));
const read=p=>fs.readFileSync(path.join(host.root,p),'utf8'),clean=x=>JSON.parse(JSON.stringify(x));
const file='renderer/src/web/item-search/WidgetItemSearch.vue';
const query=q=>{q=clean(q);q.query.stats=q.query.stats.filter(g=>!g.disabled).map(({disabled,...g})=>({...g,filters:g.filters.filter(f=>!f.disabled).map(({disabled,...f})=>f)}));return q;};
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts'),virtual=host.load('renderer/src/parser/ParsedItem.ts').createVirtualItem,presets=host.load('renderer/src/web/price-check/filters/create-presets.ts'),trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts');
 const source=read(file).match(/<script lang="ts">([\s\S]*?)<\/script>/)[1],ast=ts.createSourceFile(file,source,ts.ScriptTarget.Latest,true);
 const pure=ast.statements.filter(n=>ts.isFunctionDeclaration(n)&&['findItems','useSelectedItems'].includes(n.name?.text)).map(n=>n.getText(ast)).join('\n');
 const code=ts.transpileModule(pure,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
 const factory=vm.runInNewContext(`(function(ITEM_BY_TRANSLATED,AppConfig,ref){${code};return {findItems,useSelectedItems};})`);
 const original=factory(data.ITEM_BY_TRANSLATED,()=>({language:'en'}),value=>({value}));
 const setup=read(file).match(/<script setup lang="ts">([\s\S]*?)<\/script>/)[1],setupAst=ts.createSourceFile(file,setup,ts.ScriptTarget.Latest,true);
 const selectBody=setupAst.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name?.text==='selectItem').getText(setupAst);
 const selectCode=ts.transpileModule(selectBody,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;
 let selectedKey;
 const selectOriginal=vm.runInNewContext(`(function(queuePricesFetch,findPriceByQuery,autoCurrency,addItem,searchValue){${selectCode};return selectItem;})`)(()=>{},key=>{selectedKey=key;return undefined;},()=>{throw new Error('Unexpected missing-price conversion');},()=>true,{value:''});
 const replicas=[...data.REPLICA_UNIQUE_NAMES()],gems=[...data.ALTQ_GEM_NAMES()],cases=[];
 const add=(search,kind='replica')=>cases.push({search,kind});
 for(const search of ['', 'a','re','rep','replica','replica zzz','head','eadhunter','fireball','\ufeffReplica Headhunter\ufeff','😀','😀x','repl\u0085hea']){add(search);add(search,'gem');}
 for(const name of replicas){
  const words=name.split(/\s+/);for(const q of [name,name.toUpperCase(),'  '+name+'\t',words.slice().reverse().join(' '),...words,...words.flatMap(w=>[w.slice(0,3),w.slice(1),w.slice(0,4)]),name+' zzz'])add(q);
 }
 const baskets=[];
 for(let offset=0;offset<replicas.length;offset+=5){
  const names=Array.from({length:8},(_,i)=>replicas[(offset+i)%replicas.length]),ops=[];
  names.forEach((search,i)=>{ops.push({search,chaos:i%4===0?null:((i*37)%101)});if(i%2===0)ops.push({search,chaos:999});});baskets.push(ops);
 }
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/item-search'));
 const proc=cp.spawnSync(binary,{input:JSON.stringify({cases,baskets}),encoding:'utf8',maxBuffer:256*1024*1024});assert.equal(proc.status,0,proc.stderr);const output=JSON.parse(proc.stdout);assert.equal(output.cases.length,cases.length);
 let mismatches=0,selected=0,basket_steps=0;const failures=[];
 const itemNames=kind=>kind==='gem'?data.ALTQ_GEM_NAMES:data.REPLICA_UNIQUE_NAMES;
 for(let i=0;i<cases.length;i++)try{
  const c=cases[i],expected=original.findItems({search:c.search,namespace:c.kind==='gem'?'GEM':'UNIQUE',itemNames:itemNames(c.kind)}),a=output.cases[i];
  assert.deepEqual(a.choices?.map(({id,...c})=>c)??null,expected===false?null:clean(expected.map(e=>({name:e.name,reference:e.refName,namespace:e.namespace,base:e.unique?.base??null}))));
  if(expected===false){assert.equal(a.virtuals,null);continue;}
  assert.equal(a.virtuals.length,expected.length);
  expected.forEach((info,j)=>{
   const v=info.namespace==='GEM'?virtual({category:parser.ItemCategory.Gem,info,gemLevel:1}):virtual({rarity:parser.ItemRarity.Unique,info}),aItem=a.virtuals[j].item;
   assert.equal(aItem.raw,v.rawText);assert.equal(aItem.name,info.name);assert.equal(aItem.reference,info.refName);assert.equal(aItem.namespace,info.namespace);assert.equal(aItem.category,v.category??null);assert.equal(aItem.item_level,v.itemLevel??null);assert.equal(aItem.gem_level,v.gemLevel??null);assert.deepEqual(aItem.modifiers,[]);assert.equal(aItem.links,0);selected++;
   const p=presets.createPresets(v,{searchStatRange:10,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,useEn:true});
   const modes=id=>({'filters.preset_exact':'exact','filters.preset_base_item':'base','filters.preset_pseudo':'calculated'}[id]??id);
   assert.deepEqual(a.virtuals[j].presets.map(p=>p.mode),p.presets.map(p=>modes(p.id)));
   a.virtuals[j].presets.forEach((ap,k)=>{assert.deepEqual(ap.plan,clean(p.presets[k].filters));assert.deepEqual(query(ap.query),query(trade.createTradeRequest(p.presets[k].filters,p.presets[k].stats.filter(f=>!f.disabled))));});
   selectOriginal(info,{unique:true});
   assert.deepEqual(a.virtuals[j].quote_identity,clean({namespace:selectedKey.ns,name:selectedKey.name,variant:selectedKey.variant??null}));
   const enriched=virtual({...v,uniqueBase:info.unique?data.ITEM_BY_REF('ITEM',info.unique.base)[0]:undefined}),ep=presets.createPresets(enriched,{searchStatRange:10,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,useEn:true});
   a.virtuals[j].enriched_presets.forEach((ap,k)=>{assert.deepEqual(ap.plan,clean(ep.presets[k].filters));assert.deepEqual(query(ap.query),query(trade.createTradeRequest(ep.presets[k].filters,ep.presets[k].stats.filter(f=>!f.disabled))));});
  });
 }catch(e){mismatches++;if(failures.length<10)failures.push({case:i,input:cases[i],error:e.message});}
 for(let i=0;i<baskets.length;i++){
  const basket=original.useSelectedItems();
  for(let j=0;j<baskets[i].length;j++)try{
   const op=baskets[i][j],info=data.ITEM_BY_TRANSLATED('UNIQUE',op.search)[0],added=basket.addItem({info,chaos:op.chaos??undefined}),actual=output.baskets[i][j];basket_steps++;
   assert.equal(actual.added,added);assert.deepEqual(actual.items.map(i=>({name:i.choice.name,chaos:i.chaos})),clean(basket.items.value.map(i=>({name:i.info.name,chaos:i.chaos??null}))));
  }catch(e){mismatches++;if(failures.length<10)failures.push({basket:i,step:j,error:e.message});}
 }
 const sources=[file,'renderer/src/parser/ParsedItem.ts','renderer/src/assets/data/index.ts','renderer/src/web/price-check/filters/create-presets.ts','renderer/src/web/price-check/filters/create-item-filters.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts'];
 const report={search_cases:cases.length,virtual_consultations:selected,replica_candidates:replicas.length,legacy_gem_candidates:gems.length,basket_sequences:baskets.length,basket_steps,mismatch_count:mismatches,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Unchanged English Heist findItems and selected basket, current catalogue, createVirtualItem, explicit quote identities from unchanged selectItem, original complete preset/filter models and effective Trade queries, plus separate known-base enrichment compared with unchanged functions on enriched inputs. No OCR, modern-gem search, images or live pricing implied.',source_sha256:Object.fromEntries(sources.map(p=>[p,crypto.createHash('sha256').update(read(p)).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatches)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
