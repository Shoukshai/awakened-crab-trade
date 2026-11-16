#!/usr/bin/env node
// Offline full upstream parser -> complete item-filter policy and query payload.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init();
 const policy=host.load('renderer/src/web/price-check/filters/create-item-filters.ts');
 const trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts');
 const entries=fs.readFileSync(path.join(host.root,'renderer/public/data/en/items.ndjson'),'utf8').trim().split('\n').map(JSON.parse);
 const cases=[];
 function add(text,label,options={}){cases.push({text,label,options:{league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,exact:false,useEn:true,...options}});}
 function text(base,rarity='Rare',props='',ending='',mods='',unique){
  return `Item Class: Synthetic Fixtures\nRarity: ${rarity}\n${unique?unique+'\n':rarity==='Rare'?'Synthetic Fixture\n':''}${base}\n--------\n${props?props+'\n--------\n':''}Item Level: 83\n--------\n${mods?mods+'\n--------\n':''}${ending?ending+'\n':''}`;
 }
 function properties(category,q){
  if(['Amulet','Belt','Ring'].includes(category))return q?`Quality (Attribute Modifiers): +${q}% (augmented)`:'';
  if(['Flask','Tincture'].includes(category))return `Quality: +${q}% (augmented)\nConsumes 30 of 60 Charges on use\nCurrently has 60 Charges`;
  if(['Body Armour','Helmet','Gloves','Boots','Shield'].includes(category))return `Quality: +${q}% (augmented)\nArmour: 100`;
  if(['Jewel','Abyss Jewel','Cluster Jewel','Idol','Graft','Sanctum Relic','Charm','Heist Tool','Heist Brooch','Heist Cloak','Heist Gear','Trinket','Memory Line','Sentinel','Voidstone'].includes(category))return '';
  return `Quality: +${q}% (augmented)\nPhysical Damage: 10-20\nCritical Strike Chance: 5.00%\nAttacks per Second: 1.50`;
 }
 const cats=['Wand','Body Armour','Helmet','Boots','Shield','Amulet','Belt','Ring','Flask','Tincture','Jewel','Abyss Jewel','Cluster Jewel','Idol','Graft','Sanctum Relic','Charm','Heist Tool','Heist Brooch','Heist Cloak','Heist Gear','Trinket','Memory Line','Sentinel'];
 for(const category of cats){
  const base=entries.find(v=>v.namespace==='ITEM'&&v.craftable?.category===category);if(!base)continue;
  for(const rarity of ['Normal','Magic','Rare'])for(const level of [1,49,50,67,68,74,75,81,82,83,84,86,100])for(const exact of [false,true]){
   const q=[0,20,30][level%3];
   const ending=['','Corrupted','Mirrored','Split','Unmodifiable'][level%5];
   const props=properties(category,q)+(category==='Sentinel'?'\nCharge: 12':'');
   const t=text(base.name,rarity,props,ending).replace('Item Level: 83','Item Level: '+level);
   add(t,`${category} ${rarity} ${level} ${ending}`,{exact,league:level%2?'Covens':'Standard',merchantOnly:level%3===0,collapseListings:level%4===0?'app':'api',currency:level%6===0?'chaos':null});
  }
  for(const ending of ['Shaper Item','Shaper Item\nElder Item','Synthesised Item']){
   const b=ending==='Synthesised Item'?'Synthesised '+base.name:base.name;
   for(const exact of [false,true])add(text(b,'Rare',properties(category,20),ending),category+' factors '+ending,{exact});
  }
 }
 for(const name of ['Le Heup of All','Lifesprig',"Kaom's Heart",'Agnerod East']){
  const unique=entries.find(v=>v.namespace==='UNIQUE'&&v.refName===name);assert(unique,name);
  const base=entries.find(v=>v.namespace==='ITEM'&&v.refName===unique.unique.base);assert(base);
  for(const level of [1,46,68,74,75,78,80,81,82,84,100])for(const variant of ['ordinary','foulborn','vestigial','foil','corrupted']){
   const props=properties(base.craftable.category,20);
   const t=text((variant==='vestigial'?'Vestigial ':'')+base.name,'Unique',props,variant==='foil'?'Foil Unique':variant==='corrupted'?'Corrupted':'','',(variant==='foulborn'?'Foulborn ':'')+unique.name).replace('Item Level: 83','Item Level: '+level);
   add(t,`${name} ${level} ${variant}`);
  }
  add(text(base.name,'Unique',properties(base.craftable.category,0),'Unidentified'),'unidentified '+name,{exact:true});
 }
 const gems=['Fireball','Empower Support','Enlighten Support','Portal',...entries.filter(v=>v.namespace==='GEM'&&v.gem?.transfigured).slice(0,2).map(v=>v.name)];
 for(const name of gems)for(const level of [1,19,20,21])for(const q of [0,15,16,19,20,23])for(const corrupt of [false,true]){
  add(text(name,'Gem',`Fire, Spell\nLevel: ${level}${q?'\nQuality: +'+q+'%':''}`,corrupt?'Corrupted':''),`gem ${name} ${level}/${q} ${corrupt}`,{exact:true});
 }
 add(text('Fireball','Gem','Fire, Spell\nLevel: 20\nQuality: +20%','Vaal Fireball'),'vaal gem',{exact:true});
 for(const name of ['Scroll of Wisdom','Chaos Orb','Chronicle of Atzoatl','Mirrored Tablet','Forbidden Tome','Scrying Orb','Charged Compass']){
  for(const level of [67,68,72,73,74,75,77,78,79,80,82,83]){
   const props=name==='Scrying Orb'?'Map Area: Beach':name==='Charged Compass'?'':'Area Level: '+level;
   add(text(name,'Currency',props,'','').replace('Item Level: 83','Item Level: '+level),'currency '+name+' '+level,{merchantOnly:level%2===0});
  }
 }
 for(const stock of [1,20,1234])for(const activateStockFilter of [false,true])add(`Item Class: Stackable Currency\nRarity: Currency\nChaos Orb\n--------\nStack Size: ${stock.toLocaleString('en')}/5,000\n--------`, 'currency stock '+stock,{activateStockFilter,merchantOnly:true});
 const beast=entries.find(v=>v.namespace==='CAPTURED_BEAST');add(text(beast.name,'Rare','','Right-click to add this to your bestiary.'),'captured beast');
 add(text('Fixture Brain','Normal',"","Combine this with four other different samples in Tane's Laboratory."),'metamorph');
 for(const exact of [false,true])add(text('Gold Ring','Rare','', '', '{ Prefix Modifier "Veiled" }\nVeiled Prefix'),'veiled ring',{exact});
 for(const category of ['Wand','Ring','Flask','Cluster Jewel']){
  const base=entries.find(v=>v.namespace==='ITEM'&&v.craftable?.category===category);
  for(const exact of [false,true])add(text(base.name,'Rare',properties(category,20),'','{ Fractured Prefix Modifier "Fixture" (Tier: 1) }\n+30(20-40)% to Fire Resistance'),'fractured '+category,{exact});
 }
 for(const fixture of ['driftwood-wand.txt','small-life-flask.txt','scroll-of-wisdom.txt','synthetic-map.txt','synthetic-contract.txt','synthetic-blueprint.txt'])add(fs.readFileSync(path.join(host.root,'rust/src/fixtures',fixture),'utf8'),'fixture '+fixture,{exact:true});
 const editedBase=text('Gold Ring','Rare','Quality (Attribute Modifiers): +20% (augmented)');
 for(const overrides of [
  {searchRelaxed:{disabled:true}},
  {trade:{offline:true,merchantOnly:true,listed:'1day',currency:'divine'}},
  {trade:{merchantOnly:true,collapseListings:'app'}},
  {corrupted:{value:true,exact:true}},
  {rarity:{value:'rare',disabled:false}},
  {itemLevel:{value:79,max:80,disabled:false}},
  {whiteSockets:{value:1,disabled:false}},
  {mirrored:{disabled:false},split:{disabled:false},fractured:{value:true}},
  {unidentified:{value:true,disabled:false}},
  {foil:{disabled:false}},
 ]){add(editedBase,'edited policy',{exact:false});cases.at(-1).overrides=overrides;}
 const statEntries=fs.readFileSync(path.join(host.root,'renderer/public/data/en/stats.ndjson'),'utf8').trim().split('\n').flatMap(l=>{const r=JSON.parse(l);return r.stats??[r];});
 const imbued=statEntries.find(s=>s.trade.ids.imbued&&s.matchers.some(m=>!m.string.includes('#')));
 if(imbued)add(text('Fireball','Gem','Fire, Spell\nLevel: 20\nQuality: +20%',imbued.matchers.find(m=>!m.string.includes('#')).string+'\n--------\nCorrupted'),'imbued gem',{exact:true});
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/item-filters'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:128*1024*1024});assert.equal(native.status,0,native.stderr);
 const output=JSON.parse(native.stdout);assert.equal(output.length,cases.length);
 let mismatch_count=0,rejected_count=0;const failures=[];
 const clean=v=>JSON.parse(JSON.stringify(v));
 for(let i=0;i<cases.length;i++){
  try{
   const parsed=parser.parseClipboard(cases[i].text);
   const deprecated=(cases[i].label.startsWith('currency Charged Compass') || cases[i].label==='metamorph') && !entries.some(v=>v.refName===(cases[i].label==='metamorph'?'Metamorph Brain':'Charged Compass'));
   if(deprecated){assert(parsed.isErr());assert(output[i].error);rejected_count++;continue;}
   assert(parsed.isOk(),String(parsed.error));
   const expected=clean(policy.createFilters(parsed.value,cases[i].options));
   function merge(to,from){for(const [key,value] of Object.entries(from)){if(value&&typeof value==='object'&&!Array.isArray(value)){to[key]??={};merge(to[key],value);}else to[key]=value;}}
   if(cases[i].overrides)merge(expected,cases[i].overrides);
   assert.equal(output[i].error,undefined,output[i].error);
   assert.deepEqual(output[i].plan,expected);
   const query=clean(trade.createTradeRequest(expected,[]));
   assert.deepEqual(output[i].query,query);
  }catch(error){mismatch_count++;if(failures.length<5)failures.push({case:i,label:cases[i].label,error:error.message,input:cases[i],native:output[i]});}
 }
 const sources=['renderer/src/parser/Parser.ts','renderer/src/parser/meta.ts','renderer/src/web/price-check/filters/create-item-filters.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts'];
 const report={cases:cases.length,accepted_cases:cases.length-rejected_count,edited_cases:cases.filter(c=>c.overrides).length,imbued_cases:cases.filter(c=>c.label==='imbued gem').length,rejected_deprecated:rejected_count,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Full upstream English parser -> complete createFilters model -> complete Trade payload without modifier presets. Unchanged pure function/constant ASTs; no HTTP or desktop calls. Synthetic corpus plus copied wand/Small Life Flask/Wisdom.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(error=>{console.error(error);process.exitCode=1;});
