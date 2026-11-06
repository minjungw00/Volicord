// World geometry is server-owned. This owner changes only a bounded viewport.
function createReaderFigures({lifetime,modal}) {
  const ko=document.documentElement.lang==='ko',scenes=new Map(),expandedReturns=new WeakMap();
  const label=(en,kr)=>ko?kr:en;
  function anchor(scene,id) {
    const node=id&&document.getElementById(id);
    return node?.closest('svg')===scene.svg?node:null;
  }
  function select(scene,node) {
    if(!node)return;
    scene.selected=node.id;
    scene.svg.querySelectorAll('[data-selected=true]').forEach(n=>n.removeAttribute('data-selected'));
    node.dataset.selected='true';
    const text=document.createElement('span'),link=document.createElement('a');
    text.textContent=node.getAttribute('aria-label');link.href=node.getAttribute('href');
    link.className='diagram-text-link paragraph-evidence-action';link.dataset.element=node.id;link.textContent=label('Read claim and evidence','주장과 근거 읽기');
    scene.selection.replaceChildren(text,link);scene.selection.hidden=false;
  }
  function clamp(scene) {
    const v=scene.viewport;
    v.scrollLeft=Math.max(0,Math.min(v.scrollLeft,v.scrollWidth-v.clientWidth));
    v.scrollTop=Math.max(0,Math.min(v.scrollTop,v.scrollHeight-v.clientHeight));
  }
  function center(scene,node) {
    if(!node)return;
    const box=node.getBoundingClientRect(),view=scene.viewport.getBoundingClientRect();
    scene.viewport.scrollLeft+=(box.left+box.width/2)-(view.left+view.width/2);
    scene.viewport.scrollTop+=(box.top+box.height/2)-(view.top+view.height/2);clamp(scene);
  }
  function scale(scene,next) {
    const v=scene.viewport,old=scene.scale;
    const x=(v.scrollLeft+v.clientWidth/2)/old,y=(v.scrollTop+v.clientHeight/2)/old;
    scene.scale=Math.max(.03,Math.min(2,next));
    scene.figure.dataset.scale=String(scene.scale);
    scene.svg.style.width=scene.width*scene.scale+'px';scene.svg.style.height=scene.height*scene.scale+'px';
    scene.svg.querySelector('.diagram-content')?.setAttribute('transform',`scale(${scene.scale})`);
    v.scrollLeft=x*scene.scale-v.clientWidth/2;v.scrollTop=y*scene.scale-v.clientHeight/2;clamp(scene);
    scene.figure.querySelector('.diagram-scale').textContent=Math.round(scene.scale*100)+'%';
  }
  function fit(scene,all=false) {
    scale(scene,Math.min(1,(scene.viewport.clientWidth-24)/scene.width,all?(scene.viewport.clientHeight-88)/scene.height:1));
    scene.viewport.scrollTo(0,0);scene.entry=!all;
  }
  function inset(scene) {
    const controls=scene.figure.querySelector('.diagram-controls');
    const top=Math.max(76,(controls?.offsetHeight||0)+24)+'px';
    const changed=scene.svg.style.marginTop!==top;scene.svg.style.marginTop=top;return changed;
  }
  function size(scene) {
    const v=scene.viewport;if(!v.clientWidth)return;
    // A restore already accounts for the final viewport. A queued observer
    // from the suspended evidence view must not center over that exact return.
    const changed=inset(scene)||scene.available!==v.clientWidth||scene.viewportHeight!==v.clientHeight;
    scene.available=v.clientWidth;scene.viewportHeight=v.clientHeight;
    if(!changed){clamp(scene);return;}
    if(!scene.spatial) {
      const variant=v.clientWidth>=760?'wide':v.clientWidth>=520?'medium':'narrow';
      if(scene.svg.dataset.variant!==variant){
        scene.svg.dataset.variant=variant;
        scene.width=parseFloat(scene.svg.style.getPropertyValue(`--map-${variant}-width`));
        scene.height=parseFloat(scene.svg.style.getPropertyValue(`--map-${variant}-height`));
        scale(scene,scene.scale);
      }
    }
    if(scene.entry&&scene.spatial)fit(scene);
    else if(scene.selected)center(scene,anchor(scene,scene.selected));
    clamp(scene);
  }
  function capture(figure,element) {
    const s=scenes.get(figure);if(!s)return null;
    if(element&&anchor(s,element))s.selected=element;
    return {publication:figure.dataset.readerPublication,figure:figure.id,view:s.svg.dataset.originNamespace,
      element:s.selected,expanded:figure.classList.contains('diagram-expanded'),scale:s.scale,left:s.viewport.scrollLeft,top:s.viewport.scrollTop,
      layout:s.svg.dataset.layoutKey,width:s.width,height:s.height,available:s.viewport.clientWidth};
  }
  function restore(figure,memory) {
    const s=scenes.get(figure);
    if(!s||!memory||!figure.isConnected||figure.dataset.unavailable==='true'||memory.publication!==figure.dataset.readerPublication||memory.figure!==figure.id||memory.view!==s.svg.dataset.originNamespace)return false;
    if(memory.expanded&&!figure.classList.contains('diagram-expanded')){
      figure.classList.add('diagram-expanded');
      const control=figure.querySelector('.diagram-controls [data-diagram-action=expand]');control.setAttribute('aria-expanded','true');control.textContent='↙';control.setAttribute('aria-label',label('Restore figure','도식 접기'));
      modal.set('figure',figure,()=>closeExpansion(figure));
    }
    inset(s);s.entry=false;s.selected=anchor(s,memory.element)?.id||null;scale(s,memory.scale);
    if(memory.layout===s.svg.dataset.layoutKey&&memory.width===s.width&&memory.height===s.height&&memory.available===s.viewport.clientWidth){s.viewport.scrollLeft=memory.left;s.viewport.scrollTop=memory.top;}
    else center(s,anchor(s,s.selected));clamp(s);
    s.available=s.viewport.clientWidth;s.viewportHeight=s.viewport.clientHeight;return true;
  }
  function closeExpansion(figure) {
    if(!figure)return;
    const s=scenes.get(figure),prior=expandedReturns.get(figure);
    figure.classList.remove('diagram-expanded');modal.set('figure',null);
    const control=figure.querySelector('.diagram-controls [data-diagram-action=expand]');
    if(control){control.setAttribute('aria-expanded','false');control.textContent='↗';control.setAttribute('aria-label',label('Expand figure','도식 크게 보기'));}
    if(s)size(s);
    if(prior){restore(figure,prior.memory);control?.focus({preventScroll:true});window.scrollTo(0,prior.y);expandedReturns.delete(figure);}
  }
  for(const figure of document.querySelectorAll('.admitted-diagram')) {
    const svg=figure.querySelector('svg'),viewport=svg?.closest('.diagram-viewport');if(!svg||!viewport)continue;
    const spatial=svg.classList.contains('spatial-world');
    const controller=new AbortController();
    const scene={figure,svg,viewport,spatial,controller,scale:1,entry:spatial,selected:null,
      width:Number(svg.getAttribute('width')),height:Number(svg.getAttribute('height')),disconnect:null};
    scenes.set(figure,scene);figure.querySelector('.diagram-controls').hidden=false;
    figure.dataset.cameraReady='true';
    scene.selection=document.createElement('div');scene.selection.className='diagram-selection';scene.selection.hidden=true;figure.querySelector('.diagram-full-view').after(scene.selection);
    const menu=figure.querySelector('.diagram-movement>div');
    const pan=document.createElement('div');pan.className='diagram-pan-controls';pan.append(...menu.children);menu.append(pan);
    const list=document.createElement('ul');list.className='diagram-jump-list';
    const seen=new Set();
    const locations=[...svg.querySelectorAll('a.diagram-element[id][aria-label]')];
    if(svg.dataset.sequenceLayout==='timeline')locations.sort((a,b)=>{
      const primary=n=>!!n.querySelector('.sequence-message,.diagram-condition');
      return Number(primary(b))-Number(primary(a))||(primary(a)?a.getBBox().y-b.getBBox().y:0);
    });
    for(const element of locations){
      if(seen.has(element.id))continue;seen.add(element.id);
      const item=document.createElement('li'),link=document.createElement('a');
      link.href='#'+element.id;link.dataset.diagramTarget=element.id;link.textContent=element.getAttribute('aria-label');item.append(link);list.append(item);
    }
    menu.append(list);
    const viewportObserver=lifetime.observe(viewport,()=>size(scene));
    const controlsObserver=lifetime.observe(figure.querySelector('.diagram-controls'),()=>size(scene));
    scene.disconnect=()=>{viewportObserver();controlsObserver();};size(scene);
    const on=(type,listener,options={})=>viewport.addEventListener(type,listener,{...options,signal:controller.signal});
    on('pointerdown',event=>{scene.modified=event.button!==0||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey;},{capture:true});
    for(const type of ['click','auxclick','pointercancel'])on(type,()=>lifetime.frame(()=>{scene.modified=false;}));
    let drag=null;
    on('pointerdown',event=>{
      if(!spatial||event.button!==0||event.pointerType!=='mouse'||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey||event.target.closest('a,text'))return;
      // Background dragging belongs to the viewport. Native selection here can
      // start in adjacent prose and auto-scroll against the captured pointer.
      // Text, links and modified gestures keep their ordinary browser behavior.
      event.preventDefault();
      drag={id:event.pointerId,x:event.clientX,y:event.clientY,left:viewport.scrollLeft,top:viewport.scrollTop};
      viewport.setPointerCapture(event.pointerId);scene.entry=false;
    });
    on('pointermove',event=>{
      if(!drag||event.pointerId!==drag.id)return;
      viewport.scrollLeft=drag.left+drag.x-event.clientX;viewport.scrollTop=drag.top+drag.y-event.clientY;
    });
    for(const type of ['pointerup','pointercancel','lostpointercapture'])on(type,()=>{drag=null;});
    on('wheel',()=>{scene.entry=false;},{passive:true});
    on('pointerdown',()=>{scene.entry=false;},{passive:true});
    on('keydown',event=>{
      if(event.target!==viewport||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey)return;
      if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','PageUp','PageDown'].includes(event.key))scene.entry=false;
      if(event.key==='+'||event.key==='-'){event.preventDefault();scene.entry=false;scale(scene,scene.scale+(event.key==='+'?.25:-.25));}
      if(event.key==='Home'){event.preventDefault();fit(scene);}
      // Ordinary native arrow/page scrolling remains owned by the browser.
    });
  }
  lifetime.on(document,'click',event=>{
    const close=event.target.closest('[data-figure-close]');if(close){closeExpansion(close.closest('.admitted-diagram'));return;}
    const start=event.target.closest('a[data-diagram-target]');
    if(start&&event.button===0&&!event.ctrlKey&&!event.metaKey&&!event.altKey&&!event.shiftKey){
      const scene=scenes.get(start.closest('.admitted-diagram'));if(!scene)return;
      const node=anchor(scene,start.dataset.diagramTarget);if(!node)return;event.preventDefault();scene.entry=false;
      start.closest('details').open=false;select(scene,node);
      if(scene.scale<.75)scale(scene,.75);center(scene,node);node.focus({preventScroll:true});return;
    }
    const control=event.target.closest('button[data-diagram-action]');if(!control)return;
    const figure=control.closest('.admitted-diagram'),scene=scenes.get(figure);if(!scene)return;
    const action=control.dataset.diagramAction;scene.entry=false;
    if(action==='expand'){
      if(figure.classList.contains('diagram-expanded'))closeExpansion(figure);
      else {
        expandedReturns.set(figure,{memory:capture(figure),y:scrollY});figure.classList.add('diagram-expanded');
        control.setAttribute('aria-expanded','true');control.textContent='↙';control.setAttribute('aria-label',label('Restore figure','도식 접기'));
        modal.set('figure',figure,()=>closeExpansion(figure));size(scene);
      }
    }else if(action==='fit')fit(scene);
    else if(action==='reset'){scale(scene,.75);scene.viewport.scrollTo(0,0);}
    else if(action==='in'||action==='out')scale(scene,scene.scale+(action==='in'?.25:-.25));
    else if(action==='focus'){if(scene.scale<.75)scale(scene,.75);center(scene,anchor(scene,scene.selected));}
    else {const delta={left:[-180,0],right:[180,0],up:[0,-160],down:[0,160]}[action];if(delta)scene.viewport.scrollBy(...delta);}
  });
  lifetime.on(document,'focusin',event=>{
    const element=event.target.closest?.('a.diagram-element'),scene=scenes.get(element?.closest('.admitted-diagram'));
    if(scene&&!scene.modified)select(scene,element);
  });
  lifetime.cleanup(()=>{
    for(const [figure,s] of scenes){
      s.controller.abort();s.disconnect();figure.classList.remove('diagram-expanded');figure.querySelector('.diagram-controls').hidden=true;
      s.svg.style.removeProperty('margin-top');s.svg.style.removeProperty('width');s.svg.style.removeProperty('height');s.svg.querySelector('.diagram-content')?.removeAttribute('transform');
      const menu=figure.querySelector('.diagram-movement>div'),pan=menu.querySelector('.diagram-pan-controls');if(pan){menu.append(...pan.children);pan.remove();}menu.querySelector('.diagram-jump-list')?.remove();
      s.selection.remove();
      delete figure.dataset.scale;delete figure.dataset.cameraReady;
    }scenes.clear();
  });
  return {capture,restore,closeExpansion,suspend(figure){
    if(!figure?.classList.contains('diagram-expanded'))return;
    figure.classList.remove('diagram-expanded');modal.set('figure',null);
    const control=figure.querySelector('.diagram-controls [data-diagram-action=expand]');control.setAttribute('aria-expanded','false');control.textContent='↗';control.setAttribute('aria-label',label('Expand figure','도식 크게 보기'));
  },forgetPublication(publication){
    for(const [figure,s] of scenes)if(figure.dataset.readerPublication===publication){s.controller.abort();s.disconnect();
      for(const node of s.svg.querySelectorAll('[id]')){const origin=document.createElement('span');origin.id=node.id;origin.tabIndex=-1;figure.append(origin);}
      s.svg.replaceChildren();s.svg.removeAttribute('aria-label');s.svg.removeAttribute('title');scenes.delete(figure);expandedReturns.delete(figure);if(figure.classList.contains('diagram-expanded'))modal.set('figure',null);}
  }};
}
