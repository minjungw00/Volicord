// Project drawer and actual-width document contents. No application data is cached.
(() => {
 const shell=document.querySelector('.project-shell');if(!shell)return;
 const rail=shell.querySelector('.project-rail'),trigger=shell.querySelector(':scope > summary');
 const backdrop=document.querySelector('.nav-backdrop'),main=document.querySelector('main');
 const mobile=matchMedia('(max-width:980px)'),ko=document.documentElement.lang==='ko';
 let previousFocus=null,updating=false;
 const closeContents=()=>document.querySelectorAll('.document-nav[open]').forEach(n=>n.open=false);
 function sync(restore=false){
  if(updating)return;updating=true;
  const open=mobile.matches&&shell.open;
  document.body.dataset.navOpen=String(open);backdrop.hidden=!open;
  const close=rail.querySelector('.rail-close');close.hidden=!mobile.matches;
  if(open){
   closeContents();previousFocus=previousFocus||document.activeElement;
   main.inert=true;rail.setAttribute('role','dialog');rail.setAttribute('aria-modal','true');
   trigger.setAttribute('aria-expanded','true');close.focus({preventScroll:true});
  }else{
   main.inert=false;rail.removeAttribute('role');rail.removeAttribute('aria-modal');
   trigger.setAttribute('aria-expanded','false');
   if(restore&&previousFocus?.isConnected){(mobile.matches?trigger:rail.querySelector('.brand')).focus({preventScroll:true});}
   previousFocus=null;
  }
  updating=false;
 }
 function close(restore=true){shell.open=!mobile.matches;sync(restore);}
 shell.open=!mobile.matches;sync();
 shell.addEventListener('toggle',()=>sync());
 trigger.addEventListener('click',e=>{if(!shell.open)previousFocus=trigger;});
 rail.querySelector('.rail-close').addEventListener('click',()=>close());
 backdrop.addEventListener('click',()=>close());
 shell.addEventListener('click',e=>{const a=e.target.closest('a[href]');if(a&&e.button===0&&!e.metaKey&&!e.ctrlKey&&!e.shiftKey&&!e.altKey&&mobile.matches)close(false);});
 document.addEventListener('keydown',e=>{
  if(!mobile.matches||!shell.open)return;
  if(e.key==='Escape'){e.preventDefault();close();return;}
  if(e.key!=='Tab')return;
  const controls=[...rail.querySelectorAll('a[href],button:not([hidden]),summary,input,select')].filter(n=>n.getClientRects().length);
  const first=controls[0],last=controls.at(-1);
  if(e.shiftKey&&(!rail.contains(document.activeElement)||document.activeElement===first)){e.preventDefault();last?.focus();}
  else if(!e.shiftKey&&(!rail.contains(document.activeElement)||document.activeElement===last)){e.preventDefault();first?.focus();}
 });
 mobile.addEventListener('change',()=>close(true));
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
  new ResizeObserver(fit).observe(article);document.fonts.ready.then(()=>fit(true));
  toc.addEventListener('toggle',()=>{if(toc.open&&mobile.matches&&shell.open)close(false);});
 }
 if(document.body.dataset.viewerMode==='snapshot'){
  const update=()=>{
   const target=document.getElementById(location.hash.slice(1).split('!')[0]);
   const section=target?.closest('.snapshot-page');
   const destination=section?.id==='repository-explanation'?'repository':section?.id==='overview'?'overview':'work';
   document.querySelectorAll('.project-navigation a').forEach(a=>{if(a.dataset.destination===destination)a.setAttribute('aria-current','page');else a.removeAttribute('aria-current');});
  };window.addEventListener('hashchange',update);update();
 }
})();
