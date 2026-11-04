// World geometry is server-owned. This owner changes only a bounded viewport.
function createReaderFigures({lifetime,modal}) {
  const ko=document.documentElement.lang==='ko',scenes=new Map(),expandedReturns=new WeakMap();
  const label=(en,kr)=>ko?kr:en;
  function anchor(scene,id) {
    const node=id&&document.getElementById(id);
    return node?.closest('svg')===scene.svg?node:null;
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
    scene.figure.querySelector('.diagram-scale').textContent=Math.round(scene.scale*100)+'%'+(scene.scale<.75?label(' · position',' · 위치'):'');
  }
  function fit(scene,all=false) {
    scale(scene,Math.min(1,(scene.viewport.clientWidth-4)/scene.width,all?(scene.viewport.clientHeight-4)/scene.height:1));
    scene.viewport.scrollTo(0,0);scene.entry=!all;
  }
  function size(scene) {
    const v=scene.viewport;if(!v.clientWidth)return;
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
      width:s.width,height:s.height,available:s.viewport.clientWidth};
  }
  function restore(figure,memory) {
    const s=scenes.get(figure);
    if(!s||!memory||!figure.isConnected||figure.dataset.unavailable==='true'||memory.publication!==figure.dataset.readerPublication||memory.figure!==figure.id||memory.view!==s.svg.dataset.originNamespace)return false;
    if(memory.expanded&&!figure.classList.contains('diagram-expanded')){
      figure.classList.add('diagram-expanded');
      const control=figure.querySelector('[data-diagram-action=expand]');control.setAttribute('aria-expanded','true');control.textContent=label('Close (Esc)','닫기 (Esc)');
      modal.set('figure',figure,()=>closeExpansion(figure));
    }
    s.entry=false;s.selected=anchor(s,memory.element)?.id||null;scale(s,memory.scale);
    if(memory.width===s.width&&memory.height===s.height&&memory.available===s.viewport.clientWidth){s.viewport.scrollLeft=memory.left;s.viewport.scrollTop=memory.top;}
    else center(s,anchor(s,s.selected));clamp(s);return true;
  }
  function closeExpansion(figure) {
    if(!figure)return;
    const s=scenes.get(figure),prior=expandedReturns.get(figure);
    figure.classList.remove('diagram-expanded');modal.set('figure',null);
    const control=figure.querySelector('[data-diagram-action=expand]');
    control?.setAttribute('aria-expanded','false');if(control)control.textContent=label('Expand','크게 보기');
    if(s)size(s);
    if(prior){restore(figure,prior.memory);control?.focus({preventScroll:true});window.scrollTo(0,prior.y);expandedReturns.delete(figure);}
  }
  for(const figure of document.querySelectorAll('.admitted-diagram')) {
    const svg=figure.querySelector('svg'),viewport=svg?.closest('.diagram-viewport');if(!svg||!viewport)continue;
    const spatial=svg.classList.contains('spatial-world');
    const scene={figure,svg,viewport,spatial,scale:1,entry:spatial,selected:null,
      width:Number(svg.getAttribute('width')),height:Number(svg.getAttribute('height')),disconnect:null};
    scenes.set(figure,scene);figure.querySelector('.diagram-controls').hidden=false;
    scene.disconnect=lifetime.observe(viewport,()=>size(scene));size(scene);
    let drag=null;
    lifetime.on(viewport,'pointerdown',event=>{
      if(!spatial||event.button!==0||event.pointerType!=='mouse'||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey||event.target.closest('a,text'))return;
      drag={id:event.pointerId,x:event.clientX,y:event.clientY,left:viewport.scrollLeft,top:viewport.scrollTop};
      viewport.setPointerCapture(event.pointerId);scene.entry=false;
    });
    lifetime.on(viewport,'pointermove',event=>{
      if(!drag||event.pointerId!==drag.id)return;
      viewport.scrollLeft=drag.left+drag.x-event.clientX;viewport.scrollTop=drag.top+drag.y-event.clientY;
    });
    for(const type of ['pointerup','pointercancel','lostpointercapture'])lifetime.on(viewport,type,()=>{drag=null;});
    lifetime.on(viewport,'wheel',()=>{scene.entry=false;},{passive:true});
    lifetime.on(viewport,'pointerdown',()=>{scene.entry=false;},{passive:true});
    lifetime.on(viewport,'keydown',event=>{
      if(event.target!==viewport||event.ctrlKey||event.metaKey||event.altKey||event.shiftKey)return;
      if(['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','PageUp','PageDown'].includes(event.key))scene.entry=false;
      if(event.key==='+'||event.key==='-'){event.preventDefault();scene.entry=false;scale(scene,scene.scale+(event.key==='+'?.25:-.25));}
      if(event.key==='Home'){event.preventDefault();fit(scene);}
      // Ordinary native arrow/page scrolling remains owned by the browser.
    });
  }
  lifetime.on(document,'click',event=>{
    const control=event.target.closest('button[data-diagram-action]');if(!control)return;
    const figure=control.closest('.admitted-diagram'),scene=scenes.get(figure);if(!scene)return;
    const action=control.dataset.diagramAction;scene.entry=false;
    if(action==='expand'){
      if(figure.classList.contains('diagram-expanded'))closeExpansion(figure);
      else {
        expandedReturns.set(figure,{memory:capture(figure),y:scrollY});figure.classList.add('diagram-expanded');
        control.setAttribute('aria-expanded','true');control.textContent=label('Close (Esc)','닫기 (Esc)');
        modal.set('figure',figure,()=>closeExpansion(figure));size(scene);
      }
    }else if(action==='fit')fit(scene,true);
    else if(action==='reset'){scale(scene,.75);scene.viewport.scrollTo(0,0);}
    else if(action==='in'||action==='out')scale(scene,scene.scale+(action==='in'?.25:-.25));
    else if(action==='focus'){if(scene.scale<.75)scale(scene,.75);center(scene,anchor(scene,scene.selected));}
    else {const delta={left:[-180,0],right:[180,0],up:[0,-160],down:[0,160]}[action];if(delta)scene.viewport.scrollBy(...delta);}
  });
  lifetime.on(document,'focusin',event=>{
    const element=event.target.closest?.('a.diagram-element'),scene=scenes.get(element?.closest('.admitted-diagram'));
    if(scene)scene.selected=element.id;
  });
  lifetime.cleanup(()=>{
    for(const [figure,s] of scenes){
      s.disconnect();figure.classList.remove('diagram-expanded');figure.querySelector('.diagram-controls').hidden=true;
      s.svg.style.removeProperty('width');s.svg.style.removeProperty('height');s.svg.querySelector('.diagram-content')?.removeAttribute('transform');
      delete figure.dataset.scale;
    }scenes.clear();
  });
  return {capture,restore,closeExpansion,suspend(figure){
    if(!figure?.classList.contains('diagram-expanded'))return;
    figure.classList.remove('diagram-expanded');modal.set('figure',null);
    const control=figure.querySelector('[data-diagram-action=expand]');control.setAttribute('aria-expanded','false');control.textContent=label('Expand','크게 보기');
  },forgetPublication(publication){
    for(const [figure,s] of scenes)if(figure.dataset.readerPublication===publication){s.disconnect();scenes.delete(figure);expandedReturns.delete(figure);if(figure.classList.contains('diagram-expanded'))modal.set('figure',null);}
  }};
}
