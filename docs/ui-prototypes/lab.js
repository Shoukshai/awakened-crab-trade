'use strict';
(() => {
  const {concepts,items}=CrabLab;
  const options=selected=>Object.entries(items).map(([id,item])=>`<option value="${id}" ${id===selected?'selected':''}>${item.name}</option>`).join('');
  const styleOptions=selected=>concepts.map((c,i)=>`<option value="${c.id}" ${selected===c.id?'selected':''}>${String(i+1).padStart(2,'0')} / ${c.name}</option>`).join('');
  const query=new URLSearchParams(location.search);
  let item=Object.hasOwn(items,query.get('item'))?query.get('item'):'ring';
  let language=query.get('lang')==='fr'?'fr':'en';
  const notice=text=>{document.querySelector('.notice')?.remove();const n=document.createElement('div');n.className='notice';n.setAttribute('role','status');n.textContent=text;document.body.append(n);setTimeout(()=>n.remove(),2500);};
  if(document.body.classList.contains('gallery-page')){
    const selected=new Set();
    const picker=document.querySelector('#gallery-item');picker.innerHTML=options(item);
    document.querySelector('#gallery').innerHTML=concepts.map((c,i)=>`<article class="concept-card" data-style="${c.id}"><a class="card-preview" href="prototype.html?style=${c.id}&item=${item}" aria-label="Tester ${c.name}"><span class="preview-surface"></span><span class="preview-index">${String(i+1).padStart(2,'0')}</span><iframe class="preview-frame" style="width:${c.width}px;margin-left:-${c.width/2}px" title="Aperçu ${c.name}" src="prototype.html?style=${c.id}&preview=1&item=${item}" tabindex="-1" aria-hidden="true" inert loading="lazy"></iframe><span class="preview-open">Tester ce style ↗</span></a><div class="card-title-row"><span class="card-number">${String(i+1).padStart(2,'0')}</span><h2>${c.name}</h2><label class="compare-choice"><input type="checkbox" value="${c.id}" aria-label="Comparer ${c.name}">Comparer</label></div><p class="concept-tag">${c.tag}</p><p>${c.description}</p></article>`).join('');
    document.querySelectorAll('.concept-card').forEach(card=>card.prepend(card.querySelector('.card-title-row')));
    const compare=document.querySelector('#compare-selected');
    document.querySelector('#gallery').addEventListener('change',event=>{
      const box=event.target;if(!box.matches('input[type=checkbox]'))return;
      if(box.checked&&selected.size===2){box.checked=false;notice('Deux styles à la fois. Désélectionne un style pour le remplacer.');return;}
      if(box.checked)selected.add(box.value);else selected.delete(box.value);
      compare.disabled=selected.size!==2;document.querySelector('#selection-count').textContent=`${selected.size} / 2 sélectionnés`;
    });
    compare.onclick=()=>{if(selected.size===2){const [left,right]=selected;location.href=`compare.html?left=${left}&right=${right}&item=${item}`;}};
    picker.onchange=()=>{item=picker.value;document.querySelectorAll('.concept-card').forEach(card=>{const id=card.dataset.style;card.querySelector('iframe').src=`prototype.html?style=${id}&preview=1&item=${item}`;card.querySelector('a').href=`prototype.html?style=${id}&item=${item}`;});};
  }else{
    const valid=id=>concepts.some(c=>c.id===id);
    const sides=[valid(query.get('left'))?query.get('left'):'tide',valid(query.get('right'))?query.get('right'):'focus'];
    const comparison=document.querySelector('#comparison');
    comparison.innerHTML=sides.map((id,i)=>`<section class="compare-column"><div class="compare-label"><span>${i?'B':'A'}</span><select class="compare-style" aria-label="Style ${i?'B':'A'}">${styleOptions(id)}</select><a class="open-single" href="prototype.html?style=${id}&item=${item}">En contexte ↗</a></div><iframe class="compare-frame" title="Prototype ${id}" src="prototype.html?style=${id}&embedded=1&item=${item}&lang=${language}"></iframe></section>`).join('');
    const send=message=>document.querySelectorAll('.compare-frame').forEach(frame=>frame.contentWindow.postMessage({crabLab:true,...message},'*'));
    const picker=document.querySelector('#compare-item');picker.innerHTML=options(item);
    picker.onchange=()=>{item=picker.value;send({item});updateLinks();};
    document.querySelector('#compare-language').onclick=()=>{language=language==='en'?'fr':'en';send({language});updateLinks();};
    document.querySelector('#compare-reset').onclick=()=>send({reset:true});
    function updateLinks(){document.querySelectorAll('.compare-column').forEach((column,i)=>column.querySelector('.open-single').href=`prototype.html?style=${sides[i]}&item=${item}&lang=${language}`);const q=new URLSearchParams({left:sides[0],right:sides[1],item,lang:language});history.replaceState(null,'',`?${q}`);}
    document.querySelectorAll('.compare-style').forEach((select,i)=>select.onchange=()=>{sides[i]=select.value;const column=select.closest('.compare-column');column.querySelector('iframe').src=`prototype.html?style=${select.value}&embedded=1&item=${item}&lang=${language}`;column.querySelector('iframe').title=`Prototype ${select.value}`;updateLinks();});
  }
})();
