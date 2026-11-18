'use strict';
(() => {
  const {concepts,items,words}=CrabLab;
  const params=new URLSearchParams(location.search);
  const concept=concepts.find(c=>c.id===params.get('style'))||concepts[0];
  const preview=params.get('preview')==='1';
  const embedded=preview||params.get('embedded')==='1';
  const panel=document.querySelector('#trade-panel');
  const stage=document.querySelector('#stage');
  const tip=document.querySelector('#offer-tooltip');
  const dialog=document.querySelector('#settings-dialog');
  const veil=document.querySelector('#scene-veil');
  const reopen=document.querySelector('#reopen');
  const escape=text=>String(text).replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
  let language=params.get('lang')==='fr'?'fr':'en';
  let itemId=Object.hasOwn(items,params.get('item'))?params.get('item'):'ring';
  let state,epoch=0,hoverTimer=null,tipOwner=null;
  const t=key=>words[language][key]||key;
  const asset=name=>`assets/${name}`;
  const money=(value,large=false,currency='chaos')=>`<span class="${large?'quote-number':'offer-price'}">${value}<img ${large?'class="currency-art"':''} src="${asset(currency+'.png')}" alt="${currency==='chaos'?'Chaos Orb':'Divine Orb'}"><span class="price-unit">${currency}</span></span>`;
  const allMods=()=>[...items[itemId].mods,...items[itemId].more];
  const mark='<span class="selection-mark" aria-hidden="true"></span>';
  const isCurrency=()=>itemId==='currency';
  function reset(){
    epoch++;
    const dense=state?.dense||false;
    state={league:'Covens',open:true,busy:false,searched:false,dirty:false,offers:[],searches:0,query:null,tab:'filters',base:false,online:true,corruption:items[itemId].corrupted?'true':'false',priceCurrency:'any',mirrored:false,rarity:items[itemId].rarity==='rare'?'rare':items[itemId].rarity==='unique'?'unique':'any',sort:'asc',more:false,options:false,dense,filters:{},gates:{}};
    for(const mod of allMods())state.filters[mod.id]={selected:mod.selected,min:String(Math.max(mod.lo,Math.min(mod.hi,Math.floor(mod.roll*.9)))),max:''};
    for(const name of ['ilvl','level','quality','tier']){
      if(name==='ilvl'&&['gem','map','currency'].includes(itemId))continue;
      const value=name==='ilvl'?items[itemId].ilvl:items[itemId][name];
      if(value!=null)state.gates[name]={active:name!=='ilvl',value:String(value)};
    }
    if(preview&&!isCurrency()){state.searched=true;state.offers=filteredOffers();state.tab=concept.layout==='tabs'?'offers':'filters';}
    hideTip();render();
  }
  function meta(){
    const item=items[itemId];
    if(itemId==='gem')return `${t('level')} ${item.level} · ${t('quality')} ${item.quality}%`;
    if(itemId==='viscera')return `Viscera · ${item.slot} · ilvl ${item.ilvl}`;
    if(itemId==='map')return `${t('tier')} ${item.tier} · ${t('quality')} ${item.quality}%`;
    return item.ilvl==null?item.family:`${item.family} · ilvl ${item.ilvl}`;
  }
  function numericGate(name){
    const gate=state.gates[name];if(!gate)return '';
    const label=name==='ilvl'?'ilvl':t(name);
    return `<div class="numeric-chip"><button class="chip" data-gate="${name}" aria-pressed="${gate.active}" aria-label="${escape(label)} ${t('minimum')}">${mark}<span>${label}</span></button><input data-gate-value="${name}" aria-label="${escape(label)} ${t('minimum')}" type="number" step="1" min="0" max="${name==='ilvl'?100:100}" value="${escape(gate.value)}" ${gate.active?'':'disabled'}></div>`;
  }
  function selectField(name,label,choices,value){return `<label class="field-control">${label}<select data-option="${name}" aria-label="${label}">${choices.map(([id,text])=>`<option value="${id}" ${id===value?'selected':''}>${text}</option>`).join('')}</select></label>`;}
  function criteria(){
    const item=items[itemId];
    const dropdowns=selectField('corruption',t('corruption'),[['any',t('any')],['false',t('notCorrupted')],['true',t('corrupted')]],state.corruption)+(itemId==='gem'||itemId==='viscera'?'':selectField('rarity',t('rarity'),[['any',t('any')],['rare',t('rare')],['unique',t('unique')],['normal',t('normal')]],state.rarity));
    const advanced=`<label class="field-control wide"><input type="checkbox" data-extra="mirrored" ${state.mirrored?'checked':''}>${t('mirrored')}</label>${selectField('priceCurrency',t('price'),[['any',t('allCurrencies')],['chaos','Chaos Orb'],['divine','Divine Orb']],state.priceCurrency)}`;
    return `<div class="criteria">${itemId==='gem'?'':`<button class="chip" data-action="base" aria-pressed="${state.base}" aria-label="${t('sameBase')}: ${escape(item.base)}">${mark}<span class="chip-text">${escape(item.base)}</span></button>`}${Object.keys(state.gates).map(numericGate).join('')}</div>${concept.id==='compact'?'':`<div class="option-grid">${dropdowns}</div>`}<details class="compact-options" ${state.options?'open':''}><summary>${t('options')}</summary><div class="option-grid">${concept.id==='compact'?dropdowns:''}${advanced}</div></details>`;
  }
  function modRow(mod){
    const f=state.filters[mod.id];
    const min=Number(f.min===''?mod.lo:f.min),max=Number(f.max===''?mod.hi:f.max);
    const pct=value=>Math.max(0,Math.min(100,(value-mod.lo)/(mod.hi-mod.lo||1)*100));
    return `<div class="mod-row ${f.selected?'selected':''}" data-mod="${mod.id}"><button class="mod-toggle" aria-pressed="${f.selected}" aria-label="${escape(mod.text)}">${mark}<span class="mod-label">${escape(mod.text)}${mod.implicit?'<span class="mod-kind">implicit</span>':''}</span>${mod.influence?`<img class="mod-influence" src="${asset(mod.influence)}" alt="Hunter influence">`:''}</button><div class="range-controls"><div class="range-wrap" data-range="${mod.id}"><div class="range-base"><span class="range-fill" style="width:${pct(min)}%"></span></div><span class="roll-marker" style="left:${pct(mod.roll)}%" title="${mod.roll}"></span><input class="range-input min-handle" type="range" min="${mod.lo}" max="${mod.hi}" value="${min}" step="1" data-bound="min" aria-label="${t('minimum')}: ${escape(mod.text)}"><input class="range-input max-handle" type="range" min="${mod.lo}" max="${mod.hi}" value="${max}" step="1" data-bound="max" aria-label="${t('maximum')}: ${escape(mod.text)}" ${f.max===''?'disabled':''}></div><div class="bounds"><input type="number" step="1" min="-100000" max="100000" inputmode="numeric" data-bound="min" value="${escape(f.min)}" placeholder="${t('min')}" aria-label="${t('minimum')}: ${escape(mod.text)}"><input type="number" step="1" min="-100000" max="100000" inputmode="numeric" data-bound="max" value="${escape(f.max)}" placeholder="${t('max')}" aria-label="${t('maximum')}: ${escape(mod.text)}"></div></div></div>`;
  }
  function actions(){return `<div class="action-bar"><button class="primary-action" data-action="search">${t('search')}</button><button class="secondary-action" data-action="trade" ${state.searched?'':'disabled'}>${t('openTrade')} ↗</button></div>`;}
  function render(){
    const item=items[itemId];
    document.documentElement.lang=language;
    panel.className=`trade-panel theme-${concept.id}${state.dense?' dense-ui':''}`;
    panel.dataset.tab=state.tab;panel.dataset.layout=concept.layout;panel.hidden=!state.open;veil.hidden=!state.open;reopen.hidden=state.open;
    for(const node of [tip,dialog])node.className=`${node===tip?'offer-tooltip':'settings-dialog'} owned theme-${concept.id}`;
    const count=Object.values(state.filters).filter(f=>f.selected).length;
    const head=`<header class="panel-head"><img class="brand-mark" src="${asset('crab.png')}" alt=""><h2 class="head-name" title="${escape(item.name)}">${escape(item.name)}</h2><button class="head-close" data-action="close" aria-label="${t('close')}">×</button><div class="head-controls"><select data-option="league" aria-label="League"><option ${state.league==='Covens'?'selected':''}>Covens</option><option ${state.league==='Standard'?'selected':''}>Standard</option></select><label class="online-control"><input type="checkbox" data-option="online" ${state.online?'checked':''}>${t('online')}</label><button class="head-settings" data-action="settings">${t('settings')}</button></div></header>`;
    const band=`<div class="item-band"><img class="item-art" src="${asset(item.image)}" alt="${escape(item.base)}"><div class="item-info"><strong>${isCurrency()?t('reference'):meta()}</strong>${isCurrency()?t('ninja'):item.corrupted?t('corrupted'):item.quote==null?'':t('reference')}</div>${item.quote==null||isCurrency()?'':`<div class="item-quote">${money(item.quote,true)}<span class="quote-source">poe.ninja</span></div>`}</div>`;
    const progressive=['tide','focus','compact'].includes(concept.id);
    const mainMods=item.mods.filter(m=>!progressive||m.selected);
    const otherMods=[...item.mods.filter(m=>progressive&&!m.selected),...item.more];
    const mods=`<div class="mod-list">${mainMods.map(modRow).join('')}</div>${otherMods.length?`<details class="more-mods" ${state.more?'open':''}><summary>${t('more')} · ${otherMods.length}</summary><div class="mod-list">${otherMods.map(modRow).join('')}</div></details>`:''}`;
    if(isCurrency()){
      panel.innerHTML=head+`<div class="currency-view">${band}<div class="currency-rate">${item.quote}<img src="${asset('chaos.png')}" alt="Chaos Orb"></div><div class="exchange-line"><span>${t('stack')} × ${item.stack}</span><strong>${item.stack*item.quote}<img class="currency-art" src="${asset('chaos.png')}" alt="Chaos Orb"></strong></div><button class="secondary-action ninja-action" data-action="ninja">${t('viewNinja')} ↗</button></div>`;
    }else{
      panel.innerHTML=head+`<section class="context-section" aria-label="${t('identity')}">${band}${criteria()}</section><nav class="panel-tabs" role="tablist" aria-label="View"><button role="tab" id="filters-tab" aria-controls="filters-view" tabindex="${state.tab==='filters'?0:-1}" aria-selected="${state.tab==='filters'}" data-tab-action="filters">${t('filters')}</button><button role="tab" id="offers-tab" aria-controls="offers-view" tabindex="${state.tab==='offers'?0:-1}" aria-selected="${state.tab==='offers'}" data-tab-action="offers">${t('offers')}<span class="tab-count">${state.searched?' · '+state.offers.length:''}</span></button></nav><div class="workspace"><section id="filters-view" class="filter-column" ${concept.layout==='tabs'?'role="tabpanel" aria-labelledby="filters-tab"':''} aria-label="${t('filters')}"><h3 class="section-heading"><span class="section-index">02</span>${t('mods')}<span class="selection-count">${count} ${t('selected')}</span></h3>${allMods().length?mods:`<div class="empty-results"><strong>${t('noMods')}</strong></div>`}<div class="error-line" role="alert" hidden></div>${concept.layout==='dock'?'':actions()}</section><section id="offers-view" class="results-column" ${concept.layout==='tabs'?'role="tabpanel" aria-labelledby="offers-tab"':''} aria-label="${t('offers')}" aria-live="polite"></section></div>${concept.layout==='dock'?actions():''}`;
      renderResults();
    }
    panel.querySelectorAll('details').forEach(details=>details.addEventListener('toggle',()=>{if(details.classList.contains('more-mods'))state.more=details.open;else state.options=details.open;}));
    panel.querySelectorAll('select').forEach(select=>select.addEventListener('wheel',()=>select.blur(),{passive:true}));
    document.querySelector('#lab-language').textContent=language.toUpperCase()+' / '+(language==='en'?'FR':'EN');
    reopen.innerHTML=`${t('reopen')} <kbd>Ctrl Alt D</kbd>`;
    validate(false);
  }
  function makeOffers(){
    const item=items[itemId];
    const prices=itemId==='ring'?[5,7,8,10,12,15]:itemId==='wand'?[1,2,3,5,6,8]:[item.quote||6,(item.quote||6)+1,(item.quote||6)+2,(item.quote||6)+3,(item.quote||6)+5,(item.quote||6)+8];
    return prices.map((price,index)=>{
      const values={};allMods().forEach((mod,j)=>{const ratio=[.7,.45,1,.25,.85,.6][(index+j)%6];values[mod.id]=Math.round(mod.lo+(mod.hi-mod.lo)*ratio);});
      const base=itemId==='ring'?['Gold Ring','Coral Ring','Two-Stone Ring'][index%3]:itemId==='wand'?['Driftwood Wand','Spiraled Wand'][index%2]:item.base;
      const name=itemId==='ring'?['Dread Circle','Mind Loop','Soul Coil','Gloom Circle','Dusk Knot','Bramble Loop'][index]:itemId==='wand'?'Mind Branch':item.name;
      return {id:index,name,base,price,currency:index===5?'divine':'chaos',online:index!==5,mirrored:index===4,values,ilvl:item.ilvl==null?null:item.ilvl+(index%3),seller:['TidalMerchant','HarborExile','MavenEnjoyer','QuietAtlas','CopperCrab','MapRunner'][index],age:[2,5,8,14,25,42][index],corrupted:item.corrupted||false,implicit:itemId==='ring'?['15% increased Rarity of Items found']:item.mods.filter(m=>m.implicit).map(m=>m.text)};
    });
  }
  function filteredOffers(){
    return makeOffers().filter(offer=>{
      if(state.online&&!offer.online)return false;
      if(state.mirrored&&offer.mirrored)return false;
      if(state.priceCurrency!=='any'&&offer.currency!==state.priceCurrency)return false;
      if(state.base&&offer.base!==items[itemId].base)return false;
      if(state.gates.ilvl?.active&&offer.ilvl<Number(state.gates.ilvl.value))return false;
      if(state.corruption!=='any'&&offer.corrupted!==(state.corruption==='true'))return false;
      if(state.rarity!=='any'&&state.rarity!==items[itemId].rarity)return false;
      for(const [name,gate] of Object.entries(state.gates)){if(name!=='ilvl'&&gate.active&&(items[itemId][name]??0)<Number(gate.value))return false;}
      return allMods().every(mod=>{const f=state.filters[mod.id];return !f.selected||((f.min===''||offer.values[mod.id]>=Number(f.min))&&(f.max===''||offer.values[mod.id]<=Number(f.max)));});
    });
  }
  function renderResults(){
    const container=panel.querySelector('.results-column');if(!container)return;
    if(!state.searched||state.dirty||state.busy){
      const title=state.busy?t('searchTitle'):state.dirty?t('changedTitle'):t('emptyTitle');
      container.innerHTML=`<div class="results-head"><span class="section-heading"><span class="section-index">03</span>${t('offers')}</span></div><div class="empty-results"><span class="empty-dash"></span><strong>${title}</strong><span>${state.busy?'':state.dirty?t('changedBody'):t('emptyBody')}</span>${state.busy?'<span class="loading-line" aria-hidden="true"></span>':''}</div>`;
    }else{
      const offers=[...state.offers].sort((a,b)=>state.sort==='asc'?a.price*(a.currency==='divine'?200:1)-b.price*(b.currency==='divine'?200:1):b.price*(b.currency==='divine'?200:1)-a.price*(a.currency==='divine'?200:1));
      container.innerHTML=`<div class="results-head"><span><span class="section-index">03</span>${offers.length} ${t('offersAvailable')}</span><button data-action="sort" aria-label="${t('price')} ${state.sort==='asc'?'descending':'ascending'}">${t('price')} ${state.sort==='asc'?'↑':'↓'}</button></div><div class="offer-list" role="list" aria-label="${t('offers')}">${offers.length?offers.map(offer=>`<div role="listitem"><button class="offer-row" data-offer="${offer.id}" aria-label="${escape(offer.name)}, ${offer.price} ${offer.currency==='chaos'?'Chaos Orb':'Divine Orb'}"><span class="offer-name">${escape(offer.name)} · ${escape(offer.base)}</span><span class="offer-seller">${offer.seller} · ${offer.age} ${t('minute')}</span>${money(offer.price,false,offer.currency)}</button></div>`).join(''):`<div class="empty-results"><strong>${language==='fr'?'Aucune offre correspondante.':'No matching offers.'}</strong><span>${language==='fr'?'Ajuste les critères puis relance la recherche.':'Adjust the filters and search again.'}</span></div>`}</div>`;
      container.querySelectorAll('.offer-row').forEach(row=>{
        row.addEventListener('pointerenter',()=>{clearTimeout(hoverTimer);hoverTimer=setTimeout(()=>showTip(row),65);});
        row.addEventListener('pointerleave',hideTip);
        row.addEventListener('focus',()=>showTip(row));row.addEventListener('blur',hideTip);
      });
      container.querySelector('.offer-list')?.addEventListener('scroll',hideTip,{passive:true});
    }
    panel.setAttribute('aria-busy',String(state.busy));
    panel.querySelectorAll('[data-action=search]').forEach(button=>{button.disabled=state.busy;button.textContent=state.busy?t('searching'):t('search');});
    panel.querySelectorAll('[data-action=trade]').forEach(button=>button.disabled=!state.searched||state.dirty||state.busy);
    const count=panel.querySelector('.tab-count');if(count)count.textContent=state.searched&&!state.dirty?' · '+state.offers.length:'';
  }
  function invalidate(){
    const wasDirty=state.dirty,wasBusy=state.busy;epoch++;state.busy=false;state.dirty=true;state.searched=false;state.query=null;state.offers=[];
    hideTip();if(!wasDirty||wasBusy)renderResults();else panel.querySelectorAll('[data-action=search]').forEach(b=>{b.disabled=false;b.textContent=t('search');});
    const count=panel.querySelector('.selection-count');if(count)count.textContent=`${Object.values(state.filters).filter(f=>f.selected).length} ${t('selected')}`;
    validate(false);
  }
  function validate(show){
    let error='';
    for(const mod of allMods()){
      const f=state.filters[mod.id],min=f.min===''?null:Number(f.min),max=f.max===''?null:Number(f.max);
      const invalid=f.selected&&((min!==null&&!Number.isInteger(min))||(max!==null&&!Number.isInteger(max)));
      const inverted=f.selected&&min!==null&&max!==null&&min>max;
      panel.querySelectorAll(`[data-mod="${mod.id}"] .bounds input`).forEach(input=>input.setAttribute('aria-invalid',String(invalid||inverted)));
      if(invalid)error=t('invalidNumber');else if(inverted)error=t('invalid');
    }
    for(const gate of Object.values(state.gates))if(gate.active&&(!Number.isInteger(Number(gate.value))||gate.value===''||Number(gate.value)<0||Number(gate.value)>100))error=t('invalidNumber');
    const line=panel.querySelector('.error-line');if(line){line.hidden=!error;line.textContent=error;}
    if(error&&show)panel.querySelector('[aria-invalid=true]')?.focus();
    return !error;
  }
  function search(){
    if(isCurrency()||state.busy||!state.open||!validate(true))return;
    hideTip();const job=++epoch;state.busy=true;state.searches++;state.query={league:state.league,base:state.base?items[itemId].base:null,online:state.online,priceCurrency:state.priceCurrency,excludeMirrored:state.mirrored,corruption:state.corruption,rarity:state.rarity,modifiers:allMods().filter(m=>state.filters[m.id].selected).map(m=>({text:m.text,min:state.filters[m.id].min===''?null:Number(state.filters[m.id].min),max:state.filters[m.id].max===''?null:Number(state.filters[m.id].max)})),properties:structuredClone(state.gates)};
    state.tab='offers';panel.dataset.tab=state.tab;syncTabs();renderResults();
    setTimeout(()=>{if(job!==epoch||!state.open)return;state.offers=filteredOffers();state.busy=false;state.searched=true;state.dirty=false;renderResults();},180);
  }
  function syncTabs(){panel.querySelectorAll('[data-tab-action]').forEach(b=>{const active=b.dataset.tabAction===state.tab;b.setAttribute('aria-selected',String(active));b.tabIndex=active?0:-1;});}
  panel.addEventListener('keydown',event=>{if(event.target.matches('[role=tab]')&&['ArrowLeft','ArrowRight'].includes(event.key)){event.preventDefault();state.tab=state.tab==='filters'?'offers':'filters';panel.dataset.tab=state.tab;syncTabs();panel.querySelector(`[data-tab-action="${state.tab}"]`).focus();hideTip();}});
  function updateMod(id){
    const mod=allMods().find(m=>m.id===id),f=state.filters[id],row=panel.querySelector(`[data-mod="${id}"]`);
    row.classList.toggle('selected',f.selected);row.querySelector('.mod-toggle').setAttribute('aria-pressed',String(f.selected));
    const min=Number(f.min===''?mod.lo:f.min),max=Number(f.max===''?mod.hi:f.max);
    row.querySelector('.range-fill').style.width=Math.max(0,Math.min(100,(min-mod.lo)/(mod.hi-mod.lo||1)*100))+'%';
    row.querySelector('.min-handle').value=String(min);row.querySelector('.max-handle').value=String(max);row.querySelector('.max-handle').disabled=f.max==='';
    row.querySelectorAll('.bounds input').forEach(input=>{if(document.activeElement!==input)input.value=f[input.dataset.bound];});
    invalidate();
  }
  panel.addEventListener('click',event=>{
    const action=event.target.closest('[data-action]')?.dataset.action;
    if(action){if(action==='search')search();else if(action==='close')close();else if(action==='settings')settings();else if(action==='base'){state.base=!state.base;const b=panel.querySelector('[data-action=base]');b.setAttribute('aria-pressed',String(state.base));invalidate();}else if(action==='sort'){hideTip();state.sort=state.sort==='asc'?'desc':'asc';renderResults();}else if(action==='trade'||action==='ninja')notice(t('demoLink'));return;}
    const gate=event.target.closest('[data-gate]');if(gate){const name=gate.dataset.gate;state.gates[name].active=!state.gates[name].active;gate.setAttribute('aria-pressed',String(state.gates[name].active));panel.querySelector(`[data-gate-value="${name}"]`).disabled=!state.gates[name].active;invalidate();return;}
    const tab=event.target.closest('[data-tab-action]');if(tab){state.tab=tab.dataset.tabAction;panel.dataset.tab=state.tab;syncTabs();hideTip();return;}
    const row=event.target.closest('[data-mod]');if(row&&!event.target.closest('input,.range-wrap,.bounds')){const id=row.dataset.mod;state.filters[id].selected=!state.filters[id].selected;updateMod(id);}
  });
  panel.addEventListener('input',event=>{
    const input=event.target;if(input.dataset.bound){const row=input.closest('[data-mod]'),f=state.filters[row.dataset.mod];f[input.dataset.bound]=input.value;f.selected=true;updateMod(row.dataset.mod);}
    else if(input.dataset.gateValue){state.gates[input.dataset.gateValue].value=input.value;invalidate();}
  });
  panel.addEventListener('change',event=>{
    const input=event.target;if(input.dataset.option){if(input.dataset.option==='online')state.online=input.checked;else state[input.dataset.option]=input.value;invalidate();}
    else if(input.dataset.extra){state[input.dataset.extra]=input.checked;invalidate();}
  });
  let drag=null;
  panel.addEventListener('pointerdown',event=>{
    const wrap=event.target.closest('.range-wrap');if(!wrap||event.target.matches('input'))return;
    const id=wrap.dataset.range,mod=allMods().find(m=>m.id===id),f=state.filters[id];
    const valueAt=event=>{const r=wrap.getBoundingClientRect();return Math.round(mod.lo+Math.max(0,Math.min(1,(event.clientX-r.left)/r.width))*(mod.hi-mod.lo));};
    const n=valueAt(event),bound=f.max!==''&&Math.abs(n-Number(f.max))<Math.abs(n-Number(f.min||mod.lo))?'max':'min';
    drag={wrap,id,bound,valueAt};wrap.setPointerCapture(event.pointerId);f[bound]=String(n);f.selected=true;updateMod(id);event.preventDefault();
  });
  panel.addEventListener('pointermove',event=>{if(drag){state.filters[drag.id][drag.bound]=String(drag.valueAt(event));updateMod(drag.id);}});
  panel.addEventListener('pointerup',()=>drag=null);panel.addEventListener('pointercancel',()=>drag=null);
  function offeredMods(offer){
    return allMods().filter(m=>!m.implicit).map(mod=>{
      const value=offer.values[mod.id];return mod.text.replace(/[+-]?\d+/,match=>match.startsWith('+')?'+'+value:String(value));
    });
  }
  function showTip(row){
    if(!row.isConnected||!state.open||dialog.open||state.dirty)return;
    const offer=state.offers.find(o=>o.id===Number(row.dataset.offer));if(!offer)return;
    tipOwner=row;tip.id='offer-tooltip';row.setAttribute('aria-describedby','offer-tooltip');
    tip.innerHTML=`<div class="tip-title rarity-${items[itemId].rarity}">${escape(offer.name)}</div><div class="tip-base rarity-${items[itemId].rarity}">${escape(offer.base)}</div><div class="tip-meta">${offer.ilvl==null?'':`ilvl ${offer.ilvl}`}${itemId==='gem'?` · ${t('level')} ${items[itemId].level} · ${t('quality')} ${items[itemId].quality}%`:''}${offer.corrupted?`<span class="tip-corrupted">${t('corrupted')}</span>`:''}</div><div class="tip-mods">${offer.implicit.length?`<div class="tip-implicit">${offer.implicit.map(escape).join('<br>')}</div>`:''}${offeredMods(offer).map(escape).join('<br>')}</div><div class="tip-price">${offer.price}<img class="currency-art" src="${asset(offer.currency+'.png')}" alt="${offer.currency==='chaos'?'Chaos Orb':'Divine Orb'}"><span class="price-unit">${offer.currency}</span></div>`;
    tip.style.maxHeight='none';tip.style.width=Math.min(320,innerWidth-16)+'px';tip.hidden=false;
    const p=panel.getBoundingClientRect(),r=row.getBoundingClientRect(),box=tip.getBoundingClientRect();let x,y;
    if(p.left>=box.width+20){x=p.left-box.width-10;y=Math.max(8,Math.min(innerHeight-box.height-8,r.top));}
    else if(innerWidth-p.right>=box.width+20){x=p.right+10;y=Math.max(8,Math.min(innerHeight-box.height-8,r.top));}
    else{
      x=Math.max(8,Math.min(innerWidth-box.width-8,r.left));const above=r.top-16,below=innerHeight-r.bottom-16;
      if(above>=box.height||above>below){tip.style.maxHeight=Math.max(0,above)+'px';y=Math.max(8,r.top-Math.min(box.height,above)-8);}else{tip.style.maxHeight=Math.max(0,below)+'px';y=r.bottom+8;}
    }
    tip.style.left=x+'px';tip.style.top=y+'px';
  }
  function hideTip(){clearTimeout(hoverTimer);tip.hidden=true;tipOwner?.removeAttribute('aria-describedby');tipOwner=null;}
  function close(){epoch++;state.open=false;state.busy=false;panel.hidden=true;veil.hidden=true;reopen.hidden=false;hideTip();if(dialog.open)dialog.close();}
  function notice(text){document.querySelector('.notice')?.remove();const element=document.createElement('div');element.className='notice owned';element.setAttribute('role','status');element.textContent=text;document.body.append(element);setTimeout(()=>element.remove(),2200);}
  function settings(){
    hideTip();dialog.innerHTML=`<h2 id="settings-title">${t('settings')}</h2><form method="dialog"><label>${t('language')}<select name="language"><option value="en" ${language==='en'?'selected':''}>English</option><option value="fr" ${language==='fr'?'selected':''}>Français</option></select></label><label>${t('density')}<select name="density"><option value="comfortable" ${state.dense?'':'selected'}>${t('comfortable')}</option><option value="dense" ${state.dense?'selected':''}>${t('dense')}</option></select></label><div class="settings-actions"><button value="cancel">${t('cancel')}</button><button value="save" class="primary-action">${t('save')}</button></div></form>`;
    dialog.returnValue='';dialog.showModal();
  }
  dialog.addEventListener('close',()=>{if(dialog.returnValue==='save'){language=dialog.querySelector('[name=language]').value;state.dense=dialog.querySelector('[name=density]').value==='dense';render();}});
  dialog.addEventListener('click',event=>{if(event.target===dialog){const r=dialog.getBoundingClientRect();if(event.clientX<r.left||event.clientX>r.right||event.clientY<r.top||event.clientY>r.bottom)dialog.close('cancel');}});
  stage.addEventListener('click',event=>{if(!embedded&&state.open&&!dialog.open&&!event.target.closest('.trade-panel,.owned,#reopen,.concept-aside'))close();});
  reopen.onclick=reset;
  document.addEventListener('keydown',event=>{
    if(event.ctrlKey&&event.altKey&&event.code==='KeyD'){event.preventDefault();if(dialog.open)dialog.close('cancel');reset();}
    else if(event.key==='Escape'&&!dialog.open){event.preventDefault();close();}
    else if(event.key==='Enter'&&!dialog.open&&state.open&&!event.target.closest('button,select,summary')){event.preventDefault();search();}
  });
  addEventListener('resize',hideTip);stage.addEventListener('scroll',hideTip,{passive:true});
  panel.addEventListener('scroll',hideTip,{capture:true,passive:true});
  addEventListener('message',event=>{
    if(event.source!==parent||parent===window||!event.data?.crabLab)return;
    const message=event.data;if(message.item&&Object.hasOwn(items,message.item)){itemId=message.item;reset();}
    if(message.language&&['fr','en'].includes(message.language)){language=message.language;render();}
    if(message.reset)reset();
  });
  const index=concepts.indexOf(concept);document.title=`${String(index+1).padStart(2,'0')} ${concept.name} — Crab Trade`;
  document.body.classList.toggle('embedded',embedded);document.body.classList.toggle('preview',preview);
  panel.style.setProperty('--panel-width',`${concept.width}px`);
  const stylePicker=document.querySelector('#style-picker');stylePicker.innerHTML=concepts.map((c,i)=>`<option value="${c.id}" ${c===concept?'selected':''}>${String(i+1).padStart(2,'0')} / ${c.name}</option>`).join('');
  const navigate=id=>{location.href=`prototype.html?style=${id}&item=${itemId}&lang=${language}`;};
  stylePicker.onchange=()=>navigate(stylePicker.value);
  document.querySelector('#previous-style').onclick=()=>navigate(concepts[(index+9)%10].id);
  document.querySelector('#next-style').onclick=()=>navigate(concepts[(index+1)%10].id);
  const itemPicker=document.querySelector('#item-picker');itemPicker.innerHTML=Object.entries(items).map(([id,item])=>`<option value="${id}" ${itemId===id?'selected':''}>${escape(item.name)}</option>`).join('');
  itemPicker.onchange=()=>{itemId=itemPicker.value;params.set('item',itemId);params.set('style',concept.id);history.replaceState(null,'','?'+params);reset();};
  const widthPicker=document.querySelector('#width-picker');widthPicker.value=concept.width;widthPicker.onchange=()=>{hideTip();panel.style.setProperty('--panel-width',widthPicker.value+'px');};
  document.querySelector('#lab-language').onclick=()=>{language=language==='en'?'fr':'en';render();};
  document.querySelector('#concept-aside').innerHTML=`<p class="eyebrow">DIRECTION ${String(index+1).padStart(2,'0')} / 10</p><h1>${concept.name}</h1><p>${concept.description}</p><details><summary>Les choix de design</summary><ul>${concept.notes.map(n=>`<li>${n}</li>`).join('')}</ul><p class="tradeoff">${concept.tradeoff}</p></details>`;
  document.querySelector('#design-note').textContent=concept.principles.join(' / ');
  const inventory=document.querySelector('.inventory-grid');inventory.innerHTML=Array.from({length:36},(_,i)=>`<span class="inventory-cell">${i%5===0?`<img src="${asset(['ring.png','gem.png','map.png','divine.png','viscera-siren.png'][Math.floor(i/5)%5])}" alt="">`:''}</span>`).join('');
  window.crabPrototype=Object.freeze({snapshot:()=>structuredClone({concept:concept.id,item:itemId,language,...state})});
  reset();
})();
