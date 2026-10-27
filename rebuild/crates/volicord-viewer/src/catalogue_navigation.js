// Ephemeral UI coordinates only. Content and authority always come from GET.
(() => {
 const catalogue=document.querySelector('#works[data-catalogue-route]');
 if(!catalogue)return;
 const route=new URL(catalogue.dataset.catalogueRoute,location.href).href;
 const storageKey='volicord:work-list-positions';
 const state=()=>history.state&&typeof history.state==='object'?history.state:{};
 const rows=()=>[...catalogue.querySelectorAll('.work-row')];
 const shape=value=>value&&typeof value.route==='string'&&value.route.length<4096&&typeof value.id==='string'&&/^row-[0-9a-f]{32}$/.test(value.id)&&Number.isFinite(value.top)&&Math.abs(value.top)<100000&&Number.isFinite(value.y)&&value.y>=0&&value.y<100000000;
 const valid=value=>shape(value)&&value.route===route;
 function positions(){try{const data=JSON.parse(sessionStorage.getItem(storageKey)||'[]');return Array.isArray(data)?data.filter(shape).slice(-24):[];}catch{return [];}}
 function save(row){
  const position={route,id:row.id,top:row.getBoundingClientRect().top,y:scrollY};
  history.replaceState({...state(),workListPosition:position},'');
  try{const saved=positions().filter(p=>p.route!==route);saved.push(position);sessionStorage.setItem(storageKey,JSON.stringify(saved.slice(-24)));}catch{}
 }
 catalogue.addEventListener('click',event=>{
  const row=event.target.closest('a.work-row');
  if(row&&event.button===0&&!event.metaKey&&!event.ctrlKey&&!event.shiftKey&&!event.altKey)save(row);
 });
 function restore(){
  const own=state().workListPosition;
  const target=location.hash.slice(1);
  const saved=valid(own)?own:positions().find(p=>valid(p)&&p.id===target);
  if(!valid(saved))return;
  const row=rows().find(n=>n.id===saved.id);
  if(!row){catalogue.querySelector('[role=status]')?.focus({preventScroll:true});return;}
  const apply=()=>{
   window.scrollTo(0,saved.y);
   window.scrollBy(0,row.getBoundingClientRect().top-saved.top);
   row.focus({preventScroll:true});row.dataset.returnSelected='true';
  };
  Promise.resolve(document.fonts?.ready).then(()=>requestAnimationFrame(()=>requestAnimationFrame(apply)));
 }
 window.addEventListener('pageshow',event=>{
  // Revalidate authority instead of reviving a previous page's cached summaries.
  if(event.persisted){location.reload();return;}
  restore();
 });
 // Fragment history within a restored list keeps its scoped selected row.
 window.addEventListener('popstate',restore);
})();
