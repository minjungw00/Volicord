// Frozen local exports only. No fetch, Runtime access, storage, authoring or authority simulation.
(() => {
  'use strict';
  const memories=new Map();
  const nativeMemories=new Map();
  const grouped=document.body.classList.contains('grouped-reading');
  if(grouped)document.body.dataset.readingEnhanced='true';
  let modal=null,priorOverflow=document.body.style.overflow;
  let restoring=false;
  const inerted=new Map();
  const backdrop=grouped?document.createElement('button'):null;
  if(backdrop){backdrop.type='button';backdrop.className='reading-backdrop';backdrop.tabIndex=-1;backdrop.hidden=true;backdrop.setAttribute('aria-label','창 닫기');document.body.append(backdrop);backdrop.addEventListener('click',closePanel);}
  let current=null;
  for(const figure of document.querySelectorAll('.spatial-figure')) {
    const viewport=figure.querySelector('.spatial-viewport'),svg=viewport.querySelector('svg');
    const nav=figure.querySelector('.spatial-navigation');
    nav.querySelector('.spatial-enhancement').hidden=false;
    const cameraHelp=figure.querySelector('.spatial-camera-help');
    if(cameraHelp){cameraHelp.hidden=false;figure.querySelector('.spatial-native-help').hidden=true;}
    const w=Number(svg.dataset.worldWidth),h=Number(svg.dataset.worldHeight);
    const overviewStart=figure.classList.contains('work-overview');
    if(overviewStart)figure.dataset.cameraReady='true';
    let overview=overviewStart;
    let scale=1,selected=null,drag=null,expanded=false;
    const strip=document.createElement('div');strip.className='spatial-selection';
    const stage=figure.querySelector('.diagram-stage');if(stage)stage.after(strip);else nav.after(strip);
    function bounded(){viewport.scrollLeft=Math.max(0,Math.min(viewport.scrollLeft,viewport.scrollWidth-viewport.clientWidth));viewport.scrollTop=Math.max(0,Math.min(viewport.scrollTop,viewport.scrollHeight-viewport.clientHeight));}
    function zoom(next){const x=(viewport.scrollLeft+viewport.clientWidth/2)/scale,y=(viewport.scrollTop+viewport.clientHeight/2)/scale;
      overview=false;scale=Math.max(overviewStart ? .03 : .15,Math.min(2,next));svg.style.width=w*scale+'px';svg.style.height=h*scale+'px';
      viewport.scrollLeft=x*scale-viewport.clientWidth/2;viewport.scrollTop=y*scale-viewport.clientHeight/2;bounded();
      const output=nav.querySelector('output');output.textContent=Math.round(scale*100)+'%'+(!overviewStart&&scale<.75?' · 위치 확인용':'');output.title=scale<.75?'전체 위치 확인용 배율':'읽기 배율';
    }
    function fit(){zoom(overviewStart?(viewport.clientWidth-24)/w:Math.min(1,(viewport.clientWidth-6)/w,(viewport.clientHeight-6)/h));viewport.scrollTo(0,0);overview=true;}
    function focusElement(a,move=true,pageMove=false){
      selected?.removeAttribute('data-world-selected');selected=a;current=state;a.dataset.worldSelected='true';
      overview=false;
      strip.replaceChildren();const text=document.createElement('span');text.textContent=a.getAttribute('aria-label')+' ';
      const link=document.createElement('a');link.href=a.getAttribute('href');link.textContent='주장과 근거 읽기';if(overviewStart)link.className='paragraph-evidence-action';link.dataset.worldOrigin=a.id;strip.append(text,link);
      if(move){const b=(a.querySelector('text')||a).getBBox();viewport.scrollLeft=(b.x+b.width/2)*scale-viewport.clientWidth/2;viewport.scrollTop=(b.y+b.height/2)*scale-viewport.clientHeight/2;bounded();}
      if(pageMove)viewport.scrollIntoView({block:'center'});
    }
    function expand(value){expanded=value;figure.classList.toggle('world-expanded',value);const b=nav.querySelector('[data-world-action="expand"]');b.textContent=b.dataset.compactLabel||(value?'닫기 (Esc)':'크게 보기');b.setAttribute('aria-expanded',String(value));bounded();if(overviewStart)syncPanels();}
    figure.closeReadingPanel=()=>expand(false);
    figure.querySelector('[data-panel-close]')?.addEventListener('click',e=>{e.preventDefault();expand(false);nav.querySelector('[data-world-action="expand"]').focus({preventScroll:true});});
    const state={figure,viewport,svg,get selected(){return selected;},save(){return {scale,left:viewport.scrollLeft,top:viewport.scrollTop,y:scrollY,origin:selected?.id,expanded,windowWidth:innerWidth,windowHeight:innerHeight};},restore(m){expand(m.expanded);zoom(m.scale);viewport.scrollLeft=m.left;viewport.scrollTop=m.top;bounded();const resized=m.windowWidth!==innerWidth||m.windowHeight!==innerHeight;const a=document.getElementById(m.origin);if(a){focusElement(a,resized,resized);a.focus({preventScroll:true});}if(!resized)window.scrollTo(0,m.y);}};
    let modifiedReturn=null;
    svg.addEventListener('pointerdown',e=>{if(e.ctrlKey||e.metaKey||e.shiftKey||e.altKey||e.button!==0)modifiedReturn={memory:state.save(),focus:document.activeElement};},true);
    svg.addEventListener('focusin',e=>{const a=e.target.closest('a.diagram-element');if(a&&!modifiedReturn)focusElement(a,false);});
    function finishModified(){if(!modifiedReturn)return;const prior=modifiedReturn;requestAnimationFrame(()=>{viewport.scrollLeft=prior.memory.left;viewport.scrollTop=prior.memory.top;prior.focus?.focus({preventScroll:true});window.scrollTo(0,prior.memory.y);modifiedReturn=null;});}
    svg.addEventListener('click',finishModified);svg.addEventListener('auxclick',finishModified);
    svg.addEventListener('pointercancel',()=>{modifiedReturn=null;});
    svg.addEventListener('click',e=>{const a=e.target.closest('a.diagram-element');if(!a||e.ctrlKey||e.metaKey||e.shiftKey||e.altKey)return;focusElement(a,false);memories.set(a.id,state.save());expand(false);});
    strip.addEventListener('click',e=>{if(e.target.closest('a')&&selected&&!e.ctrlKey&&!e.metaKey&&!e.shiftKey&&!e.altKey){memories.set(selected.id,state.save());expand(false);}});
    nav.addEventListener('click',e=>{
      const target=e.target.closest('[data-world-target]');if(target&&!e.ctrlKey&&!e.metaKey&&!e.shiftKey&&!e.altKey){e.preventDefault();const a=document.getElementById(target.dataset.worldTarget);if(a){target.closest('details').open=false;focusElement(a,true,true);a.focus({preventScroll:true});}return;}
      const button=e.target.closest('[data-world-action]');if(!button)return;
      const action=button.dataset.worldAction;
      if(action==='in'||action==='out')zoom(scale+(action==='in'?.25:-.25));
      if(action==='fit')fit();
      if(action==='reset'){zoom(overviewStart ? .75 : 1);viewport.scrollTo(0,0);}
      if(action==='focus'&&selected){if(scale<.75)zoom(overviewStart ? .75 : 1);focusElement(selected,true,true);}
      const shifts={left:[-180,0],right:[180,0],up:[0,-160],down:[0,160]};if(shifts[action])viewport.scrollBy(...shifts[action]);
      if(action==='expand'){nav.querySelector('details').open=false;expand(!expanded);}
    });
    viewport.addEventListener('keydown',e=>{
      if(e.target!==viewport)return;
      if((e.key==='+'||e.key==='-')&&!e.ctrlKey&&!e.metaKey&&!e.altKey&&document.body.dataset.comparisonNavigation==='camera'){e.preventDefault();zoom(scale+(e.key==='+'?.25:-.25));}
      if(e.key==='Home'&&!e.ctrlKey&&!e.metaKey&&!e.altKey){e.preventDefault();if(overviewStart)fit();else{zoom(1);viewport.scrollTo(0,0);}}
    });
    // Panning starts only on blank background with the primary mouse. Text and links retain selection.
    if(document.body.dataset.comparisonNavigation==='camera'){
      viewport.addEventListener('pointerdown',e=>{if(e.button||e.pointerType!=='mouse'||e.target.closest('a,text'))return;drag={id:e.pointerId,x:e.clientX,y:e.clientY,left:viewport.scrollLeft,top:viewport.scrollTop};viewport.setPointerCapture(e.pointerId);});
      viewport.addEventListener('pointermove',e=>{if(!drag||drag.id!==e.pointerId)return;viewport.scrollLeft=drag.left+drag.x-e.clientX;viewport.scrollTop=drag.top+drag.y-e.clientY;});
      const end=()=>{drag=null;};viewport.addEventListener('pointerup',end);viewport.addEventListener('pointercancel',end);
    }
    let size=[innerWidth,innerHeight];
    function resized(){if(!viewport.clientWidth||!viewport.clientHeight)return;const next=[innerWidth,innerHeight];if(overview)fit();else if(selected&&(next[0]!==size[0]||next[1]!==size[1]))focusElement(selected);size=next;bounded();}
    if(window.ResizeObserver)new ResizeObserver(resized).observe(viewport);
    else window.addEventListener('resize',resized);
    document.addEventListener('keydown',e=>{if(e.key==='Escape'&&expanded&&!document.querySelector('.source-point:target,.source-origin-target:target')){expand(false);nav.querySelector('[data-world-action="expand"]').focus();}});
  }
  for(const figure of document.querySelectorAll('.work-overview:not(.spatial-figure)')){
    const toggle=figure.querySelector('.native-expand-toggle'),viewport=figure.querySelector('.diagram-stage > .diagram-viewport');
    figure.closeReadingPanel=()=>{toggle.checked=false;syncPanels();};
    toggle.addEventListener('change',syncPanels);
    figure.querySelector('[data-panel-close]').addEventListener('click',()=>requestAnimationFrame(()=>{syncPanels();toggle.focus({preventScroll:true});}));
    figure.querySelector('svg').addEventListener('click',e=>{
      const a=e.target.closest('a.diagram-element');if(!a||e.ctrlKey||e.metaKey||e.shiftKey||e.altKey)return;
      nativeMemories.set(a.id,{expanded:toggle.checked,left:viewport.scrollLeft,top:viewport.scrollTop,y:scrollY,width:innerWidth,height:innerHeight});
      toggle.checked=false;syncPanels();
    });
  }
  for(const close of document.querySelectorAll('.work-overview [data-panel-close]')){
    close.tabIndex=0;close.setAttribute('role','button');
    close.addEventListener('keydown',e=>{if(e.key==='Enter'||e.key===' '){e.preventDefault();close.click();}});
  }
  function returnTo(id){
    const a=document.getElementById(id),native=nativeMemories.get(id);
    if(native&&a){requestAnimationFrame(()=>{restoring=true;try{location.hash=id;const figure=a.closest('.work-overview'),viewport=figure.querySelector('.diagram-stage > .diagram-viewport');figure.querySelector('.native-expand-toggle').checked=native.expanded;syncPanels();viewport.scrollLeft=native.left;viewport.scrollTop=native.top;a.focus({preventScroll:true});if(native.width===innerWidth&&native.height===innerHeight)window.scrollTo(0,native.y);else a.scrollIntoView({block:'center',inline:'center'});}finally{restoring=false;}});return;}
    const m=memories.get(id);if(!m)return;const figure=a?.closest('.spatial-figure');if(!figure)return;requestAnimationFrame(()=>{restoring=true;try{location.hash=id;current?.restore(m);}finally{restoring=false;}});
  }
  document.addEventListener('click',e=>{
    const link=e.target.closest('a[href^="#"]');if(!link||e.ctrlKey||e.metaKey||e.shiftKey||e.altKey)return;
    const id=decodeURIComponent(link.getAttribute('href').slice(1));if(memories.has(id)||nativeMemories.has(id))returnTo(id);
    const toc=link.closest('.doc[data-work-layout="grouped"] .document-nav');
    if(toc){
      toc.open=false;
      for(let target=document.getElementById(id);target;target=target.parentElement)if(target.tagName==='DETAILS')target.open=true;
    }
  });
  document.addEventListener('change',e=>{
    const select=e.target.closest('.evidence-range-select');if(!select)return;
    const next=document.getElementById(decodeURIComponent(select.value.slice(1)));
    if(!next||!next.closest('.appendix'))return;
    for(const code of document.querySelectorAll('.appendix > .cited-code[open]'))code.open=false;
    location.hash=select.value;
  });
  function syncPanels(){
    if(!grouped)return;
    const target=document.getElementById(decodeURIComponent(location.hash.slice(1).split('!')[0]));
    const appendix=target?.closest('.appendix');
    if(appendix)for(const figure of document.querySelectorAll('.work-overview.world-expanded,.work-overview:has(.native-expand-toggle:checked)'))figure.closeReadingPanel?.();
    const figure=document.querySelector('.work-overview.world-expanded,.work-overview:has(.native-expand-toggle:checked)');
    const next=appendix&&innerWidth<1200?appendix:figure;
    if(next!==modal){
      for(const [node,prior] of inerted){node.inert=prior;}inerted.clear();
      modal?.removeAttribute('aria-modal');if(modal?.matches('.work-overview'))modal.removeAttribute('role');
      modal=next;
      if(modal){
        modal.setAttribute('role','dialog');modal.setAttribute('aria-modal','true');modal.setAttribute('aria-label',modal===appendix?'근거':'도식 크게 보기');
        for(let branch=modal;branch.parentElement;branch=branch.parentElement)for(const sibling of branch.parentElement.children)if(sibling!==branch&&sibling!==backdrop){inerted.set(sibling,sibling.inert);sibling.inert=true;}
        if(!restoring)requestAnimationFrame(()=>{const close=[...modal?.querySelectorAll('.panel-close')||[]].find(n=>n.getClientRects().length);close?.focus({preventScroll:true});});
      }
      document.body.style.overflow=modal?'hidden':priorOverflow;
      backdrop.hidden=!modal;
    }
    if(appendix&&!next){appendix.setAttribute('role','region');appendix.setAttribute('aria-label','근거');}
    const point=target?.closest('.source-point,.source-origin-target');
    if(point)for(const select of point.querySelectorAll(':scope > .paragraph-source-picker select'))select.value='#'+target.id;
  }
  function closePanel(){
    const target=document.querySelector('.appendix :target');
    if(target){const point=target.closest('.source-origin-target,.source-point');point?.querySelector(':scope > .point-bar a.return')?.click();}
    else if(modal){const figure=modal;figure.closeReadingPanel?.();(figure.querySelector('[data-world-action="expand"]')||figure.querySelector('.native-expand-toggle'))?.focus({preventScroll:true});}
  }
  if(grouped){
    for(const nav of document.querySelectorAll('.doc[data-work-layout] .document-nav')){
      const links=[...nav.querySelectorAll('a[href]')];let pending=false;
      function update(){pending=false;let at=0;const threshold=nav.getBoundingClientRect().bottom+32;links.forEach((a,i)=>{const n=document.getElementById(a.hash.slice(1));if(n&&n.getClientRects().length&&n.getBoundingClientRect().top<=threshold)at=i;});links.forEach((a,i)=>{if(i===at)a.setAttribute('aria-current','location');else a.removeAttribute('aria-current');});nav.querySelector('.toc-cue').textContent=links[at].lastElementChild.textContent;nav.querySelector('.toc-position').textContent=(at+1)+' / '+links.length;}
      window.addEventListener('scroll',()=>{if(!pending){pending=true;requestAnimationFrame(update);}},{passive:true});window.addEventListener('hashchange',update);window.addEventListener('load',()=>requestAnimationFrame(update));requestAnimationFrame(update);
      document.addEventListener('click',e=>{if(nav.open&&!nav.contains(e.target))nav.open=false;});
    }
    let lastSource=null;
    window.addEventListener('hashchange',()=>{const target=document.querySelector('.appendix :target');if(target?.id!==lastSource)for(const code of document.querySelectorAll('.appendix > .cited-code[open]'))code.open=false;lastSource=target?.id;syncPanels();});
    window.addEventListener('resize',syncPanels);syncPanels();
    document.addEventListener('keydown',e=>{
      const target=document.querySelector('.appendix :target');
      if(e.key==='Escape'&&(modal||target)){
        e.preventDefault();e.stopImmediatePropagation();
        closePanel();
      }else if(e.key==='Tab'&&modal){
        const controls=[...modal.querySelectorAll('a[href],button,select,input,summary,[tabindex="0"]')].filter(n=>!n.disabled&&!n.closest('[inert]')&&n.getClientRects().length&&getComputedStyle(n).visibility!=='hidden');
        const first=controls[0],last=controls.at(-1);if(first&&(!modal.contains(document.activeElement)||e.shiftKey&&document.activeElement===first||!e.shiftKey&&document.activeElement===last)){e.preventDefault();(e.shiftKey?last:first).focus();}
      }else if(e.key==='Escape'){
        const nav=document.querySelector('.doc[data-work-layout] .document-nav[open]');if(nav){e.preventDefault();nav.open=false;nav.querySelector(':scope > summary').focus({preventScroll:true});}
      }
    },true);
  }
  document.addEventListener('keydown',e=>{
    if(e.key!=='Escape'||!current?.selected)return;
    if(document.querySelector('.source-point:target,.source-origin-target:target,.shared-source-target:target')){const id=current.selected.id;location.hash=id;returnTo(id);}
  });
})();
