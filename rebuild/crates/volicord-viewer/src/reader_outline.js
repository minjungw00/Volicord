// Sole owner of contents disclosure, available-width policy and passive cue.
function createReaderOutline({lifetime,navigate,closeDrawer}) {
  const ko=document.documentElement.lang==='ko',outlineScroll=new WeakMap();
  function updateCue() {
    document.querySelectorAll('.document-nav').forEach(toc => {
      const links=[...toc.querySelectorAll('.toc-primary a')];
      const clearance=toc.getBoundingClientRect().bottom+16;
      let current=links[0];
      for(const link of links){const node=document.getElementById(link.hash.slice(1));if(node&&node.getBoundingClientRect().top<=clearance)current=link;}
      const article=toc.closest('.doc');
      if(article&&article.getBoundingClientRect().bottom<=innerHeight+2)current=links.at(-1);
      for(const link of links){if(link===current)link.setAttribute('aria-current','location');else link.removeAttribute('aria-current');}
      const cue=toc.querySelector('.toc-cue');
      if(cue)cue.textContent=current?.querySelector('span:last-child')?.textContent||(ko?'이 문서의 내용':'On this page');
      const count=toc.querySelector('.toc-position');if(count)count.textContent=links.length?`${links.indexOf(current)+1} / ${links.length}`:'';
    });
  }
  lifetime.on(document,'click',event=>{
    const plainLink=event.button===0&&!event.ctrlKey&&!event.metaKey&&!event.shiftKey&&!event.altKey;
    const outlineTrigger = event.target.closest('.document-nav > summary');
    if (outlineTrigger) {
      const toc = outlineTrigger.parentElement;
      if (!toc.open) outlineScroll.set(toc, scrollY);
      else {
        const offset = outlineScroll.get(toc);
        lifetime.frame(() => { if (offset !== undefined) window.scrollTo(0, offset); });
      }
    }
    const outlineLink = event.target.closest('.document-nav a');
    if (outlineLink && plainLink) {
      const toc = outlineLink.closest('.document-nav');
      toc.open = false;
      event.preventDefault();navigate(outlineLink.hash);
      const target=document.getElementById(outlineLink.hash.slice(1));
      lifetime.frame(()=>{if(target){target.focus({preventScroll:true});target.scrollIntoView({block:'start'});}});
    }
  });
  lifetime.on(document,'keydown', event => {
    if(event.defaultPrevented)return;
    const toc=event.target.closest('.document-nav[open]');
    if(event.key==='Escape'&&toc){event.preventDefault();toc.open=false;toc.querySelector('summary').focus({preventScroll:true});const offset=outlineScroll.get(toc);if(offset!==undefined)window.scrollTo(0,offset);}
  });
  let scheduled = false;
  lifetime.on(window,'scroll', () => {
    if (!scheduled) { scheduled = true; lifetime.frame(() => { scheduled = false; updateCue(); }); }
  }, {passive:true});
 // Desktop shows a single row only when every full title fits. Many/long
 // headings use the same complete dropdown instead of clipping or omission.
 for(const toc of document.querySelectorAll('.document-nav')){
  const article=toc.closest('.doc'),links=[...toc.querySelectorAll('.toc-primary a')];
  let observedWidth=null;
  function fit(force=false){
   const width=article.getBoundingClientRect().width;
   if(width===observedWidth&&force!==true)return;observedWidth=width;
   if(width<730){toc.dataset.compact='true';return;}
   toc.removeAttribute('data-compact');delete toc.dataset.overflow;
   const items=[...toc.querySelectorAll('li')];
   if(items.length>1&&items.some(n=>n.offsetTop!==items[0].offsetTop))toc.dataset.overflow='true';
  }
  lifetime.observe(article,fit);document.fonts.ready.then(()=>{if(!lifetime.disposed)fit(true);});
  lifetime.on(toc,'toggle',()=>{if(toc.open)closeDrawer();});

 }
  return {updateCue,close(){document.querySelectorAll('.document-nav[open]').forEach(n=>n.open=false);}};
}
