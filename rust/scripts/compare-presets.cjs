#!/usr/bin/env node
// Full upstream parsing and specialised exact presets. Offline; no game input.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts');
 const source=host.load('renderer/src/web/price-check/filters/create-presets.ts');
 const trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts');
 const cases=[];
 function item(base,category,rarity,props,mods,ending=''){
  return `Item Class: ${category}\nRarity: ${rarity}\n${rarity==='Rare'?'Synthetic Fixture\n':''}${base}${category==='Maps'?' (Tier 16)':''}\n--------\n${props}\n--------\nItem Level: 83\n--------\n${mods}\n${ending?'--------\n'+ending:''}`;
 }
 const mod=(text,tier=1,type='Prefix Modifier')=>type==='Enchantment Modifier'?`--------\n${text} (enchant)\n--------`:`{ ${type} "Fixture" (Tier: ${tier}) }\n${text}`;
 function add(text,label,range=10){cases.push({text,label,range});}
 for(const rarity of ['Normal','Magic','Rare'])for(const range of [0,2,10,50])for(const valuable of [false,true])for(const count of [0,6,8]){
  let props='Map Area: Beach\nItem Quantity: +80% (augmented)\nItem Rarity: +40% (augmented)\nMonster Pack Size: +25% (augmented)';
  if(valuable)props+='\nMore Maps: +20%\nMore Scarabs: +40%\nMore Currency: +50%\nMore Divination Cards: +30%';
  const mods=Array.from({length:count},(_,n)=>mod(`${10+n}(5-30)% increased Monster Damage`,n+1)).join('\n');
  add(item('Map','Maps',rarity,props,mods),'map '+rarity+' '+count,range);
 }
 for(const ref of ['Blighted Map','Blight-ravaged Map'])add(item(ref,'Maps','Rare','Map Area: Beach\nItem Quantity: +80%',mod('Monsters have 20(10-30)% increased Attack Speed')),'blight');
 for(const lethal of [false,true])add(item('Map','Maps','Rare',"Map Area: Beach\nReward: Foil Kaom's Heart",lethal?mod('Players who Die in area are sent to the Void'):mod('Area contains two Unique Bosses')),'valdo');
 for(const tier of [1,2,3,8])for(const range of [0,10,50])for(const effect of [false,true])for(const enchant of ['none','instilling','enkindling']){
  const mods=[mod('30(20-40)% increased Charge Recovery',tier)];if(effect)mods.push(mod('25(20-30)% increased effect',tier));
  if(enchant==='instilling')mods.push(mod('Used when Charges reach full',1,'Enchantment Modifier'));
  if(enchant==='enkindling')mods.push(mod('Gains no Charges during Effect',1,'Enchantment Modifier'));
  add(item('Granite Flask','Utility Flasks','Magic','Quality: +20%',mods.join('\n')),'flask '+tier+' '+effect+' '+enchant,range);
 }
 const jobs=['Lockpicking','Brute Force','Perception','Demolition','Counter-Thaumaturgy','Trap Disarmament','Agility','Deception','Engineering'];
 for(const job of jobs)for(const level of [1,3,5])for(const priceless of [false,true])add(item('Contract: Bunker','Contracts','Rare',`Area Level: 83\nRequires ${job} (Level ${level})\nHeist Target: Fixture (${priceless?'Priceless':'Moderate'})`,mod('20(10-30)% increased Monster Damage')),'heist '+job);
 for(const enchant of [false,true])for(const wings of ['0/3','1/4','3/3','4/4'])add(item('Blueprint: Bunker','Blueprints','Rare',`Area Level: 83\nHeist Target: Replicas or Experimented Items\nWings Revealed: ${wings}`,enchant?mod('10% increased effect',1,'Enchantment Modifier'):''),'blueprint');
 // Current catalogue chart names are read directly below; this is test data,
 // never a substitute for parsing real copied items in the native program.
 const entries=fs.readFileSync(path.join(host.root,'renderer/public/data/en/items.ndjson'),'utf8').trim().split('\n').map(JSON.parse);
 const chart=entries.find(v=>v.namespace==='ITEM'&&v.craftable?.category==='Chart');
 const area=entries.find(v=>v.namespace==='AREA'&&!v.area?.special);
 if(chart&&area)for(const rarity of ['Normal','Magic','Rare'])add(item(chart.name,'Charts',rarity,`${area.name}\nArea Level: 83\nItem Quantity: +70%\nItem Rarity: +40%\nMonster Pack Size: +25%\nDead Man's Sulphur: +20%`,mod('20(10-30)% increased Monster Damage')),'chart');
 const special=entries.find(v=>v.namespace==='AREA'&&v.area?.special);
 if(chart&&special)add(item(chart.name,'Charts','Rare',`${special.name}\nArea Level: 83\nItem Quantity: +70%`,''),'special chart');
 for(const rarity of ['Normal','Magic','Rare'])add(item('Map','Maps',rarity,'Map Area: Beach\nItem Quantity: +80%',mod('Area contains two Unique Bosses'),'Unidentified'),'unidentified map');
 add(item('Map','Maps','Rare','Map Area: Beach\nMemory Strands: 70',mod('Area contains two Unique Bosses')),'memory map');
 for(const influenced of [false,true])add(item('Granite Flask','Utility Flasks','Magic','Quality: +20%',mod('30(20-40)% increased Charge Recovery',1)+'\n'+mod('20(10-30)% increased Duration',1,'Implicit Modifier')+'\n'+mod('10(5-15)% increased Charges gained',1,'Master Crafted Prefix Modifier'),influenced?'Shaper Item':''),'flask types');
 add(fs.readFileSync(path.join(host.root,'rust/src/fixtures/small-life-flask.txt'),'utf8'),'actual copied Small Life Flask');
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/presets'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:64*1024*1024});assert.equal(native.status,0,native.stderr);
 const output=JSON.parse(native.stdout);let mismatch_count=0;const failures=[];
 const compact=f=>({reference:f.statRef,ids:f.tradeId,value:f.roll?.value??null,min:f.roll?.min??null,max:f.roll?.max??null,disabled:!!f.disabled,hidden:f.hidden??null,not:!!f.not});
 const compactNative=c=>({reference:c.modifier.reference,ids:c.modifier.ids,value:c.display_value,min:c.initial_min,max:c.initial_max,disabled:c.disabled,hidden:c.hidden,not:c.exclude});
 const mode=id=>({'filters.preset_exact':'exact','filters.preset_bulk':'bulk','filters.preset_pseudo':'properties'}[id]??id);
 for(let i=0;i<cases.length;i++){
  try{
   const parsed=parser.parseClipboard(cases[i].text);assert(parsed.isOk(),String(parsed.error));
   const expected=source.createPresets(parsed.value,{searchStatRange:cases[i].range,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,exact:true,useEn:true});
   const actual=output[i];assert.equal(actual.modes[0],mode(expected.active));
   assert.deepEqual([...actual.modes].sort(),expected.presets.map(p=>mode(p.id)).sort());
   for(const p of actual.presets){
    const e=expected.presets.find(e=>mode(e.id)===p.mode);assert.deepEqual(p.filters.map(compactNative),e.stats.map(compact));
    const q=JSON.parse(JSON.stringify(trade.createTradeRequest(e.filters,e.stats.filter(f=>!f.disabled)))).query;
    const n=p.query.query;
    assert.deepEqual(n.type,q.type);assert.deepEqual(n.name,q.name);
    assert.deepEqual(n.filters.map_filters?.filters??{},q.filters.map_filters?.filters??{});
    assert.deepEqual(n.filters.heist_filters?.filters??{},q.filters.heist_filters?.filters??{});
    const clean=groups=>groups.map(g=>({...g,filters:g.filters.map(({disabled,...f})=>f)}));
    assert.deepEqual(clean(n.stats),clean(q.stats));
   }
  }catch(e){mismatch_count++;if(failures.length<6)failures.push({case:i,label:cases[i].label,error:e.message,text:cases[i].text,native:output[i]});}
 }
 const sources=['renderer/src/parser/Parser.ts','renderer/src/web/price-check/filters/create-presets.ts','renderer/src/web/price-check/filters/create-item-filters.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts','renderer/src/web/price-check/filters/create-stat-filters.ts','renderer/src/web/price-check/filters/pseudo/maps.ts','renderer/src/web/price-check/filters/pseudo/heist.ts','renderer/src/web/price-check/filters/pseudo/flasks.ts'];
 const report={cases:cases.length,enchant_items:output.filter(o=>o.modifiers.some(m=>m.kind==='enchant')).length,exclusion_filters:output.flatMap(o=>o.presets).flatMap(p=>p.filters).filter(c=>c.exclude).length,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Full upstream English parser, preset mode choice and stat filters for map/chart/flask/Heist corpus; unchanged item-filter/query AST functions compare search identity, map/Heist fields and stat groups. Misc/type/trade policy outside scope. Synthetic fixtures plus actual copied Small Life Flask.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
