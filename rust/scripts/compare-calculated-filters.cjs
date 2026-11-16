#!/usr/bin/env node
// Full upstream parseClipboard -> equipment/pseudo functions, executed offline.
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts');
 const prop=host.load('renderer/src/web/price-check/filters/pseudo/item-property.ts');
 const pseudo=host.load('renderer/src/web/price-check/filters/pseudo/index.ts');
 const rules=JSON.parse(fs.readFileSync(path.join(host.root,'rust/src/data/pseudo-rules.json'))).rules;
 const stats=new Map();
 for(const rule of rules)for(const input of rule.stats){
  const record=data.STAT_BY_REF_V2(input.ref);
  const stat=(record.stats??[record]).find(s=>s.ref===input.ref&&s.trade.ids.explicit);
  if(stat)stats.set(input.ref,stat);
 }
 const cases=[];
 function text(base,rarity,lines,extra='',properties=''){
  const item=data.ITEM_BY_TRANSLATED('ITEM',base)?.[0];assert(item,base);
  const cls=item.craftable.category==='Ring'?'Rings':item.craftable.category==='Wand'?'Wands':item.craftable.category==='Body Armour'?'Body Armours':'One Hand Swords';
  const name=rarity==='Rare'?'Synthetic Fixture\n'+base:rarity==='Unique'?'Le Heup of All\nIron Ring':base;
  return `Item Class: ${cls}\nRarity: ${rarity}\n${name}\n--------\n${properties?properties+'\n--------\n':''}Item Level: 83\n--------\n${lines.join('\n')}\n${extra?'--------\n'+extra+'\n':''}`;
 }
 function mod(ref,value=20,kind='explicit'){
  const stat=stats.get(ref);assert(stat,ref);
  const matcher=stat.matchers.find(m=>!m.negate && m.value==null)??stat.matchers[0];
  let index=0;
  const line=(matcher.advanced??matcher.string).replace(/#/g,()=>{const v=value+(index++*4);return `${v}(${v-3}-${v+5})`;});
  return `{ ${kind==='crafted'?'Master Crafted Prefix Modifier':kind==='implicit'?'Implicit Modifier':kind==='fractured'?'Fractured Prefix Modifier "Fixture" (Tier: 1)':'Prefix Modifier "Fixture" (Tier: 2)'} }\n${line}`;
 }
 function add(t,range=10,label=''){cases.push({text:t,range,label});}
 const refs=[...stats.keys()];
 for(const ref of refs){
  add(text('Gold Ring','Rare',[mod(ref)]),10,'single '+ref);
  add(text('Gold Ring','Rare',[mod(ref,30),mod(ref,12,'crafted')]),0,'duplicate '+ref);
  add(text('Gold Ring','Rare',[mod(ref,-20)]),10,'negative '+ref);
 }
 for(const rule of rules){
  const inputs=rule.stats.filter(s=>stats.has(s.ref));
  if(inputs.length)add(text('Gold Ring','Rare',inputs.map((s,i)=>mod(s.ref,10+i*7))),10,rule.pseudo);
 }
 // Deterministic mixed sources exercise replacement groups, ties and keep rules.
 let seed=0x12345678;const random=n=>{seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed%n;};
 for(let i=0;i<1200;i++){
  const rarity=i%5===0?'Unique':'Rare';
  const lines=Array.from({length:1+random(7)},(_,j)=>mod(refs[random(refs.length)],4+random(75),['explicit','crafted','implicit','fractured'][random(4)]));
  add(text(rarity==='Unique'?'Iron Ring':'Gold Ring',rarity,lines,i%3===0?'Corrupted':''),[0,10,25,100][i%4],'mixed '+i);
 }
 for(const quality of [0,10,20,30])for(const rarity of ['Normal','Magic','Rare']){
  for(let n=0;n<12;n++){
   const mods=rarity==='Normal'?[]:[`{ Prefix Modifier "Fixture" (Tier: 2) }\nAdds 4(2-6) to 8(6-10) Physical Damage`,`{ Prefix Modifier "Fixture" (Tier: 2) }\n${20+n}(10-40)% increased Physical Damage`,`{ Suffix Modifier (Tier: 2) }\n12(8-16)% increased Attack Speed`];
   add(text(n%2?'Rusted Sword':'Driftwood Wand',rarity,mods,'',`Quality: +${quality}% (augmented)\nPhysical Damage: ${10+n}-${25+n}\nElemental Damage: 2-8, 4-12\nCritical Strike Chance: 8.30%\nAttacks per Second: 1.50`),[0,10,25][n%3],'weapon');
   add(text(n%2?'Vaal Regalia':'Plate Vest',rarity,rarity==='Normal'?[]:[`{ Prefix Modifier "Fixture" (Tier: 2) }\n+12(8-16) to ${n%2?'maximum Energy Shield':'Armour'}`,`{ Prefix Modifier "Fixture" (Tier: 2) }\n30(20-40)% increased ${n%2?'Energy Shield':'Armour'}`],'',`Quality: +${quality}% (augmented)\n${n%2?'Energy Shield':'Armour'}: ${30+n*20}\nMemory Strands: 67`),[0,10,25][n%3],'armour');
  }
 }
 add(fs.readFileSync(path.join(host.root,'rust/src/fixtures/driftwood-wand.txt'),'utf8'),10,'real copied wand');
 add(fs.readFileSync(path.join(host.root,'rust/src/fixtures/synthetic-calculated-ring.txt'),'utf8'),10,'synthetic native UI fixture');
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/calculated-filters'));
 const native=cp.spawnSync(binary,{input:JSON.stringify(cases),encoding:'utf8',maxBuffer:128*1024*1024});assert.equal(native.status,0,native.stderr);
 const output=JSON.parse(native.stdout);assert.equal(output.length,cases.length);
 let mismatch_count=0;const failures=[];
 const compact=f=>({reference:f.statRef,ids:f.tradeId,value:f.roll?.value??null,min:f.roll?.min??null,max:f.roll?.max??null,disabled:!!f.disabled,hidden:f.hidden??null});
 const compactNative=c=>({reference:c.modifier.reference,ids:c.modifier.ids,value:c.display_value,min:c.initial_min,max:c.initial_max,disabled:c.disabled,hidden:c.hidden});
 for(let i=0;i<cases.length;i++){
  const input=cases[i];
  try{
   const parsed=parser.parseClipboard(input.text);assert(parsed.isOk(),String(parsed.error));const item=parsed.value;
   const ctx={item,statsByType:[...item.statsByType],filters:[],searchInRange:item.rarity==='Normal'?100:input.range};
   prop.filterEquipmentProps(ctx);prop.filterBasePercentile(ctx);prop.filterMemoryStrands(ctx,'hide_memory_strands');
   const equipment=ctx.filters.map(compact);ctx.filters=[];
   if(item.info.refName!=='Split Personality')pseudo.filterPseudo(ctx);
   const remaining=ctx.statsByType.map(m=>({reference:m.stat.ref,kind:m.type,sources:m.sources.length}));
   assert.deepEqual(output[i].equipment.map(compactNative),equipment);
   assert.deepEqual(output[i].pseudo.map(compactNative),ctx.filters.map(compact));
   assert.deepEqual(output[i].remaining,remaining);
  }catch(error){mismatch_count++;if(failures.length<8)failures.push({case:i,label:input.label,error:error.message,input});}
 }
 const sources=['renderer/src/parser/Parser.ts','renderer/src/parser/calc-q20.ts','renderer/src/web/price-check/filters/pseudo/index.ts','renderer/src/web/price-check/filters/pseudo/item-property.ts','renderer/src/web/price-check/filters/create-stat-filters.ts'];
 const report={cases:cases.length,mismatch_count,failures,rule_count:rules.length,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Full upstream item parser, equipment/pseudo calculation and initial bounds; synthetic English item corpus plus real copied wand; excludes complete presets/final UI rules',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');
 console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
