#!/usr/bin/env node
// Full unchanged parser + economy identity, prices, trends and related-item functions.
process.env.APT_FULL_FILTERS='1';
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process'),vm=require('node:vm'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const host=require('./upstream-core.cjs'),ts=require(path.join(host.root,'renderer/node_modules/typescript'));
const read=p=>fs.readFileSync(path.join(host.root,p),'utf8');
function script(file){const text=read(file);return file.endsWith('.vue')?text.match(/<script[^>]*>([\s\S]*?)<\/script>/)[1]:text;}
function functions(file,names){const src=ts.createSourceFile(file,script(file),ts.ScriptTarget.Latest,true);const found=[];function walk(n){if(ts.isFunctionDeclaration(n)&&names.includes(n.name?.text))found.push(n.getText(src));ts.forEachChild(n,walk);}walk(src);return found.join('\n');}
function evaluate(source,context={}){const js=ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.CommonJS}}).outputText;vm.runInNewContext(js,context);return context.result;}
async function main(){
 const parser=await host.init(),data=host.load('renderer/src/assets/data/index.ts');
 const details=host.load('renderer/src/web/price-check/trends/getDetailsId.ts');
 const pricesFile='renderer/src/web/background/Prices.ts',relatedFile='renderer/src/web/price-check/related-items/RelatedItems.vue',trendFile='renderer/src/web/price-check/trends/PriceTrend.vue';
 const priceFunctions=functions(pricesFile,['selectedLeagueToUrl','findPriceByQuery','autoCurrency','chaosToStable','denseInfoToDetailsId','splitJsonBlob']);
 const makePrices=evaluate(`result=(league,snapshot)=>{const leagues={selectedId:{value:league}},xchgRate={value:undefined};${priceFunctions}\nconst PRICES_DB=splitJsonBlob(JSON.stringify(snapshot));const divine=findPriceByQuery({ns:'ITEM',name:'Divine Orb',variant:undefined});if(divine&&divine.chaos>=30)xchgRate.value=divine.chaos;return {findPriceByQuery,autoCurrency,rate:xchgRate.value};};`);
 const trend=evaluate(functions(trendFile,['deltaFromGraph'])+'\nresult=deltaFromGraph;');
 const drop=JSON.parse(read('renderer/public/data/item-drop.json'));
 const related=evaluate(`result=(id,prices)=>{const ITEM_DROP=drop,ITEM_BY_REF=byRef,findPriceByQuery=prices.findPriceByQuery,autoCurrency=prices.autoCurrency;${functions(relatedFile,['findItemByQueryId','findPriceByQueryId','getItemPrices'])}\nreturn getItemPrices(id);};`,{drop,byRef:data.ITEM_BY_REF});
 const entries=read('renderer/public/data/en/items.ndjson').trim().split('\n').map(JSON.parse),stats=read('renderer/public/data/en/stats.ndjson').trim().split('\n').flatMap(l=>{const s=JSON.parse(l);return s.stats??[s];});
 const cases=[];const add=(text,label)=>cases.push({text,label});
 const text=(base,rarity='Normal',props='',mods='',name='Synthetic Fixture',ending='')=>`Item Class: Synthetic Fixtures\nRarity: ${rarity}\n${rarity==='Unique'||rarity==='Rare'?name+'\n':''}${base}\n--------\n${props}${props?'\n--------\n':''}Item Level: 83\n--------\n${mods}${ending?'\n--------\n'+ending:''}`;
 const mod=(s)=>`{ Modifier "Fixture" }\n${s}`;
 const render=(ref,value=10,kind='explicit')=>{const stat=stats.find(s=>s.ref===ref&&s.trade.ids[kind])??stats.find(s=>s.ref===ref.replace('during Flask effect','during Effect')&&s.trade.ids[kind]);assert(stat,ref);return (stat.matchers[0].advanced??stat.matchers[0].string).replaceAll('#',String(value));};
 const numeric=(name)=>['Bubonic Trail','Lightpoacher','Shroud of the Lightless','Tombfist'].includes(name)?mod(render('Has # Abyssal Sockets',1)):name==='Voices'?mod(render('Adds # Small Passive Skills which grant nothing',3)):'';
 const uniqueEntries=entries.filter(e=>e.namespace==='UNIQUE');const seen=new Set();
 for(const u of uniqueEntries){const base=entries.find(e=>e.namespace==='ITEM'&&e.refName===u.unique.base);assert(base,u.refName);const id=u.refName+' // '+base.refName;if(seen.has(id))continue;seen.add(id);add(text(base.name,'Unique','',numeric(u.refName),u.name),'unique '+id);}
 // Extract each unique branch's referenced stats from the real function AST.
 const identityFile='renderer/src/web/price-check/trends/getDetailsId.ts',ast=ts.createSourceFile(identityFile,read(identityFile),ts.ScriptTarget.Latest,true);
 const fn=ast.statements.find(n=>ts.isFunctionDeclaration(n)&&n.name.text==='getUniqueVariant');
 let branch=fn.body.statements.find(ts.isIfStatement);
 while(branch&&ts.isIfStatement(branch)){
  const names=[];function nameWalk(n){if(ts.isBinaryExpression(n)&&ts.isIdentifier(n.left)&&n.left.text==='uniqueName'&&ts.isStringLiteral(n.right))names.push(n.right.text);ts.forEachChild(n,nameWalk);}nameWalk(branch.expression);
  const refs=[];function refWalk(n){if(ts.isCallExpression(n)&&ts.isIdentifier(n.expression)&&n.expression.text==='hasStat')refs.push(n.arguments[1].text);ts.forEachChild(n,refWalk);}refWalk(branch.thenStatement);refs.splice(0,refs.length,...new Set(refs));
  for(const name of names){const u=uniqueEntries.find(u=>u.refName===name),base=entries.find(e=>e.namespace==='ITEM'&&e.refName===u.unique.base);assert(u&&base,name);
   for(let mask=0;mask<(1<<refs.length);mask++){
    const mods=refs.filter((_,i)=>mask&(1<<i)).map(r=>mod(render(r))).join('\n');add(text(base.name,'Unique','',mods||numeric(name),u.name),'variant '+name+' mask '+mask);
   }
   if(['Bubonic Trail','Lightpoacher','Shroud of the Lightless','Tombfist','Voices'].includes(name))for(const value of [1,2,3,5,7]){
    const ref=name==='Voices'?'Adds # Small Passive Skills which grant nothing':'Has # Abyssal Sockets';for(const links of [0,5,6])add(text(base.name,'Unique',links?'Sockets: '+Array(links).fill('W').join('-'):'',mod(render(ref,value)),u.name),'numeric variant '+name+' '+value+' '+links);
   }
  }
  branch=branch.elseStatement;
 }
 for(const name of ['Lifesprig','Le Heup of All','The First Crest'])for(const ending of ['Corrupted','Mirrored','Foil Unique','Foulborn','Vestigial']){
  const u=uniqueEntries.find(u=>u.refName===name),base=entries.find(e=>e.namespace==='ITEM'&&e.refName===u.unique.base);add(text(ending==='Vestigial'?'Vestigial '+base.name:base.name,'Unique','',numeric(name),ending==='Foulborn'?'Foulborn '+u.name:u.name,['Foulborn','Vestigial'].includes(ending)?'':ending),'trait '+name+' '+ending);
 }
 const cats=['Wand','Body Armour','Helmet','Boots','Gloves','Shield','Quiver','Amulet','Ring','Belt','Jewel','Flask','Tincture','Sanctum Relic','Idol'];
 for(const category of cats){const base=entries.find(e=>e.namespace==='ITEM'&&e.craftable?.category===category);assert(base,category);
  for(const rarity of ['Normal','Magic','Rare'])for(const level of [1,67,68,81,82,85,86,100])for(const influence of ['', 'Shaper Item','Elder Item','Hunter Item','Redeemer Item','Warlord Item','Crusader Item','Shaper Item\nElder Item'])add(text(base.name,rarity).replace('Item Level: 83','Item Level: '+level)+(influence?'\n--------\n'+influence:''),'base '+category+' '+rarity+' '+level+' '+influence);
 }
 for(const g of entries.filter(e=>e.namespace==='GEM'))add(text(g.name,'Gem','Spell\nLevel: 20\nQuality: +20%'),'gem '+g.refName);
 for(const name of ['Fireball','Vaal Fireball','Empower Support','Enlighten Support','Enhance Support','Portal','Brand Recall','Blood and Sand'])for(const level of [1,2,3,4,19,20,21])for(const quality of [0,15,16,20,23])for(const corrupted of [false,true])add(text(name,'Gem',`Spell\nLevel: ${level}\nQuality: +${quality}%`,'','',corrupted?'Corrupted':''),'gem details '+name+' '+level+' '+quality+' '+corrupted);
 for(const rarity of ['Normal','Magic','Rare'])for(const base of ['Map','Blighted Map','Blight-ravaged Map'])for(const tier of [0,1,16,17])add(text(base,rarity).replace('Item Class: Synthetic Fixtures','Item Class: Maps').replace(base+'\n--------',base+(tier?' (Tier '+tier+')':'')+'\n--------'),'map '+base+' '+rarity+' '+tier);
 if(entries.some(e=>e.namespace==='ITEM'&&e.refName==='Filled Coffin'))for(const stat of stats.filter(s=>s.trade.ids.necropolis).slice(0,80))for(const level of [1,69,70,79,80,83,84,100])add(text('Filled Coffin','Normal','',render(stat.ref,10,'necropolis')+' (implicit)').replace('Item Level: 83','Item Level: '+level),'coffin '+stat.ref+' '+level);
 for(const fixture of ['driftwood-wand.txt','scroll-of-wisdom.txt','small-life-flask.txt'])add(read('rust/src/fixtures/'+fixture),'real copied '+fixture);
 add(text('Divine Orb','Currency','Stack Size: 3/10'),'divine held stack');
 for(const stack of ['3/10','1,212/5,000','1\u202f212/5\u202f000'])add(text('Chaos Orb','Currency','Stack Size: '+stack),'stack separators '+stack);
 add(text('Driftwood Wand','Unique','','','Lifesprig','Unidentified'),'unidentified unique');

 for(const card of entries.filter(e=>e.namespace==='DIVINATION_CARD'))add(text(card.name,'Divination Card','Stack Size: 3/8'),'card '+card.refName);
 for(const beast of entries.filter(e=>e.namespace==='CAPTURED_BEAST'))add(text(beast.name,'Rare','','','Synthetic Beast','Right-click to add this to your bestiary.'),'beast '+beast.refName);
 const parsed=cases.map(c=>{const p=parser.parseClipboard(c.text);assert(p.isOk(),c.label+': '+p.error);return p.value;});
 const keys=parsed.map((item,i)=>{try{return details.getDetailsId(item);}catch(e){throw Error(cases[i].label+': '+e.message);}});
 const decode=id=>{const [ns,encoded]=id.split('::'),[name,variant]=encoded.split(' // ');return {ns,name,variant};};
 const relationIds=drop.flatMap(d=>d.query),relationKeys=relationIds.map(decode);
 const namespaces=[['ITEM','Currency'],['UNIQUE','UniqueAccessory'],['GEM','SkillGem'],['DIVINATION_CARD','DivinationCard'],['CAPTURED_BEAST','Beast']];
 const feed={currencyOverviews:[],itemOverviews:[]},overview=new Map();
 function bucket(type){if(!overview.has(type)){const out={type,lines:[]};overview.set(type,out);(type==='Currency'?feed.currencyOverviews:feed.itemOverviews).push(out);}return overview.get(type).lines;}
 const allKeys=[...keys.filter(Boolean),...drop.flatMap(d=>[...d.query,...d.items]).map(decode)];const used=new Set();let seq=0;
 for(const key of allKeys){const id=JSON.stringify(key);if(used.has(id))continue;used.add(id);const ns=namespaces.find(([ns])=>ns===key.ns);assert(ns,key.ns);seq++;if(seq%11===0)continue;bucket(ns[1]).push({name:key.name,...(key.variant!=null?{variant:key.variant}:{}),chaos:(seq%31)*17.25,graph:[null,-4,2,0,5,1,3]});}
 const divine=bucket('Currency').find(l=>l.name==='Divine Orb'&&l.variant==null);if(divine)divine.chaos=200;else bucket('Currency').push({name:'Divine Orb',chaos:200,graph:[]});
 const priceAst=ts.createSourceFile(pricesFile,read(pricesFile),ts.ScriptTarget.Latest,true);let routeArray;
 function routeWalk(n){if(ts.isVariableDeclaration(n)&&n.name.getText(priceAst)==='NAMESPACE_MAP')routeArray=n.initializer.getText(priceAst);ts.forEachChild(n,routeWalk);}routeWalk(priceAst);assert(routeArray);
 const routes=evaluate('result='+routeArray+';');const lookupKeys=[];
 for(const route of routes){const key={ns:route.ns,name:'Route '+route.type+' é',variant:'A/B, 6L'};bucket(route.type).push({name:key.name,variant:key.variant,chaos:101,graph:[0,1,null]});lookupKeys.push(key);}
 lookupKeys.push({ns:'ITEM',name:'Missing from feed',variant:undefined},{ns:'UNKNOWN',name:'Chaos Orb',variant:undefined});
 const prices=makePrices('Hardcore Covens',feed);
 const money=[];for(const rate of [null,30,200])for(const value of [0,0.008,1,29,30,100,188,199,200,201,211,212,10000])for(const max of [null,value+30])money.push({value,max,rate});
 const graphs=[[],[null],[0],[0,0],[null,1,null,2],[-1,-2,-3,-4,-5,-6,-7],[1,2,3,4,5,6,7],[1,-1,0,0,1,-1,0]];let seed=1;for(let n=0;n<500;n++){const g=[];for(let i=0;i<n%10;i++){seed=(Math.imul(seed,1664525)+1013904223)>>>0;g.push(seed%5===0?null:(seed%201-100)/10);}graphs.push(g);}
 const slugKeys=entries.map(e=>({ns:e.namespace,name:e.refName,variant:undefined}));slugKeys.push(...['Å é Æ ß : A-B','A__B/ C','é́','No variant'].map(name=>({ns:'ITEM',name,variant:''})));
 const nativeKey=k=>({namespace:k.ns,name:k.name,variant:k.variant??null});
 const input={snapshot:feed,league:'Hardcore Covens',items:cases.map(c=>c.text),lookups:lookupKeys.map(nativeKey),relations:relationKeys.map(nativeKey),slugs:slugKeys.map(nativeKey),money,graphs};
 const binary=path.resolve(process.argv[2]??path.join(host.root,'rust/target/debug/examples/economy-identity'));
 const out=cp.spawnSync(binary,{input:JSON.stringify(input),encoding:'utf8',maxBuffer:128*1024*1024});assert.equal(out.status,0,out.stderr);const actual=JSON.parse(out.stdout);assert.equal(actual.items.length,cases.length);
 let mismatch_count=0;const failures=[];const check=(label,fn)=>{try{fn();}catch(e){mismatch_count++;if(failures.length<8)failures.push({label,error:e.message});}};
 const plain=v=>JSON.parse(JSON.stringify(v));
 const quote=q=>q?{chaos:q.chaos,variant:q.variant??null,graph:[...q.graph],url:q.url}:null;
 const close=(a,b)=>assert(Math.abs(a-b)<=1e-10*Math.max(1,Math.abs(b)),`${a} != ${b}`);
 function eqMoney(a,b){if(!a||!b){assert.equal(a,b??null);return;}assert.equal(a.currency,b.currency);close(a.min,b.min);close(a.max,b.max);}
 for(let i=0;i<cases.length;i++)check(cases[i].label,()=>{
  const key=keys[i],row=actual.items[i];assert.deepEqual(row.key,key?nativeKey(key):null);
  if(!key){assert.equal(row.report,null);return;}
  const found=prices.findPriceByQuery(key);assert.deepEqual(quote(row.report.quote),quote(found));
  eqMoney(row.report.price,found?(key.name==='Divine Orb'?{min:found.chaos,max:found.chaos,currency:'chaos'}:prices.autoCurrency(found.chaos)):null);
  assert.equal(row.report.divine_rate,200);
  if(found&&parsed[i].stackSize){assert.equal(row.report.stack.have,parsed[i].stackSize.value);assert.equal(row.report.stack.maximum,parsed[i].stackSize.max);const money=n=>key.name==='Divine Orb'?{min:n*found.chaos,max:n*found.chaos,currency:'chaos'}:prices.autoCurrency(n*found.chaos);eqMoney(row.report.stack.price,money(parsed[i].stackSize.value));eqMoney(row.report.stack.full_stack_price,money(parsed[i].stackSize.max));}
 });
 for(let i=0;i<lookupKeys.length;i++)check('lookup '+i,()=>assert.deepEqual(quote(actual.lookups[i]),quote(prices.findPriceByQuery(lookupKeys[i]))));
 for(let i=0;i<relationIds.length;i++)check('related '+relationIds[i],()=>{
  const expected=plain(related(relationIds[i],prices)),a=actual.relations[i];
  if(expected===null){assert.equal(a,null);return;}if(expected.error){assert.equal(a.error,expected.error);return;}
  const compact=(v,native)=>({name:v.name,price:v.price??null,...('highlight'in v?{highlight:v.highlight}:native&&v.highlight?{highlight:true}:{})});
  assert.equal(a.error,null);
  assert.deepEqual(a.related.map(v=>({name:v.name,price:v.price,highlight:v.highlight})),expected.related.map(v=>({name:v.name,price:v.price??null,highlight:v.highlight})));
  assert.deepEqual(a.items.map(v=>({name:v.name,price:v.price})),expected.items.map(v=>({name:v.name,price:v.price??null})));
 });
 for(let i=0;i<money.length;i++)check('currency '+i,()=>{const m=money[i],p=makePrices('Covens',{currencyOverviews:[{type:'Currency',lines:m.rate==null?[]:[{name:'Divine Orb',chaos:m.rate}]}]});eqMoney(actual.money[i],p.autoCurrency(m.max==null?m.value:[m.value,m.max]));});
 for(let i=0;i<graphs.length;i++)check('trend '+i,()=>{const e=plain(trend(graphs[i])),a=actual.graphs[i];if(!e){assert.equal(a,null);return;}assert.deepEqual(a.points,e.graph.points);assert.equal(a.forecast,e.forecast);assert.equal(a.text,e.text);close(a.draw_min,e.graph.drawMin);close(a.draw_max,e.graph.drawMax);});
 for(let i=0;i<slugKeys.length;i++)check('slug '+i,()=>{const k=slugKeys[i],expected=(k.variant?`${k.name}, ${k.variant}`:k.name).normalize('NFKD').replace(/[^a-zA-Z0-9:\- ]/g,'').toLowerCase().replace(/ /g,'-');assert.equal(actual.slugs[i],expected);});
 const ruleRefs=[];function ruleWalk(n){if(ts.isCallExpression(n)&&ts.isIdentifier(n.expression)&&n.expression.text==='hasStat')ruleRefs.push(n.arguments[1].text);ts.forEachChild(n,ruleWalk);}ruleWalk(fn);
 const staleRefs=[...new Set(ruleRefs)].filter(ref=>!stats.some(s=>s.ref===ref));
 const sources=[identityFile,pricesFile,relatedFile,trendFile,'renderer/public/data/item-drop.json'];
 const report={item_cases:cases.length,route_lookup_cases:lookupKeys.length,stale_unique_rule_references:staleRefs,coffin_base_in_catalogue:entries.some(e=>e.namespace==='ITEM'&&e.refName==='Filled Coffin'),related_cases:relationIds.length,currency_cases:money.length,trend_cases:graphs.length,slug_cases:slugKeys.length,mismatch_count,failures,upstream_commit:'3188208957422b21b69b12083a03edf3d1bac187',scope:'Unchanged English parser and pure AST functions for economy identity, dense lookup, autoCurrency, trend variance/forecast and related-item prices. Synthetic deterministic feed with missing and zero prices; all unique identities, specialised modifier combinations, base brackets/influences, gems/maps and any catalogue-supported coffins, three actual copied ordinary fixtures. No live price parity or image coverage.',source_sha256:Object.fromEntries(sources.map(s=>[s,crypto.createHash('sha256').update(read(s)).digest('hex')]))};
 if(process.argv[3])fs.writeFileSync(process.argv[3],JSON.stringify(report,null,2)+'\n');console.log(JSON.stringify(report,null,2));if(mismatch_count)process.exitCode=1;
}
main().catch(e=>{console.error(e);process.exitCode=1;});
