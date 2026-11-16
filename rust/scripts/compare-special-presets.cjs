#!/usr/bin/env node
// Full upstream English parser, special presets and effective Trade bodies.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init();
 const source=host.load('renderer/src/web/price-check/filters/create-presets.ts'),trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts');
 const stats=fs.readFileSync(path.join(host.root,'renderer/public/data/en/stats.ndjson'),'utf8').trim().split('\n').flatMap(l=>{const s=JSON.parse(l);return s.stats??[s];});
 const rules=JSON.parse(fs.readFileSync(path.join(host.root,'rust/src/data/special-rules.json'),'utf8'));
 const cases=[];
 const text=(base,sections,level=83)=>`Item Class: Synthetic Fixtures\nRarity: Normal\n${base}\n--------\nArea Level: ${level}\n--------\n${sections.join('\n--------\n')}`;
 const add=(text,label,range=10,enable_all=false)=>cases.push({text,label,range,enable_all});
 const room=r=>r.replace('Has Room: ','');
 for(const r of rules.atzoatl)for(const state of [1,2])for(const explosives of [0,1,2])for(const enable of [false,true]){
  let lines=['Open Rooms:'];
  if(explosives===1)lines.push('Explosives Room (Tier 1)');
  if(state===2||explosives===2)lines.push('Obstructed Rooms:');
  if(explosives===2)lines.push('Demolition Lab (Tier 2)');
  // Use distinct rooms to avoid invalid duplicate room states in fixtures.
  if(r.ref.includes('Explosives Room')&&explosives===1||r.ref.includes('Demolition Lab')&&explosives===2)continue;
  lines.push(room(r.ref));
  add(text('Chronicle of Atzoatl',[lines.join('\n')]),`temple ${r.ref} state=${state} explosives=${explosives} enable=${enable}`,10,enable);
 }
 const reflections=stats.filter(s=>s.ref.startsWith('Reflection of ')&&s.trade.ids.pseudo);
 for(const s of reflections)for(const difficulty of [1,7,8,16])for(const enable of [false,true])for(const range of [0,10]){
  // Eight physical lines are required by the real clipboard parser. Repeated
  // reflection tiles must use max difficulty, including lower duplicate tiles.
  const lines=[s.matchers[0].string.replace('#',difficulty),s.matchers[0].string.replace('#',difficulty===16?3:1),...Array(6).fill('Reflection of Delirium (Difficulty 2)')];
  add(text('Mirrored Tablet',[lines.join('\n')]),`tablet ${s.ref} difficulty=${difficulty} enable=${enable}`,range,enable);
 }
 const factions=stats.filter(s=>s.ref.startsWith('Has Logbook Faction:'));
 const bosses=stats.filter(s=>s.better===0&&s.trade.ids.implicit?.some(id=>id.startsWith('implicit.stat_3159649981|')));
 assert.equal(bosses.length,4);
 for(const rarity of ['Normal','Magic','Rare'])for(let zones=1;zones<=3;zones++)for(const start of [0,1,2,3])for(const boss of [null,...bosses])for(const enable of [false,true])for(const range of [0,10]){
  const areas=Array.from({length:zones},(_,i)=>`Synthetic Area ${i+1}\n${factions[(start+i)%factions.length].matchers[0].string}\n${i===zones-1&&boss?boss.matchers[0].string:'20(10-30)% increased number of Runic Monster Markers'} (implicit)`);
  add(text('Expedition Logbook',areas).replace('Rarity: Normal\nExpedition Logbook',`Rarity: ${rarity}\n${rarity==='Rare'?'Synthetic Fixture\n':''}Expedition Logbook`),`logbook ${rarity} zones=${zones} faction=${start} boss=${boss?.ref} enable=${enable}`,range,enable);
 }
 for(const name of ['synthetic-logbook.txt','synthetic-atzoatl.txt','synthetic-mirrored-tablet.txt'])for(const enable of [false,true])add(fs.readFileSync(path.join(host.root,'rust/src/fixtures',name),'utf8'),'fixture '+name,10,enable);
 // Actual ordinary copied wand remains a guard against section-parser leakage.
 add(fs.readFileSync(path.join(host.root,'rust/src/fixtures/driftwood-wand.txt'),'utf8'),'ordinary real wand');
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/equipment-presets'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:128*1024*1024});assert.equal(native.status,0,native.stderr);const output=JSON.parse(native.stdout);assert.equal(output.length,cases.length);
 const mode=id=>({'filters.preset_exact':'exact'}[id]??id);
 const compact=f=>({reference:f.statRef,ids:f.tradeId,value:f.roll?.value??null,min:f.roll?.min??null,max:f.roll?.max??null,option:f.option?.value??null,disabled:!!f.disabled,hidden:f.hidden??null,not:!!f.not,tag:f.tag});
 const compactNative=c=>({reference:c.modifier.reference,ids:c.modifier.ids,value:c.modifier.option?null:c.display_value,min:c.initial_min,max:c.initial_max,option:c.modifier.option?c.modifier.value:null,disabled:c.disabled,hidden:c.hidden,not:c.exclude,tag:c.tag});
 const cleanQuery=q=>{q=JSON.parse(JSON.stringify(q));q.query.stats=q.query.stats.filter(g=>!g.disabled).map(({disabled,...g})=>({...g,filters:g.filters.filter(f=>!f.disabled).map(({disabled,...f})=>f)}));return q;};
 let mismatch_count=0;const failures=[];
 for(let i=0;i<cases.length;i++)try{
  const parsed=parser.parseClipboard(cases[i].text);assert(parsed.isOk(),String(parsed.error));
  const expected=source.createPresets(parsed.value,{searchStatRange:cases[i].range,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,useEn:true});
  const actual=output[i];
  if(cases[i].label.startsWith('temple'))assert.deepEqual(actual.unknown,parsed.value.unknownModifiers.map(m=>({text:m.text,kind:m.type})));
  assert.equal(actual.modes[0],mode(expected.active));assert.deepEqual(actual.modes,expected.presets.map(p=>mode(p.id)));
  for(const p of actual.presets){
   const e=expected.presets.find(e=>mode(e.id)===p.mode);if(cases[i].enable_all)for(const f of e.stats)f.disabled=false;
   assert.deepEqual(p.filters.map(compactNative),e.stats.map(compact));
   assert.deepEqual(p.plan,JSON.parse(JSON.stringify(e.filters)));
   assert.deepEqual(cleanQuery(p.query),cleanQuery(trade.createTradeRequest(e.filters,e.stats.filter(f=>!f.disabled))));
  }
 }catch(e){mismatch_count++;if(failures.length<8)failures.push({case:i,label:cases[i].label,error:e.message,text:cases[i].text,native:output[i]});}
 const sources=['renderer/src/parser/Parser.ts','renderer/src/web/price-check/filters/create-presets.ts','renderer/src/web/price-check/filters/create-stat-filters.ts','renderer/src/web/price-check/filters/pseudo/atzoatl-rules.ts','renderer/src/web/price-check/filters/pseudo/reflection-rules.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts'];
 const report={cases:cases.length,temple_cases:cases.filter(c=>c.label.startsWith('temple')).length,tablet_cases:cases.filter(c=>c.label.startsWith('tablet')).length,logbook_cases:cases.filter(c=>c.label.startsWith('logbook')).length,logbook_presets:output.flatMap(o=>o.presets).filter(p=>['I','II','III'].includes(p.mode)).length,mismatch_count,failures,upstream_commit:rules.upstream_commit,catalogue_room_anomalies:rules.atzoatl.filter(r=>!stats.find(s=>s.ref===r.ref)?.trade.option).map(r=>r.ref),scope:'Full unchanged upstream English parser, temple/tablet/logbook presets, complete item-filter models and effective Trade query bodies. All room rules tested with printed room names (one malformed catalogue entry remains unknown), all reflection stats and max duplicate difficulty, four factions/four bosses, 1-3 logbook zones. Initial selections and explicit selection of all retained filters. Synthetic special items; actual copied ordinary wand. Mercenary excluded.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
