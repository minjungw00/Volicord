// Presentation only. Citation reading remains the sole body/permission owner.
function createReaderEvidence({lifetime,navigate,state}) {
  const ko=document.documentElement.lang==='ko',handles=[];
  let width=null;
  function bounds(){const min=innerWidth>=1200?360:320;return [min,Math.max(min,Math.min(960,innerWidth-(innerWidth>=1200?600:48)))];}
  function resize(next){
    const [min,max]=bounds();width=next===null?null:Math.max(min,Math.min(max,next));
    if(width===null)document.body.style.removeProperty('--evidence-width');else document.body.style.setProperty('--evidence-width',width+'px');
    for(const handle of handles){handle.setAttribute('aria-valuemin',min);handle.setAttribute('aria-valuemax',max);handle.setAttribute('aria-valuenow',Math.round(width??(innerWidth>=1200?464:Math.min(540,innerWidth))));}
  }
  for(const appendix of document.querySelectorAll('.appendix')){
    const handle=document.createElement('button');handle.type='button';handle.className='evidence-resizer';handle.setAttribute('role','separator');handle.setAttribute('aria-orientation','vertical');handle.setAttribute('aria-label',ko?'근거 창 너비 조절 · 방향키로 조절 · Home으로 복원':'Evidence width · arrow keys to resize · Home to reset');appendix.prepend(handle);handles.push(handle);
    let drag=null;
    lifetime.on(handle,'pointerdown',event=>{if(event.button!==0)return;event.preventDefault();drag={id:event.pointerId};handle.setPointerCapture(event.pointerId);});
    lifetime.on(handle,'pointermove',event=>{if(drag?.id===event.pointerId)resize(innerWidth-event.clientX);});
    for(const type of ['pointerup','pointercancel','lostpointercapture'])lifetime.on(handle,type,()=>{drag=null;});
    lifetime.on(handle,'dblclick',()=>resize(null));
    lifetime.on(handle,'keydown',event=>{
      if(event.ctrlKey||event.metaKey||event.altKey)return;
      if(!['ArrowLeft','ArrowRight','Home','Enter'].includes(event.key))return;event.preventDefault();
      resize(['Home','Enter'].includes(event.key)?null:(width??appendix.clientWidth)+(event.key==='ArrowLeft'?24:-24));
    });
  }
  lifetime.on(window,'resize',()=>resize(width));resize(null);
  function targets(claim) {
    const action=claim?.querySelector(':scope > .evidence-actions > a[data-citation-targets]');
    let values;try{values=JSON.parse(action?.dataset.citationTargets||'[]');}catch{return [];}
    if(!Array.isArray(values)||values.length>256||values.some(value=>typeof value!=='string'||!value.startsWith('#')))return [];
    const publication=claim.closest('.admitted-diagram')?.dataset.readerPublication;
    return [...new Set(values)].filter(value=>{
      const origin=document.getElementById(value.slice(1)),point=document.getElementById(origin?.dataset.selectedPoint);
      return origin?.matches('.source-origin-target')&&origin.dataset.origin===claim.id&&point?.matches('.source-point')&&point.dataset.readerPublication===publication&&origin.closest('.source-point')===point;
    });
  }
  function show(point,origin) {
    const claim=origin&&document.getElementById(origin),diagram=claim?.matches('.diagram-claim');
    const values=diagram?targets(claim):[...point.querySelectorAll(':scope > .range-selector .point-others a')].map(link=>link.getAttribute('href'));
    point.querySelector(':scope > .evidence-choice-picker')?.remove();
    if(!point.querySelector(':scope > .point-bar .evidence-width-reset')){
      const reset=document.createElement('button');reset.type='button';reset.className='evidence-width-reset';reset.textContent='↔';reset.title=ko?'근거 창 기본 너비':'Default evidence width';reset.setAttribute('aria-label',reset.title);point.querySelector(':scope > .point-bar').append(reset);
    }
    const code=document.getElementById(point.dataset.citedCode);if(code?.matches('.cited-code')&&code.dataset.sourceId===point.dataset.sourceId&&code.dataset.readerPublication===point.dataset.readerPublication)code.open=true;
    if(!values.length)return;
    const label=document.createElement('label');label.className='evidence-choice-picker';label.append(document.createTextNode(ko?'근거 선택 · '+values.length+'개 범위':'Choose evidence · '+values.length+' ranges'));
    const select=document.createElement('select');select.setAttribute('aria-label',ko?'이 설명의 근거 선택':'Choose evidence for this explanation');label.append(select);
    for(const [i,value] of values.entries()){
      const target=document.getElementById(value.slice(1)),selectedPoint=target?.matches('.source-point')?target:target?.closest('.source-point');
      if(!selectedPoint||selectedPoint.dataset.readerPublication!==point.dataset.readerPublication)continue;
      const option=document.createElement('option');option.value=value+(diagram?'!'+origin:'');
      option.textContent=(i+1)+'. '+selectedPoint.querySelector(':scope > .source-path').textContent;option.selected=selectedPoint===point;select.append(option);
    }
    point.querySelector(':scope > .source-path').before(label);
  }
  lifetime.on(document,'click',event=>{if(event.target.closest('.evidence-width-reset'))resize(null);});
  lifetime.on(document,'change',event=>{if(event.target.matches('.evidence-choice-picker select'))navigate(event.target.value,state());});
  lifetime.cleanup(()=>{for(const h of handles)h.remove();document.body.style.removeProperty('--evidence-width');document.querySelectorAll('.evidence-choice-picker,.evidence-width-reset').forEach(n=>n.remove());});
  return {show,targets};
}
