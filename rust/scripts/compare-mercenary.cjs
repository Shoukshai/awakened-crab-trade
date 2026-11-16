#!/usr/bin/env node
// Execute unchanged upstream Mercenary parser, preset and Trade group functions.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs');
async function main(){
 const parser=await host.init(),source=host.load('renderer/src/web/price-check/filters/create-presets.ts'),trade=host.load('renderer/src/web/price-check/trade/pathofexile-trade.ts');
 const items=fs.readFileSync(path.join(host.root,'renderer/public/data/en/items.ndjson'),'utf8').trim().split('\n').map(JSON.parse);
 const stats=fs.readFileSync(path.join(host.root,'renderer/public/data/en/stats.ndjson'),'utf8').trim().split('\n').flatMap(l=>{const s=JSON.parse(l);return s.stats??[s];});
 const builds=items.filter(i=>i.namespace==='MERCENARY_BUILD');
 const gem=name=>{const s=stats.find(s=>s.ref===name&&s.mercenary);assert(s,name);return s;};
 const print=s=>s.matchers[0].advanced??s.matchers[0].string;
 const cases=[];
 const text=(build,groups,level=83)=>`Item Class: Mercenary Warrants\nRarity: Normal\nMercenary Warrant\n--------\nMercenary Level: ${level}\nBuild: ${build.name}\n--------\n${groups.map(g=>g.map(s=>typeof s==='string'?s:print(s)).join('\n')).join('\n--------\n')}`;
 for(const build of builds){
  const data=typeof build.mercenaryBuild==='string'?builds.find(b=>b.refName===build.mercenaryBuild).mercenaryBuild:build.mercenaryBuild;
  for(const skill of data.skills){
   const s=gem(skill.name);cases.push({text:text(build,[[s]]),label:`bare ${build.name} ${s.ref}`,edits:[]});
   const families=(s.mercenary.supports??[]).map(name=>{const stat=gem(name);return stat.modFamily?.map(gem)??[stat];});
   if(!families.length)continue;
   for(const family of families)for(const support of family){
    cases.push({text:text(build,[[s,support]]),label:`support ${build.name} ${s.ref} ${support.ref}`,edits:[]});
    cases.push({text:text(build,[[s,support]]),label:`required ${build.name} ${s.ref} ${support.ref}`,edits:[{node:0,enabled:true},{node:0,child:0,enabled:true}]});
   }
   if(families.length>=5){
    const supports=families.slice(0,5).map((f,i)=>f[f[0].mercenary.syntheticFamily?f.length-1:Math.min(i%4,f.length-1)]);
    const t=text(build,[[s,...supports]]);
    // Stable child indexes follow sorted supports, with the six-link property first.
    for(const mode of ['initial','sixlink','tier3','all-required','optional','mixed','not-only','optional-disabled-skill']){
     if(mode==='not-only'&&families.length===5)continue;
     const edits=[];if(mode!=='initial'&&mode!=='optional-disabled-skill')edits.push({node:0,enabled:true});
     if(mode==='sixlink')edits.push({node:0,child:0,enabled:true,min:0});
     if(mode==='tier3')edits.push({node:0,child:0,enabled:true,min:5});
     if(mode==='all-required')for(let c=1;c<=5;c++)edits.push({node:0,child:c,enabled:true});
     if(['optional','mixed','optional-disabled-skill'].includes(mode))for(let c=1;c<=3;c++)edits.push({node:0,child:c,enabled:true,option:mode==='mixed'&&c===1?1:0});
     if(mode==='not-only')edits.push({node:0,child:6,enabled:true});
     cases.push({text:t,label:`five ${build.name} ${s.ref} ${mode}`,edits});
    }
    for(const min of [-1,1,2,3,4,6,2.5])cases.push({text:t,label:`count ${build.name} ${s.ref} ${min}`,edits:[{node:0,enabled:true},{node:0,child:0,enabled:true,min}]});
    cases.push({text:t,label:`no minimum ${build.name} ${s.ref}`,edits:[{node:0,enabled:true},{node:0,child:0,enabled:true,clear_min:true}]});
    cases.push({text:t,label:`one optional ${build.name} ${s.ref}`,edits:[{node:0,enabled:true},{node:0,child:1,enabled:true,option:0}]});
   }
  }
 }
 for(const level of [1,83,100])for(const ending of ['','Unknown support']){
  const raw=fs.readFileSync(path.join(host.root,'rust/src/fixtures/synthetic-mercenary.txt'),'utf8').replace('Mercenary Level: 83',`Mercenary Level: ${level}`).replace('\n--------\nTempest Shield',`${ending?'\n'+ending:''}\n--------\nTempest Shield`);
  cases.push({text:raw,label:`fixture level ${level} ${ending}`,edits:[]});
 }
 // Keep the corpus bounded while retaining all builds, all reachable skills and support families.
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/mercenary'));
 const compact=f=>({reference:f.statRef,text:f.text,ids:f.tradeId,tag:f.tag,disabled:f.disabled,hidden:f.hidden??null,option:f.option?.value??null,tier:f.mercenary?.tier??null,max_tier:f.mercenary?.maxTier??null,icon:f.mercenary?.icon??null,exclude:!!f.not,value:f.roll?.value??null,min:f.roll?.min??null,families:f.tradeId[0]==='item.mercenary_6link'?f.sources.map(s=>s.modifier.stats.map(s=>s.stat.ref)):[]});
 const nodes=s=>s.map(f=>f.group?{group:f.group,meta:compact(f.meta),stats:f.stats.map(compact)}:compact(f));
 const clean=q=>{q=JSON.parse(JSON.stringify(q));q.query.stats=q.query.stats.filter(g=>!g.disabled).map(({disabled,...g})=>({...g,filters:g.filters.filter(f=>!f.disabled).map(({disabled,...f})=>f)}));return q;};
 let mismatch_count=0;const failures=[];
 for(let offset=0;offset<cases.length;offset+=64){
 const chunk=cases.slice(offset,offset+64);
 const native=cp.spawnSync(binary,{input:JSON.stringify(chunk),encoding:'utf8',maxBuffer:32*1024*1024});assert.equal(native.status,0,native.stderr);const output=JSON.parse(native.stdout);assert.equal(output.length,chunk.length);
 for(let local=0;local<chunk.length;local++){const i=offset+local;try{
  const parsed=parser.parseClipboard(cases[i].text);assert(parsed.isOk(),String(parsed.error));
  const e=source.createPresets(parsed.value,{searchStatRange:10,league:'Covens',merchantOnly:false,currency:null,collapseListings:'api',activateStockFilter:false,useEn:true}).presets[0];
  const actual=output[local];assert.deepEqual(actual.initial.nodes,nodes(e.stats));
  assert.equal(actual.parsed.build,parsed.value.mercenaryBuild.refName);assert.equal(actual.parsed.level,parsed.value.itemLevel);
  assert.deepEqual(actual.parsed.skills,parsed.value.mercenarySkills.map(g=>g.map(s=>s.stat.ref)));
  assert.deepEqual(actual.parsed.unknown,parsed.value.unknownModifiers.map(m=>({text:m.text,kind:m.type})));
  for(const edit of cases[i].edits){const node=e.stats[edit.node],f=node.group?(edit.child==null?node.meta:node.stats[edit.child]):node;assert(f,JSON.stringify(edit));if(edit.enabled!=null)f.disabled=!edit.enabled;if(edit.option!=null)f.option.value=edit.option;if(edit.clear_min)f.roll.min=undefined;else if(edit.min!=null)f.roll.min=edit.min;}
  assert.deepEqual(actual.edited.nodes,nodes(e.stats));assert.deepEqual(actual.plan,JSON.parse(JSON.stringify(e.filters)));
  assert.deepEqual(clean(actual.query),clean(trade.createTradeRequest(e.filters,e.stats)));
 }catch(e){mismatch_count++;if(failures.length<6)failures.push({case:i,label:cases[i].label,error:e.message,text:cases[i].text,edits:cases[i].edits,native:output[local]});}
 }}
 const sources=['renderer/src/parser/Parser.ts','renderer/src/web/price-check/filters/pseudo/mercenary.ts','renderer/src/web/price-check/trade/pathofexile-trade.ts'];
 const report={cases:cases.length,builds:builds.length,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Full unchanged English parser, Mercenary model and complete effective Trade body; reachable build skills, supports, tier families, five-support groups and explicit selection scenarios. Synthetic only.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(fs.readFileSync(path.join(host.root,s))).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
