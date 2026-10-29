// Presentation and exact same-origin retained citation reads only. No writes.
((lifetime, modal) => {
  document.body.dataset.readerEnhanced='true';
  const diagramReturns = new Map();
  const sourceReturns = new Map();
  let navigationSequence = 0;
  const remember = (map, value) => {
    const token = String(++navigationSequence);
    map.set(token, value);
    if (map.size > 128) map.delete(map.keys().next().value);
    return token;
  };
  const state = () => history.state && typeof history.state === 'object' ? history.state : {};
  function navigate(hash, next = state()) {
    // Native fragment navigation updates :target as well as URL/history.
    // pushState alone leaves CSS targeting a previously selected element.
    if (location.hash !== hash) location.hash = hash;
    history.replaceState(next,'');
    apply(); lifetime.frame(apply);
  }
  const narrow = matchMedia('(max-width:1199px)');
  let selected = null;
  let selectedReturnToken = null;
  let focusedHash = null;
  let lastAppliedHash = null;
  const ko = document.documentElement.lang === 'ko';
  // A deep Source fragment carries its exact originating diagram claim. The
  // native href remains script-free; only JS adds this bounded return selector.
  function fragment() {
    const [id, encodedOrigin, extra] = location.hash.slice(1).split('!');
    let origin=encodedOrigin;
    let target = document.getElementById(id);
    if(target?.matches('.shared-source-target,.source-origin-target')){
      const nativeOrigin=target.dataset.origin;
      const selectedPoint=document.getElementById(target.dataset.selectedPoint);
      if(!selectedPoint?.matches('.source-point')||selectedPoint.dataset.readerPublication!==target.closest('.source-point')?.dataset.readerPublication)return {target:null};
      if(nativeOrigin){if(origin&&origin!==nativeOrigin)return {target:null};origin=nativeOrigin;}
      target=selectedPoint;
    }
    if (extra !== undefined) return {target:null};
    if (origin !== undefined) {
      const claim = document.getElementById(origin);
      if (!target?.matches('.source-point') || !claim?.matches('.diagram-claim') ||
          ![...claim.querySelectorAll('a.evidence-link')].some(a=>a.getAttribute('href')==='#'+id||a.getAttribute('href')+'-shared-code'==='#'+id)) return {target:null};
    }
    return {target, origin};
  }
  function withholdPublication(publication, reason) {
    // A citation failure invalidates all dependent meaning, not just its file.
    // Retain opaque return positions, but no hidden code, prose or alternatives.
    // Return memories contain DOM references; release affected detached nodes
    // as well as the visible regions and the original-code buffers below.
    citations.forgetPublication(publication);
    figures.forgetPublication(publication);
    for(const memory of [diagramReturns,sourceReturns]){
      for(const [token,prior] of memory){
        if([prior.trigger,prior.figure,prior.viewport].some(node=>node?.closest?.('[data-reader-publication]')?.dataset.readerPublication===publication))memory.delete(token);
      }
    }
    for (const region of document.querySelectorAll('[data-reader-publication]')) {
      if (region.dataset.readerPublication !== publication || !region.isConnected) continue;
      region.classList.remove('diagram-expanded');
      const ids=[...region.querySelectorAll('[id]')].map(n=>n.id);
      const back=region.matches('.source-point') ? region.querySelector('a.return')?.cloneNode(true) : null;
      const notice=document.createElement('p'); notice.setAttribute('role','status');
      notice.className='publication-unavailable';
      notice.textContent=reason+' '+(ko?'현재 문서를 다시 여세요.':'Reopen the current document.');
      const placeholders=ids.map(id=>{const n=document.createElement('span');n.id=id;n.tabIndex=-1;return n;});
      region.replaceChildren(...(back?[back]:[]),notice,...placeholders);
      for(const attribute of ['aria-label','aria-labelledby','title'])region.removeAttribute(attribute);
      if(region.matches('.source-point'))region.setAttribute('aria-label',ko?'출판물을 이용할 수 없습니다':'Publication unavailable');
      region.dataset.unavailable='true';
    }
  }
  function updateComparison(publication, basis) {
    const labels = {
      verified_at_read: ko ? '이번 비교에서 로컬 저장소가 작성 당시 스냅샷과 일치했습니다. 원격 최신성은 확인하지 않았습니다.' : 'This local comparison matched the original snapshot. Remote freshness was not checked.',
      changed: ko ? '저장소가 변경되었습니다. 설명은 작성 당시 근거로 보존되며 의미상 영향은 알 수 없습니다.' : 'Repository changed. The explanation retains its original basis; semantic impact is unknown.',
      unavailable: ko ? '현재 저장소 비교를 이용할 수 없습니다. 설명은 작성 당시 근거로 보존됩니다.' : 'Current repository comparison is unavailable. The explanation retains its original basis.',
      not_checked: ko ? '현재 저장소 비교가 확립되지 않았습니다.' : 'No current repository comparison is established.'
    };
    if (!basis || !Object.hasOwn(labels,basis.repository_comparison)) return;
    for(const point of document.querySelectorAll('.source-point')){
      if(point.dataset.readerPublication!==publication)continue;
      const note=point.querySelector('[data-evidence-comparison]');if(note)note.textContent=labels[basis.repository_comparison];
    }
    for (const region of document.querySelectorAll('.applicability')) {
      if (region.dataset.readerPublication !== publication) continue;
      region.dataset.comparisonState = basis.repository_comparison;
      region.querySelector('summary').textContent = labels[basis.repository_comparison];
      const time = region.querySelector('[data-comparison-time]');
      if (time && Number.isSafeInteger(basis.checked_at_unix_micros)) {
        const raw=basis.checked_at_unix_micros,date=new Date(Math.floor(raw/1000));
        const valid=Number.isFinite(date.getTime())&&date.getUTCFullYear()>=1&&date.getUTCFullYear()<=9999;
        time.textContent=(valid?date.toISOString().slice(0,16).replaceAll('-','.').replace('T',' ')+' UTC':(ko?'날짜 확인 불가':'Date unavailable'))+` · Unix µs: ${raw}`;
      }
    }
  }
  let shell=null;
  const outline=createReaderOutline({lifetime,navigate,closeDrawer:()=>shell?.closeForContents()});
  shell=createReaderShell({lifetime,modal,contents:outline});
  const figures=createReaderFigures({lifetime,modal});
  const citations=createCitationReader({lifetime,isSelected:point=>selected===point,
    selectedSource:()=>selected?.dataset.sourceId,withholdPublication,updateComparison});
  const fragmentNotice = document.createElement('p');
  fragmentNotice.className = 'fragment-unavailable';
  fragmentNotice.setAttribute('role', 'status');
  fragmentNotice.hidden = true;
  fragmentNotice.textContent = ko ? '이 문단 링크는 현재 문서에서 사용할 수 없습니다. 목차에서 현재 내용을 선택하세요.'
    : 'This section link is unavailable in the current document. Choose a current section from the TOC.';
  document.querySelector('main')?.prepend(fragmentNotice);
  function apply() {
    const {target,origin:diagramOrigin} = fragment();
    let restoredSource = false;
    const navigationChanged = lastAppliedHash !== location.hash;
    lastAppliedHash = location.hash;
    fragmentNotice.hidden = !location.hash || !!target;
    if (target) {
      if (target.matches('details')) target.open = true;
      for (let parent = target.parentElement; parent; parent = parent.parentElement) {
        if (parent.matches('details')) parent.open = true;
      }
    }
    // A deep evidence/claim link can originate in the disclosed timeline. The
    // claim lives beside both scenes, so reveal the exact mapped visual origin
    // before focus or return; no name-based or publication-wide fallback.
    const originClaim=diagramOrigin?document.getElementById(diagramOrigin):target?.matches('.diagram-claim')?target:null;
    const visualOrigin=originClaim&&document.getElementById(originClaim.dataset.element);
    if(visualOrigin?.closest('.admitted-diagram')===originClaim?.closest('.admitted-diagram')){
      for(let parent=visualOrigin?.parentElement;parent;parent=parent.parentElement){if(parent.matches('details'))parent.open=true;}
    }
    if (navigationChanged && target?.matches('h2,h3')) {
      target.focus({preventScroll:true});lifetime.frame(()=>target.scrollIntoView({block:'start'}));
    }
    const point = target?.closest('.source-point');
    document.body.dataset.sourceOpen=point?'true':'false';
    document.querySelectorAll('.source-point[data-selected],.appendix[data-open],.diagram-claim[data-active]').forEach(n=>{delete n.dataset.selected;delete n.dataset.open;delete n.dataset.active;});
    if (point) {
      point.dataset.selected='true';point.closest('.appendix').dataset.open='true';
      const returnId=diagramOrigin || point.dataset.origin;
      point.querySelector('a.return').setAttribute('href','#'+returnId);
      const claim=document.getElementById(returnId);
      if (claim?.matches('.diagram-claim')) claim.dataset.active='true';
    }
    if(!point&&target?.matches('.diagram-claim'))target.dataset.active='true';
    outline.updateCue();
    const expanded=document.querySelector('.diagram-expanded');
    modal.set('figure',!point?expanded:null,()=>figures.closeExpansion(expanded));
    modal.set('evidence',point&&narrow.matches?point:null,()=>point?.querySelector('a.return')?.click());
    const backdrop=document.querySelector('.evidence-backdrop');if(backdrop)backdrop.hidden=!(point&&narrow.matches);
    document.querySelectorAll('.diagram-element[data-selected]').forEach(n=>delete n.dataset.selected);
    if(diagramOrigin){const element=document.getElementById(document.getElementById(diagramOrigin)?.dataset.element);if(element)element.dataset.selected='true';}
    if (selected && selected !== point) {
      citations.cancel();
    }
    if (point) {
      const changed = focusedHash !== location.hash;
      selected = point;
      selectedReturnToken = state().sourceReturn || null;
      focusedHash = location.hash;
      if (narrow.matches) point.setAttribute('aria-label',point.querySelector('.source-path')?.textContent || (ko?'근거를 사용할 수 없습니다':'Evidence unavailable'));
      const origin = document.getElementById(diagramOrigin || point.dataset.origin);
      const title=point.querySelector('.evidence-claim');
      if(title){const wording=origin?.querySelector('.diagram-selected-label')?.textContent;title.textContent=wording?`${origin.querySelector('h3').textContent}: ${wording}`:point.querySelector('.claim-quote')?.textContent||'';}
      if (changed) citations.read(point);
      if (changed && !narrow.matches && !state().sourceReturn) origin?.scrollIntoView({block:'center'});
      if (!point.contains(document.activeElement)) (point.querySelector('a.return')||point).focus({preventScroll:true});
    } else if (selected) {
      restoredSource = true;
      citations.cancel();
      selected = null;
      focusedHash = null;
      if (target) {
        const sourceReturn = sourceReturns.get(selectedReturnToken || state().sourceBack || state().sourceReturn);
        selectedReturnToken = null;
        const trigger = sourceReturn?.claim === target.id ? sourceReturn.trigger : null;
        const element=target.matches('.diagram-claim')?document.getElementById(target.dataset.element):null;
        const destination = trigger?.isConnected ? trigger : element||target;
        const offset = sourceReturn?.claim === target.id ? sourceReturn.y : undefined;
        const hash = location.hash;
        const restore = () => {
          if (selected || location.hash !== hash || !destination.isConnected) return;
          const active = document.activeElement;
          // Native fragment navigation may focus the paragraph after this
          // event. Restore the activating Source button once more after that
          // navigation, without taking focus from another user-selected control.
          if (active !== target && active !== destination && active !== document.body) return;
          if(sourceReturn?.figure?.isConnected){sourceReturn.figure.scrollLeft=sourceReturn.figureLeft;sourceReturn.figure.scrollTop=sourceReturn.figureTop;}
          if(sourceReturn?.viewport?.isConnected){sourceReturn.viewport.scrollLeft=sourceReturn.left;sourceReturn.viewport.scrollTop=sourceReturn.top;}
          destination.focus({preventScroll:true});
          if (offset !== undefined) {window.scrollTo(0, offset);if(sourceReturn.triggerTop!==undefined)window.scrollBy(0,destination.getBoundingClientRect().top-sourceReturn.triggerTop);}
          else destination.scrollIntoView({block:'center'});
        };
        destination.focus({preventScroll:true});
        if (offset !== undefined) window.scrollTo(0, offset);
        else destination.scrollIntoView({block:'center'});
        lifetime.frame(()=>lifetime.frame(restore));
      }
    }
    if (!point && !restoredSource && navigationChanged && target?.matches('.diagram-claim')) {
      target.dataset.active='true';target.focus({preventScroll:true});target.scrollIntoView({block:'nearest'});
    }
    if (!point && target?.matches('.diagram-element')) {
      const candidate=diagramReturns.get(state().diagramReturn);
      const prior=candidate?.element===target.id?candidate:null;
      if (!navigationChanged && document.activeElement !== target && document.activeElement !== prior?.trigger) return;
      (prior?.trigger?.isConnected?prior.trigger:target).focus({preventScroll:true});
      if (prior) {
        const viewport=target.closest('.diagram-viewport');
        const figure=target.closest('.admitted-diagram');
        const claim=document.getElementById(prior.claim);
        if (claim?.matches('.diagram-claim') && claim.closest('.admitted-diagram')===figure) claim.dataset.active='true';
        figure.scrollLeft=prior.figureLeft;figure.scrollTop=prior.figureTop;
        viewport.scrollLeft=prior.left;viewport.scrollTop=prior.top;
        window.scrollTo(0,prior.y);
        const trigger=prior.trigger?.isConnected?prior.trigger:target;
        window.scrollBy(0,trigger.getBoundingClientRect().top-prior.triggerTop);
      } else target.scrollIntoView({block:'center',inline:'center'});
    }
  }
  lifetime.on(window,'popstate',()=>{
    if (!location.hash) {
      const prior=diagramReturns.get(state().diagramBack);
      if(prior?.trigger?.isConnected) {prior.trigger.focus({preventScroll:true});window.scrollTo(0,prior.y);}
    }
  });
  lifetime.on(document,'focusin',event=>{
    const element=event.target.closest?.('a.diagram-element');
    if(element && diagramReturns.get(state().diagramReturn)?.element !== element.id) element.scrollIntoView({block:'nearest',inline:'nearest'});
  });
  function openEvidence(link, trigger=link, activation=null) {
    outline.close();
    const figure=trigger.closest('.admitted-diagram'),viewport=trigger.closest('.diagram-viewport')||figure?.querySelector('.diagram-viewport');
    const token=remember(sourceReturns,{claim:link.dataset.origin,trigger,y:activation?.y??scrollY,triggerTop:activation?.triggerTop??trigger.getBoundingClientRect().top,figure,figureLeft:activation?.figureLeft??figure?.scrollLeft,figureTop:activation?.figureTop??figure?.scrollTop,viewport,left:activation?.left??viewport?.scrollLeft,top:activation?.top??viewport?.scrollTop});
    const next={...state(),sourceReturn:token};
    history.replaceState({...state(),sourceBack:token},'');
    const hash=link.getAttribute('href')+(link.closest('.diagram-claim')?'!'+link.dataset.origin:'');
    const repeated=location.hash===hash;navigate(hash,next);
    if(repeated&&selected)citations.read(selected);
  }
  lifetime.on(document,'click', event => {
    // Modified links belong to native navigation (new tab/window/download).
    // They must not select, close, focus or scroll this reading context.
    const plainLink = event.button === 0 && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.altKey;
    const diagramLink=event.target.closest('a.diagram-element,a.diagram-text-link');
    if (diagramLink) {
      const id=diagramLink.matches('.diagram-element')?diagramLink.id:diagramLink.dataset.element;
      const element=document.getElementById(id);const viewport=element?.closest('.diagram-viewport');
      if (viewport && plainLink) {
        event.preventDefault();
        const figure=viewport.closest('.admitted-diagram');
        const claim=[...figure.querySelectorAll('.diagram-claim')].find(n=>n.getClientRects().length);
        const token=remember(diagramReturns,{element:id,trigger:diagramLink,y:scrollY,left:viewport.scrollLeft,top:viewport.scrollTop,
          claim:claim?.id || null,figureLeft:figure.scrollLeft,figureTop:figure.scrollTop,
          triggerTop:diagramLink.getBoundingClientRect().top});
        const next={...state(),diagramReturn:token};
        // Both the native Back destination and the new selected claim retain
        // this activation, even if a later activation uses the same element.
        history.replaceState({...state(),diagramBack:token},'');
        navigate(diagramLink.getAttribute('href'),next);
        const chosen=document.getElementById(diagramLink.getAttribute('href').slice(1));
        const evidence=chosen?.querySelector('a.evidence-link');
        if(evidence)openEvidence(evidence,diagramLink.matches('.diagram-route')?element:diagramLink,diagramReturns.get(token));
      }
    }
    const diagramReturn=event.target.closest('a.diagram-return');
    if (diagramReturn && plainLink) {
      event.preventDefault();
      navigate(diagramReturn.getAttribute('href'));
    }
    const button = event.target.closest('button.citation-read');
    if (button) citations.read(button.closest('.source-point'), Number(button.dataset.nextOffset || 0));
    const range=event.target.closest('.point-others a');
    if(range&&plainLink){
      event.preventDefault();range.closest('.range-selector')?.removeAttribute('open');
      const claim=fragment().origin;const scope=claim&&document.getElementById(claim);
      const bound=scope&&[...scope.querySelectorAll('a.evidence-link')].find(a=>document.getElementById(a.hash.slice(1))?.dataset.selectedPoint===range.hash.slice(1)||a.getAttribute('href')===range.getAttribute('href'));
      navigate((bound?bound.getAttribute('href'):range.getAttribute('href'))+(bound?'!'+claim:''));
    }
    const codeReturn=event.target.closest('.source-point a.return');
    if (codeReturn && plainLink) {
      event.preventDefault();navigate(codeReturn.getAttribute('href'));
    }
    const link = event.target.closest('a.evidence-link');
    if (link && plainLink) {
      event.preventDefault();openEvidence(link);
    }
  });
  lifetime.on(document,'keydown',event=>{
    if(!event.defaultPrevented&&event.key==='Escape'&&selected){event.preventDefault();selected.querySelector('a.return')?.click();}
  });
  lifetime.on(window,'hashchange', apply);
  lifetime.on(window,'popstate', () => {apply();lifetime.frame(()=>{if(fragment().target?.matches('.diagram-element')) apply();});});
  lifetime.on(narrow,'change', apply);
  const evidenceBackdrop=document.createElement('button');evidenceBackdrop.className='evidence-backdrop';evidenceBackdrop.type='button';evidenceBackdrop.tabIndex=-1;evidenceBackdrop.hidden=true;evidenceBackdrop.setAttribute('aria-label',ko?'근거 닫기':'Close evidence');document.body.append(evidenceBackdrop);
  lifetime.on(evidenceBackdrop,'click',()=>selected?.querySelector('a.return')?.click());
  lifetime.on(window,'pagehide',event=>{citations.cancel();if(!event.persisted)lifetime.dispose();});
  lifetime.on(window,'pageshow',event=>{if(event.persisted){citations.expire();focusedHash=null;apply();}});
  lifetime.cleanup(()=>{
    const nativeHash=location.hash.split('!')[0];
    if(fragment().target&&nativeHash!==location.hash)location.replace(nativeHash);
    diagramReturns.clear();sourceReturns.clear();evidenceBackdrop.remove();fragmentNotice.remove();
    delete document.body.dataset.readerEnhanced;delete document.body.dataset.sourceOpen;
    document.querySelectorAll('.source-point[data-selected],.appendix[data-open],.diagram-claim[data-active]').forEach(node=>{delete node.dataset.selected;delete node.dataset.open;delete node.dataset.active;});
  });
  apply();
})(readerLifetime, readerModal);
