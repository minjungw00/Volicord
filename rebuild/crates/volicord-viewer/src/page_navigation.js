// Progressive live-page navigation over the existing authoritative GET reader.
// Content is never cached or generated here; native links remain the fallback.
function createPageNavigation({startReader}) {
  if(document.body.dataset.viewerMode!=='live'||typeof fetch!=='function'||typeof DOMParser!=='function'){startReader(null);return;}
  const lifetime=createReaderLifetime();
  const bundleNode=document.querySelector('script[data-viewer-reading]');
  const bundle=bundleNode?.textContent;
  const route=url=>url.pathname+url.search;
  let currentRoute=route(location),sequence=0,pending=null,disposeReader=null,loading=false;
  const state=()=>history.state&&typeof history.state==='object'?history.state:{};
  function eligible(url){return url.origin===location.origin&&url.pathname==='/'&&!url.searchParams.has('guarded');}
  function announce(text){
    let node=document.getElementById('page-navigation-status');
    if(!node){node=document.createElement('p');node.id='page-navigation-status';node.className='sr-only';node.setAttribute('role','status');node.setAttribute('aria-live','polite');document.body.append(node);}
    node.textContent=text;
  }
  function labels(url){
    const ko=(url.searchParams.get('locale')||document.documentElement.lang)==='ko';
    const view=url.searchParams.get('view')||'overview';
    const name=view==='overview'?(ko?'개요':'Overview'):view==='repository'?(ko?'저장소 설명':'Repository explanation'):(ko?'작업':'Work');
    return {ko,name,view};
  }
  function saveScroll(){
    if(!loading&&route(location)===currentRoute)history.replaceState({...state(),viewerPageScroll:{x:scrollX,y:scrollY}},'');
  }
  function showLoading(url){
    const {ko,name,view}=labels(url),main=document.getElementById('viewer-content');
    if(!main)throw Error('missing reader surface');
    document.body.dataset.pageLoading='true';document.body.dataset.view=view;
    document.querySelectorAll('.project-navigation a').forEach(a=>{if(a.dataset.destination===view)a.setAttribute('aria-current','page');else a.removeAttribute('aria-current');});
    const mobile=document.querySelector('.mobile-location');if(mobile)mobile.textContent=name;
    main.replaceChildren();main.setAttribute('aria-busy','true');main.tabIndex=-1;
    const section=document.createElement('section');section.className='page-loading';section.lang=ko?'ko':'en';
    const title=document.createElement('h1');title.textContent=name;
    const message=document.createElement('p');message.className='page-loading-message';
    const spinner=document.createElement('span');spinner.className='page-loading-spinner';spinner.setAttribute('aria-hidden','true');
    message.append(spinner,document.createTextNode(ko?'내용을 불러오는 중입니다.':'Loading content…'));
    const skeleton=document.createElement('div');skeleton.className='page-loading-skeleton';skeleton.setAttribute('aria-hidden','true');
    for(let i=0;i<3;i++)skeleton.append(document.createElement('span'));
    section.append(title,message,skeleton);main.append(section);window.scrollTo(0,0);main.focus({preventScroll:true});
    document.title=name+' · Volicord';announce(name+' · '+(ko?'불러오는 중':'Loading'));
  }
  function showError(url){
    const main=document.getElementById('viewer-content'),{ko}=labels(url);if(!main)return;
    main.replaceChildren();main.setAttribute('aria-busy','false');delete document.body.dataset.pageLoading;
    const section=document.createElement('section');section.className='page-navigation-error';section.lang=ko?'ko':'en';
    const title=document.createElement('h1');title.textContent=ko?'페이지를 불러오지 못했습니다':'Could not load this page';
    const message=document.createElement('p');message.textContent=ko?'다시 시도하거나 다른 페이지를 선택하세요.':'Try again or choose another page.';
    const retry=document.createElement('button');retry.type='button';retry.className='button';retry.dataset.retryPage='true';retry.textContent=ko?'다시 시도':'Try again';
    const native=document.createElement('a');native.className='button';native.href=url.href;native.dataset.nativeNavigation='true';native.textContent=ko?'페이지 새로고침':'Reload page';
    section.append(title,message,retry,native);main.append(section);announce(title.textContent);main.focus({preventScroll:true});
  }
  async function go(destination,{pop=false,retry=false}={}){
    const url=new URL(destination,location.href);if(!eligible(url)){location.assign(url.href);return;}
    const token=++sequence;pending?.abort();pending=new AbortController();const controller=pending;
    const saved=pop?state().viewerPageScroll:null;
    if(!pop&&!retry)saveScroll();
    disposeReader?.();disposeReader=null;
    if(!pop&&!retry)history.pushState({},'',url.href);
    currentRoute=route(url);loading=true;
    try {
      showLoading(url);
      const response=await fetch(url.href,{credentials:'same-origin',cache:'no-store',redirect:'error',signal:controller.signal});
      const html=await response.text();
      if(token!==sequence||controller.signal.aborted)return;
      const parsed=new DOMParser().parseFromString(html,'text/html');
      const script=parsed.querySelector('script[data-viewer-reading]');
      if(script&&script.textContent!==bundle){location.replace(url.href);return;}
      if(!parsed.querySelector('main#viewer-content')||parsed.body.dataset.viewerMode!=='live'||!script||
          !/^[0-9a-f]{32}$/.test(parsed.body.dataset.projectId||'')||
          (url.searchParams.has('project')&&parsed.body.dataset.projectId!==url.searchParams.get('project'))||
          (!response.ok&&response.status!==404))throw Error('unavailable reader response');
      // Reuse only the already CSP-pinned factories. Fetched scripts are never
      // executed, and a different running build uses native document navigation.
      parsed.querySelectorAll('script').forEach(node=>node.remove());
      document.documentElement.lang=parsed.documentElement.lang;
      document.title=parsed.title;
      const selector='style,meta[name="volicord-observation"],meta[name="volicord-observation-unavailable"]';
      document.head.querySelectorAll(selector).forEach(node=>node.remove());
      parsed.head.querySelectorAll(selector).forEach(node=>document.head.append(document.importNode(node,true)));
      document.body.replaceWith(document.adoptNode(parsed.body));
      // Moving the already executed script preserves the current bundle witness;
      // no script from the response is inserted or evaluated.
      document.body.append(bundleNode);
      loading=false;pending=null;
      if(!url.hash)window.scrollTo(0,0);
      disposeReader=startReader(go);
      if(pop&&saved&&Number.isFinite(saved.x)&&Number.isFinite(saved.y)&&saved.x>=0&&saved.y>=0&&saved.y<100000000)window.scrollTo(saved.x,saved.y);
      if(!url.hash){const main=document.getElementById('viewer-content');main.focus({preventScroll:true});}
      announce(labels(url).name+' · '+(labels(url).ko?'불러왔습니다':'Loaded'));
      document.dispatchEvent(new CustomEvent('viewer:navigated',{detail:{url:url.href}}));
    } catch(error){
      if(token!==sequence||controller.signal.aborted)return;
      loading=false;pending=null;showError(url);
    }
  }
  lifetime.on(document,'click',event=>{
    if(event.defaultPrevented||event.button!==0||event.metaKey||event.ctrlKey||event.shiftKey||event.altKey)return;
    if(event.target.closest('button[data-retry-page]')){go(location.href,{retry:true});return;}
    const link=event.target.closest('a[href]');
    if(!link||link.hasAttribute('download')||link.dataset.nativeNavigation||link.target&&link.target!=='_self')return;
    const url=new URL(link.href,location.href);
    if(!eligible(url)||(route(url)===route(location)&&url.hash))return;
    event.preventDefault();go(url.href);
  });
  lifetime.on(document,'submit',event=>{
    const form=event.target;if(event.defaultPrevented||form.method.toLowerCase()!=='get'||form.target&&form.target!=='_self')return;
    const url=new URL(form.action,location.href);if(!eligible(url))return;
    const values=new FormData(form);if(event.submitter?.name)values.append(event.submitter.name,event.submitter.value);
    if([...values.values()].some(value=>typeof value!=='string'))return;
    url.search=new URLSearchParams(values).toString();event.preventDefault();go(url.href);
  });
  lifetime.on(document,'change',event=>{const select=event.target.closest('select[data-language-picker]');if(select?.value)go(select.value);});
  lifetime.on(window,'popstate',()=>{if(route(location)!==currentRoute)go(location.href,{pop:true});});
  let scrollFrame=null;
  lifetime.on(window,'scroll',()=>{if(scrollFrame===null)scrollFrame=lifetime.frame(()=>{scrollFrame=null;saveScroll();});},{passive:true});
  lifetime.on(window,'pagehide',event=>{pending?.abort();disposeReader?.();disposeReader=null;if(!event.persisted)lifetime.dispose();});
  lifetime.on(window,'pageshow',event=>{if(event.persisted)location.reload();});
  disposeReader=startReader(go);
}
