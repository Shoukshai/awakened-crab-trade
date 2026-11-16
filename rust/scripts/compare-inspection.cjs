#!/usr/bin/env node
// Offline full upstream parser -> map preparation and local item information.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts');
 const prep=host.load('renderer/src/web/map-check/prepare-map-stats.ts');
 const common=host.load('renderer/src/web/map-check/common.ts');
 const defaults=JSON.parse(fs.readFileSync(path.join(host.root,'rust/src/data/map-check-defaults.json'))).config;
 const cases=[],catalog=[];
 function map(line,rarity='Rare'){return `Item Class: Maps\nRarity: ${rarity}\n${rarity==='Rare'?'Synthetic Fixture\n':''}Map (Tier 16)\n--------\nMap Area: Beach\n--------\nItem Level: 83\n--------\n{ Prefix Modifier "Fixture" (Tier: 1) }\n${line}\n`;}
 for(const stat of data.STATS_ITERATOR('AreaMods')){
  if(!stat.fromAreaMods)continue;
  for(const matcher of stat.matchers){
   catalog.push({matcher:matcher.string,tag:stat.fromAreaMods});
   for(const value of ['7','7(1-10)','-7(-10--1)'])for(const profile of [1,2,3]){
    const config=structuredClone(defaults);config.profile=profile;config.showNewStats=true;
    config.selectedStats=config.selectedStats.filter(s=>s.matcher!==matcher.string);
    config.selectedStats.push({matcher:matcher.string,decision:'dws'});
    cases.push({text:map((matcher.advanced??matcher.string).replaceAll('#',value)),config,label:matcher.string});
   }
  }
 }
 for(const rarity of ['Normal','Magic','Rare'])cases.push({text:map('Unknown future modifier 999',rarity),config:defaults,label:'unknown '+rarity});
 cases.push({text:fs.readFileSync(path.join(host.root,'rust/src/fixtures/synthetic-map.txt'),'utf8'),config:defaults,label:'native UI map fixture'});
 cases.push({text:fs.readFileSync(path.join(host.root,'rust/src/fixtures/driftwood-wand.txt'),'utf8'),config:defaults,label:'real copied wand'});
 for(const quality of [0,10,20,30])for(const level of [1,46,47,67,68,69,83,84,100])for(const corrupt of [false,true]){
  cases.push({text:`Item Class: Rings\nRarity: Unique\nLe Heup of All\nIron Ring\n--------\nQuality (Attribute Modifiers): +${quality}% (augmented)\n--------\nItem Level: ${level}\n--------\n${corrupt?'Corrupted\n':''}`,config:defaults,label:'dust'});
 }
 for(const factors of ['Shaper Item','Shaper Item\nElder Item','{ Corruption Implicit Modifier }\n+10(8-12)% to Fire Resistance\n--------\nCorrupted','{ Corruption Implicit Modifier }\n+10(8-12)% to Fire Resistance\n{ Corruption Implicit Modifier }\n+10(8-12)% to Cold Resistance\n--------\nCorrupted']){
  cases.push({text:`Item Class: Rings\nRarity: Unique\nLe Heup of All\nIron Ring\n--------\nQuality (Attribute Modifiers): +20% (augmented)\n--------\nItem Level: 83\n--------\n${factors}\n`,config:defaults,label:'dust factors'});
 }
 for(const aps of [1.1,1.5,2.01])cases.push({text:`Item Class: Wands\nRarity: Normal\nDriftwood Wand\n--------\nPhysical Damage: 5-9\nElemental Damage: 2-8, 4-12\nAttacks per Second: ${aps}\n--------\nItem Level: 1\n`,config:defaults,label:'mixed elemental DPS'});
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/inspection'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:64*1024*1024});assert.equal(native.status,0,native.stderr);
 const output=JSON.parse(native.stdout);assert.equal(output.reports.length,cases.length);
 assert.deepEqual(output.catalog.sort((a,b)=>a.matcher.localeCompare(b.matcher)),catalog.sort((a,b)=>a.matcher.localeCompare(b.matcher)));
 const sets=[];for(const a of '-dwgs')for(const b of '-dwgs')for(const c of '-dwgs')sets.push(a+b+c);
 const decisions=sets.flatMap(set=>[1,2,3].flatMap(profile=>[...'-dwgs'].map(value=>({set,profile,value,next:common.nextDecision(value),color:common.decisionHasColor(set,profile),created:common.decisionCreate(value,profile,set)}))));
 assert.deepEqual(output.decisions,decisions);
 let mismatch_count=0;const failures=[];
 for(let i=0;i<cases.length;i++){
  try{
   const input=cases[i],parsed=parser.parseClipboard(input.text);assert(parsed.isOk(),String(parsed.error));const item=parsed.value,r=output.reports[i];
   const mapLike=(item.info.craftable?.category==='Map'&&item.rarity!=='Unique')||['Heist Contract','Heist Blueprint','Invitation','Chart'].includes(item.info.craftable?.category)||item.info.refName==='Expedition Logbook';
   assert.equal(r.map_like,mapLike);
   const stats=mapLike?item.statsByType.filter(s=>s.type==='explicit').map(s=>{
    const p=prep.prepareMapStat(s),decision=input.config.selectedStats.find(s=>s.matcher===p.matcher)?.decision[input.config.profile-1]??'-';
    return {matcher:p.matcher,roll:p.roll??null,text:p.matcher.replace(/(?<![#])[+-]?[#]/gm,String(p.roll??'#')),decision,is_new:input.config.showNewStats&&decision==='-'};
   }):[];
   assert.deepEqual(r.stats,stats);
   assert.deepEqual(r.unknown,mapLike?item.unknownModifiers.filter(s=>s.type==='explicit').map(s=>s.text):[]);
   assert.equal(r.dust,item.dustEquivalent??null);
   const dps=item.weaponAS?{physical:Math.round(item.weaponAS*(item.weaponPHYSICAL??0)),elemental:Math.round(item.weaponAS*(item.weaponELEMENTAL??0))}:null;
   if(dps)dps.total=dps.physical+dps.elemental;
   assert.deepEqual(r.dps,dps);
  }catch(error){mismatch_count++;if(failures.length<5)failures.push({case:i,label:cases[i].label,error:error.message,input:cases[i].text});}
 }
 const sources=['renderer/src/parser/Parser.ts','renderer/src/web/map-check/common.ts','renderer/src/web/map-check/prepare-map-stats.ts','renderer/src/web/map-check/settings-maps.vue','renderer/src/web/item-check/ItemInfo.vue'];
 const report={cases:cases.length,catalog_matchers:catalog.length,decision_cases:decisions.length,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Full upstream English parser, map stat preparation, three-profile decision helpers, area catalog, raw DPS and disenchantment dust; synthetic corpus plus actual copied wand. Excludes map images and graphical/browser interaction.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
