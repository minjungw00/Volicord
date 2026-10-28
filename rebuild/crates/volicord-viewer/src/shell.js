// Owns the project drawer only; contents and modal authority are injected.
function createReaderShell({lifetime,modal,contents}) {
 const shell=document.querySelector('.project-shell');if(!shell)return;
 const rail=shell.querySelector('.project-rail'),trigger=shell.querySelector(':scope > summary');
 const backdrop=document.querySelector('.nav-backdrop');
 const mobile=matchMedia('(max-width:980px)'),ko=document.documentElement.lang==='ko';
 let previousFocus=null,updating=false;
 const closeContents=()=>contents.close();
 function sync(restore=false){
  if(updating)return;updating=true;
  const open=mobile.matches&&shell.open;
  document.body.dataset.navOpen=String(open);backdrop.hidden=!open;
  const closeControl=rail.querySelector('.rail-close');closeControl.hidden=!mobile.matches;
  if(open){
   closeContents();previousFocus=previousFocus||document.activeElement;
   modal.set('drawer',rail,()=>close());
   trigger.setAttribute('aria-expanded','true');closeControl.focus({preventScroll:true});
  }else{
   modal.set('drawer',null);
   trigger.setAttribute('aria-expanded','false');
   if(restore&&previousFocus?.isConnected){(mobile.matches?trigger:rail.querySelector('.brand')).focus({preventScroll:true});}
   previousFocus=null;
  }
  updating=false;
 }
 function close(restore=true){shell.open=!mobile.matches;sync(restore);}
 shell.open=!mobile.matches;sync();
 lifetime.on(shell,'toggle',()=>sync());
 lifetime.on(trigger,'click',e=>{if(!shell.open)previousFocus=trigger;});
 lifetime.on(rail.querySelector('.rail-close'),'click',()=>close());
 lifetime.on(backdrop,'click',()=>close());
 lifetime.on(shell,'click',e=>{const a=e.target.closest('a[href]');if(a&&e.button===0&&!e.metaKey&&!e.ctrlKey&&!e.shiftKey&&!e.altKey&&mobile.matches)close(false);});
 lifetime.on(mobile,'change',()=>close(true));
 lifetime.cleanup(()=>{shell.open=!mobile.matches;delete document.body.dataset.navOpen;backdrop.hidden=true;});
 if(document.body.dataset.viewerMode==='snapshot'){
  const update=()=>{
   const target=document.getElementById(location.hash.slice(1).split('!')[0]);
   const section=target?.closest('.snapshot-page');
   const destination=section?.id==='repository-explanation'?'repository':section?.id==='overview'?'overview':'work';
   document.querySelectorAll('.project-navigation a').forEach(a=>{if(a.dataset.destination===destination)a.setAttribute('aria-current','page');else a.removeAttribute('aria-current');});
  };lifetime.on(window,'hashchange',update);update();
 }
 return {closeForContents(){if(mobile.matches&&shell.open)close(false);}};
}
