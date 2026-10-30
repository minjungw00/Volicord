// Frozen local exports only. No fetch, Runtime access, storage, authoring or authority simulation.
(() => {
  'use strict';
  const memories=new Map();
  let current=null;
  for(const figure of document.querySelectorAll('.spatial-figure')) {
    const viewport=figure.querySelector('.spatial-viewport'),svg=viewport.querySelector('svg');
    const nav=figure.querySelector('.spatial-navigation');
    nav.querySelector('.spatial-enhancement').hidden=false;
    const cameraHelp=figure.querySelector('.spatial-camera-help');
    if(cameraHelp){cameraHelp.hidden=false;figure.querySelector('.spatial-native-help').hidden=true;}
    const w=Number(svg.dataset.worldWidth),h=Number(svg.dataset.worldHeight);
    let scale=1,selected=null,drag=null,expanded=false;
    const strip=document.createElement('div');strip.className='spatial-selection';nav.after(strip);
    function bounded(){viewport.scrollLeft=Math.max(0,Math.min(viewport.scrollLeft,viewport.scrollWidth-viewport.clientWidth));viewport.scrollTop=Math.max(0,Math.min(viewport.scrollTop,viewport.scrollHeight-viewport.clientHeight));}
    function zoom(next){const x=(viewport.scrollLeft+viewport.clientWidth/2)/scale,y=(viewport.scrollTop+viewport.clientHeight/2)/scale;
      scale=Math.max(.15,Math.min(2,next));svg.style.width=w*scale+'px';svg.style.height=h*scale+'px';
      viewport.scrollLeft=x*scale-viewport.clientWidth/2;viewport.scrollTop=y*scale-viewport.clientHeight/2;bounded();
      nav.querySelector('output').textContent=Math.round(scale*100)+'%'+(scale<.75?' · 위치 확인용':'');
    }
    function focusElement(a,move=true,pageMove=false){
      selected?.removeAttribute('data-world-selected');selected=a;current=state;a.dataset.worldSelected='true';
      strip.replaceChildren();const text=document.createElement('span');text.textContent=a.getAttribute('aria-label')+' ';
      const link=document.createElement('a');link.href=a.getAttribute('href');link.textContent='주장과 근거 읽기';link.dataset.worldOrigin=a.id;strip.append(text,link);
      if(move){const b=(a.querySelector('text')||a).getBBox();viewport.scrollLeft=(b.x+b.width/2)*scale-viewport.clientWidth/2;viewport.scrollTop=(b.y+b.height/2)*scale-viewport.clientHeight/2;bounded();}
      if(pageMove)viewport.scrollIntoView({block:'center'});
    }
    function expand(value){expanded=value;figure.classList.toggle('world-expanded',value);const b=nav.querySelector('[data-world-action="expand"]');b.textContent=value?'닫기 (Esc)':'크게 보기';b.setAttribute('aria-expanded',String(value));bounded();}
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
      if(action==='fit'){zoom(Math.min(viewport.clientWidth/w,viewport.clientHeight/h));viewport.scrollTo(0,0);}
      if(action==='reset'){zoom(1);viewport.scrollTo(0,0);}
      if(action==='focus'&&selected){if(scale<.75)zoom(1);focusElement(selected,true,true);}
      const shifts={left:[-180,0],right:[180,0],up:[0,-160],down:[0,160]};if(shifts[action])viewport.scrollBy(...shifts[action]);
      if(action==='expand')expand(!expanded);
    });
    viewport.addEventListener('keydown',e=>{
      if(e.target!==viewport)return;
      if((e.key==='+'||e.key==='-')&&!e.ctrlKey&&!e.metaKey&&!e.altKey&&document.body.dataset.comparisonNavigation==='camera'){e.preventDefault();zoom(scale+(e.key==='+'?.25:-.25));}
      if(e.key==='Home'&&!e.ctrlKey&&!e.metaKey&&!e.altKey){e.preventDefault();zoom(1);viewport.scrollTo(0,0);}
    });
    // Panning starts only on blank background with the primary mouse. Text and links retain selection.
    if(document.body.dataset.comparisonNavigation==='camera'){
      viewport.addEventListener('pointerdown',e=>{if(e.button||e.pointerType!=='mouse'||e.target.closest('a,text'))return;drag={id:e.pointerId,x:e.clientX,y:e.clientY,left:viewport.scrollLeft,top:viewport.scrollTop};viewport.setPointerCapture(e.pointerId);});
      viewport.addEventListener('pointermove',e=>{if(!drag||drag.id!==e.pointerId)return;viewport.scrollLeft=drag.left+drag.x-e.clientX;viewport.scrollTop=drag.top+drag.y-e.clientY;});
      const end=()=>{drag=null;};viewport.addEventListener('pointerup',end);viewport.addEventListener('pointercancel',end);
    }
    let size=[innerWidth,innerHeight];
    function resized(){if(!viewport.clientWidth||!viewport.clientHeight)return;const next=[innerWidth,innerHeight];if(selected&&(next[0]!==size[0]||next[1]!==size[1]))focusElement(selected);size=next;bounded();}
    if(window.ResizeObserver)new ResizeObserver(resized).observe(viewport);
    else window.addEventListener('resize',resized);
    document.addEventListener('keydown',e=>{if(e.key==='Escape'&&expanded&&!document.querySelector('.source-point:target,.source-origin-target:target')){expand(false);nav.querySelector('[data-world-action="expand"]').focus();}});
  }
  function returnTo(id){const m=memories.get(id);if(!m)return;const a=document.getElementById(id);const figure=a?.closest('.spatial-figure');if(!figure)return;requestAnimationFrame(()=>{location.hash=id;current?.restore(m);});}
  document.addEventListener('click',e=>{
    const link=e.target.closest('a[href^="#"]');if(!link||e.ctrlKey||e.metaKey||e.shiftKey||e.altKey)return;
    const id=decodeURIComponent(link.getAttribute('href').slice(1));if(memories.has(id))returnTo(id);
  });
  document.addEventListener('keydown',e=>{
    if(e.key!=='Escape'||!current?.selected)return;
    if(document.querySelector('.source-point:target,.source-origin-target:target,.shared-source-target:target')){const id=current.selected.id;location.hash=id;returnTo(id);}
  });
})();
