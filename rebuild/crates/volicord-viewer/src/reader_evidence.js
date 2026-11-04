// Presentation only. Citation reading remains the sole body/permission owner.
function createReaderEvidence({lifetime,navigate,state}) {
  const ko=document.documentElement.lang==='ko',handles=[];
  let width=null;
  function bounds(){return [280,Math.max(280,innerWidth-(innerWidth>=1200?512:32))];}
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
      if(!['ArrowLeft','ArrowRight','Home'].includes(event.key))return;event.preventDefault();
      resize(event.key==='Home'?null:(width??appendix.clientWidth)+(event.key==='ArrowLeft'?24:-24));
    });
  }
  lifetime.on(window,'resize',()=>resize(width));resize(null);
  function show(point,origin) {
    const claim=origin&&document.getElementById(origin);
    const source=claim?.matches('.diagram-claim')?claim:point;
    const links=[...source.querySelectorAll(claim?'a.evidence-link':':scope > .range-selector .point-others a')];
    point.querySelector(':scope > .evidence-choice-picker')?.remove();
    if(!point.querySelector(':scope > .point-bar .evidence-width-reset')){
      const reset=document.createElement('button');reset.type='button';reset.className='evidence-width-reset';reset.textContent='↔';reset.title=ko?'근거 창 기본 너비':'Default evidence width';reset.setAttribute('aria-label',reset.title);point.querySelector(':scope > .point-bar').append(reset);
    }
    if(links.length<2)return;
    const label=document.createElement('label');label.className='evidence-choice-picker';label.append(document.createTextNode(ko?'근거 선택':'Choose evidence'));
    const select=document.createElement('select');label.append(select);
    for(const link of links){
      const option=document.createElement('option');option.value=link.getAttribute('href')+(claim?'!'+origin:'');
      option.textContent=link.getAttribute('aria-label')||link.textContent;option.selected=option.value===location.hash;select.append(option);
    }
    point.querySelector(':scope > .source-path').after(label);
  }
  lifetime.on(document,'click',event=>{if(event.target.closest('.evidence-width-reset'))resize(null);});
  lifetime.on(document,'change',event=>{if(event.target.matches('.evidence-choice-picker select'))navigate(event.target.value,state());});
  lifetime.cleanup(()=>{for(const h of handles)h.remove();document.body.style.removeProperty('--evidence-width');document.querySelectorAll('.evidence-choice-picker,.evidence-width-reset').forEach(n=>n.remove());});
  return {show};
}
