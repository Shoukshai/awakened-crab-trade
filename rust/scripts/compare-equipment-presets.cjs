#!/usr/bin/env node
// Full unchanged upstream parser/presets/item-policy/Trade body; offline only.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts');
 const source=host.load('renderer/src/web/price-check/filters/create-presets.ts'),trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts');
 const cases=[];
 const mod=(text,type='Prefix Modifier',name='Fixture',tier=1,incr=0)=>type==='Enchantment Modifier'?text+' (enchant)':`{ ${type} "${name}" (Tier: ${tier})${incr?' — '+incr+'% Increased':''} }\n${text}`;
 function text(base,rarity,props,mods,ending='',name='Synthetic Fixture'){if(ending==='Synthesised Item')base='Synthesised '+base;if(ending==='Foulborn'){name='Foulborn '+name;ending='';}if(ending==='Vestigial'){base='Vestigial '+base;ending='';}return `Item Class: Synthetic Fixtures\nRarity: ${rarity}\n${['Rare','Unique'].includes(rarity)?name+'\n':''}${base}\n--------\n${props?props+'\n--------\n':''}Item Level: 83\n--------\n${mods}${ending?'\n--------\n'+ending:''}`;}
 const add=(text,label,range=10,enable_empty=false)=>cases.push({text,label,range,enable_empty});
 const armour='Armour: 150\nEvasion Rating: 100\nEnergy Shield: 20';
 for(const base of ['Gold Ring','Amber Amulet','Leather Belt','Driftwood Wand','Plate Vest','Vaal Regalia','Cobalt Jewel','Large Cluster Jewel'])for(const rarity of ['Normal','Magic','Rare'])for(const range of [0,2,10,50])for(const flags of ['','Corrupted','Mirrored','Split','Shaper Item','Synthesised Item']){
  const props=(base==='Driftwood Wand'?'Physical Damage: 10-20\nCritical Strike Chance: 5.00%\nAttacks per Second: 1.50':base==='Plate Vest'?'Armour: 150':base==='Vaal Regalia'?'Energy Shield: 200':'');
  const mods=[mod('+73(70-79) to maximum Life'),mod('+25(22-27) to Strength','Suffix Modifier'),mod('+32(30-35)% to Fire Resistance','Suffix Modifier')].join('\n');
  add(text(base,rarity,props,mods,flags),'equipment '+base+' '+rarity+' '+flags,range);
 }
 for(const base of ['Gold Ring','Driftwood Wand','Plate Vest'])for(const quality of [0,19,20,30])for(const fractured of [false,true])for(const crafted of [false,true]){
  const props=(base==='Driftwood Wand'?'Physical Damage: 10-20\nAttacks per Second: 1.50\n':base==='Plate Vest'?'Armour: 150\n':'')+(base==='Gold Ring'?'Quality (Attribute Modifiers): +'+quality+'% (augmented)':'Quality: +'+quality+'%');
  add(text(base,'Rare',props,[mod('+73(70-79) to maximum Life',fractured?'Fractured Prefix Modifier':'Prefix Modifier'),mod('+25(22-27) to Strength',crafted?'Master Crafted Suffix Modifier':'Suffix Modifier')].join('\n')),'crafting value '+base+' '+quality+' '+fractured+' '+crafted);
 }
 const entries=fs.readFileSync(path.join(host.root,'renderer/public/data/en/items.ndjson'),'utf8').trim().split('\n').map(JSON.parse);
 const uniqueNames=['Lifesprig','Le Heup of All',"Kaom's Heart",'Tabula Rasa','Goldrim','Split Personality','The Adorned','Ventor\'s Gamble'];
 for(const name of uniqueNames){const u=entries.find(e=>e.namespace==='UNIQUE'&&e.refName===name);if(!u)throw Error(name);const base=entries.find(e=>e.namespace==='ITEM'&&e.refName===u.unique.base);assert(base,name);
  for(const flags of ['','Corrupted','Mirrored','Foil Unique','Foulborn','Vestigial'])for(const quality of [0,20,30])for(const variable of [false,true]){
   const props=(base.craftable?.category==='Wand'?'Physical Damage: 5-9\nCritical Strike Chance: 8.30%\nAttacks per Second: 1.50\n':base.armour?armour+'\n':'')+(['Ring','Amulet','Belt'].includes(base.craftable?.category)?'Quality (Attribute Modifiers): +'+quality+'% (augmented)':'Quality: +'+quality+'%');
   const mods=[mod(variable?'+73(70-79) to maximum Life':'+73 to maximum Life','Modifier'),mod('8(8-12)% increased Spell Damage','Implicit Modifier'),mod('+25(22-27) to Strength','Modifier')].join('\n');
   add(text(base.name,'Unique',props,mods,flags,u.name),'unique '+name+' '+flags+' '+variable);
  }
 }
 for(const category of ['Tincture','Sanctum Relic','Charm','Idol','Sentinel','Memory Line']){
  const base=entries.find(e=>e.namespace==='ITEM'&&e.craftable?.category===category);if(!base)continue;
  for(const rarity of ['Normal','Magic','Rare'])for(const tier of [1,3])for(const value of [10,25,30])add(text(base.name,rarity,'',mod(value+'(10-30)% increased effect','Prefix Modifier','Fixture',tier)),'special exact '+category+' '+rarity+' '+value);
 }
 for(const base of ['Plate Vest','Driftwood Wand','Gold Ring'])for(const strands of [0,59,60,70,100])add(text(base,'Rare',(base==='Plate Vest'?'Armour: 150\n':base==='Driftwood Wand'?'Physical Damage: 5-9\nAttacks per Second: 1.50\n':'')+'Memory Strands: '+strands,mod('+73(70-79) to maximum Life')),'memory '+base+' '+strands);
 for(const veiled of ['Veiled Prefix','Veiled Suffix'])add(text('Gold Ring','Rare','',mod(veiled,veiled==='Veiled Prefix'?'Prefix Modifier':'Suffix Modifier','Veiled')),'veiled '+veiled);
 // Mechanics have different display tags while retaining their Trade IDs.
 const strings=host.load('renderer/public/data/en/client_strings.js').default;
 for(const key of ['SHAPER_MODS','ELDER_MODS','VEILED_MODS','ESSENCE_MODS','DELVE_MODS','INCURSION_MODS'])for(const name of strings[key]){
  add(text('Gold Ring','Rare','',mod('+73(70-79) to maximum Life','Prefix Modifier',name)), 'mechanic '+key+' '+name);
 }
 for(const type of ['Corruption Implicit Modifier','Vestigial Implicit Modifier','Foulborn Unique Modifier'])add(text('Iron Ring','Unique','',mod('+25(20-30) to maximum Life',type),'Corrupted','Le Heup of All'),'special '+type);
 for(const passive of [2,3,4,5,6,8,9,10,11,12])for(const rarity of ['Magic','Rare'])for(const range of [0,2,10,50]){
  add(text('Large Cluster Jewel',rarity,'',[mod(`Adds ${passive} Passive Skills`,'Implicit Modifier'),mod('2 Added Passive Skills are Jewel Sockets','Implicit Modifier'),mod('Added Small Passive Skills grant: 12% increased Attack Damage','Enchantment Modifier')].join('\n')),'cluster '+passive+' '+rarity,range);
 }
 // Empty/crafted prefix and suffix choices are tested both inactive and active.
 for(const prefixCount of [2,3])for(const crafted of [false,true])for(const enable of [false,true]){
  const lines=[];for(let i=0;i<5;i++)lines.push(mod(i%2?'+25(22-27) to Strength':'+73(70-79) to maximum Life',i<prefixCount?'Prefix Modifier':'Suffix Modifier'));
  if(crafted)lines.push(mod('+15(13-16)% to Cold and Chaos Resistances','Master Crafted '+(prefixCount===2?'Prefix':'Suffix')+' Modifier'));
  add(text('Gold Ring','Rare','',lines.join('\n')),'empty '+prefixCount+' '+crafted+' '+enable,10,enable);
 }
 const stats=fs.readFileSync(path.join(host.root,'renderer/public/data/en/stats.ndjson'),'utf8').trim().split('\n').flatMap(l=>{const r=JSON.parse(l);return r.stats??[r];});
 for(const oils of ['13','12','1']){const stat=stats.find(s=>s.anointments?.some(a=>a.oils.split(',').includes(oils))&&s.matchers.some(m=>!m.string.includes('#'))&&s.trade.ids.enchant);if(!stat)continue;
  for(const ending of ['','Corrupted','Mirrored'])for(const talisman of [false,true])add(text('Amber Amulet','Rare',talisman?'Talisman Tier: 2':'',stat.matchers.find(m=>!m.string.includes('#')).string+' (enchant)',ending),'anointment '+oils+' '+ending+' '+talisman);
 }
 for(const name of ['Glorious Vanity','Lethal Pride','Brutal Restraint','Elegant Hubris','Militant Faith']){
  const u=entries.find(e=>e.namespace==='UNIQUE'&&e.refName===name);assert(u);
  const family=stats.find(s=>s.modFamily&&s.trade.ids.explicit&&s.matchers.some(m=>m.string.includes('#'))&&s.ref.includes(({ 'Glorious Vanity':'Bathed in the blood','Lethal Pride':'Commanded leadership','Brutal Restraint':'Denoted service','Elegant Hubris':'Commissioned','Militant Faith':'Carved to glorify'}[name])));
  assert(family,name);const matcher=family.matchers.find(m=>m.string.includes('#'));
  add(text('Timeless Jewel','Unique','',mod((matcher.advanced??matcher.string).replace('#','1000(100-8000)'),'Modifier'),'',u.name),'timeless '+name);
 }
 for(const fixture of ['driftwood-wand.txt','synthetic-calculated-ring.txt','small-life-flask.txt','synthetic-crafting-ring.txt','synthetic-timeless-jewel.txt'])add(fs.readFileSync(path.join(host.root,'rust/src/fixtures',fixture),'utf8'),'fixture '+fixture);
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/equipment-presets'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:128*1024*1024});assert.equal(native.status,0,native.stderr);const output=JSON.parse(native.stdout);assert.equal(output.length,cases.length);
 const mode=id=>({'filters.preset_exact':'exact','filters.preset_base_item':'base','filters.preset_pseudo':'calculated'}[id]??id);
 const compact=(f,group=null)=>({reference:f.statRef,ids:f.tradeId,value:f.roll?.value??null,min:f.roll?.min??null,max:f.roll?.max??null,disabled:!!f.disabled,hidden:f.hidden??null,not:!!f.not,tag:f.tag,group,oils:f.oils??null});
 const compactNative=c=>({reference:c.modifier.reference,ids:c.modifier.ids,value:c.modifier.option?null:c.display_value,min:c.initial_min,max:c.initial_max,disabled:c.disabled,hidden:c.hidden,not:c.exclude,tag:c.tag,group:c.group,oils:c.oils});
 const cleanQuery=q=>{q=JSON.parse(JSON.stringify(q));q.query.stats=q.query.stats.filter(g=>!g.disabled).map(({disabled,...g})=>({...g,filters:g.filters.filter(f=>!f.disabled).map(({disabled,...f})=>f)}));return q;};
 let mismatch_count=0;const failures=[];
 for(let i=0;i<cases.length;i++)try{
  const parsed=parser.parseClipboard(cases[i].text);assert(parsed.isOk(),String(parsed.error));
  const expected=source.createPresets(parsed.value,{searchStatRange:cases[i].range,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,useEn:true});
  if(cases[i].label.startsWith('timeless '))assert(expected.presets.some(p=>p.stats.some(f=>f.group==='one')),'Timeless fixture must produce a group');
  const actual=output[i];assert.equal(actual.modes[0],mode(expected.active));assert.deepEqual(actual.modes,expected.presets.map(p=>mode(p.id)));
  for(const p of actual.presets){
   const e=expected.presets.find(e=>mode(e.id)===p.mode);
   if(cases[i].enable_empty)for(const f of e.stats)if(f.tradeId?.[0]==='item.has_empty_modifier')f.disabled=false;
   const flat=e.stats.flatMap(f=>f.group?f.stats.map(c=>compact(c,'timeless_keystone')):[compact(f)]);
   assert.deepEqual(p.filters.map(compactNative),flat);
   assert.deepEqual(p.plan,JSON.parse(JSON.stringify(e.filters)));
   const q=trade.createTradeRequest(e.filters,e.stats.filter(f=>f.group?!f.meta.disabled:!f.disabled));assert.deepEqual(cleanQuery(p.query),cleanQuery(q));
  }
 }catch(e){mismatch_count++;if(failures.length<16)failures.push({case:i,label:cases[i].label,error:e.message,text:cases[i].text,native:output[i]});}
 const sources=['renderer/src/parser/advanced-mod-desc.ts','renderer/src/web/price-check/filters/create-presets.ts','renderer/src/web/price-check/filters/create-stat-filters.ts','renderer/src/web/price-check/filters/pseudo/anointments.ts','renderer/src/web/price-check/filters/pseudo/timeless-jewel.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts'];
 const report={cases:cases.length,timeless_choices:output.flatMap(o=>o.presets).flatMap(p=>p.filters).filter(f=>f.group).length,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Full English parser, equipment/crafting/unique/cluster/anointment/Timeless preset and item-filter models, effective complete Trade requests; disabled query nodes removed on both sides. Synthetic corpus and actual copied wand/flask. Other special item presets excluded.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
