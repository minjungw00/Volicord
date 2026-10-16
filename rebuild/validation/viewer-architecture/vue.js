import {createApp, h, ref, nextTick} from 'vue';
const labels = {purpose:'문제',reported_change:'변경 / 조사',reading_point:'작동 원리',observation:'관측과 범위',comparison:'조건별 결과',next_step:'다음 행동',limits:'남은 한계'};
const SourcePanel = {props:['work','selected','claim','failure'], emits:['return'], setup(p,{emit}) {
  return () => h('aside',{class:'code-pane','aria-label':'설명에 연결된 Source'},[
    h('p',{class:'legend'},'초록색: 설명이 가리키는 보존 범위 · diff 색상 아님'),
    h('p',{class:'selected-claim'},p.claim),h('a',{class:'return',href:'#claim-1',onClick:e=>{e.preventDefault();emit('return');}},'설명으로 돌아가기'),
    ...p.work.sources.map((s,i)=>h('section',{class:'source',id:`source-${i}`,tabindex:-1,hidden:i!==p.selected},[
      h('h2',{class:'source-label'},s.label),h('p',{class:'legend'},s.comparison),
      p.failure ? h('p',{class:'unavailable'},'Source unavailable — 원본 본문을 읽을 수 없습니다. 설명의 identity와 한계를 확인하세요.') :
        h('pre',h('code',s.text.replace(/\n$/,'').split('\n').map((line,n)=>h('span',{class:'line explained'},[h('span',{class:'number'},s.start+n),line]))))
    ]))
  ]);
}};
const Narrative = {props:['work'],emits:['select'],setup(p,{emit}) {return ()=>h('article',{class:'narrative'},[
  ...p.work.units.slice(1).map((u,n)=>h('section',{class:`unit ${u.role}`,id:`claim-${n+1}`,tabindex:-1},[
    u.role==='synopsis'?null:h('h2',labels[u.role]||'설명'),h('p',{class:'copy','data-unit':n+1},u.text),
    h('div',{class:'links'},u.refs.map(i=>h('a',{href:`#source-${i}`,'data-source':`source-${i}`,'data-claim':`claim-${n+1}`,onClick:e=>{e.preventDefault();emit('select',i,n+1);}},p.work.sources[i].label)))
  ])),h('details',[h('summary','원래 지시와 control Goal'),h('p',{class:'goal'},p.work.goal)]),
  h('details',{id:'audit'},[h('summary','출처·상태·출력 identity와 전체 진단'),h('pre',p.work.audit)])
]);}};
async function boot() {
  const response=await fetch('/data.json'); if(!response.ok)throw Error('Source data unavailable');
  const data=await response.json();
  createApp({setup(){
    const key=ref(new URL(location.href).searchParams.get('work')), selected=ref(0),claim=ref('');let origin=null;
    function navigate(k){key.value=k;selected.value=0;claim.value='';document.body.classList.remove('code-focus');scrollTo(0,0);}
    addEventListener('popstate',()=>navigate(new URL(location.href).searchParams.get('work')));
    function go(e,k){e.preventDefault();history.pushState({},'',k?`/?work=${k}`:'/');navigate(k);}
    async function select(i,n){origin={n,y:scrollY};selected.value=i;claim.value=data.find(w=>w.key===key.value).units[n].text;document.body.classList.add('code-focus');await nextTick();const t=document.getElementById(`source-${i}`);t.focus({preventScroll:true});if(innerWidth<=800)t.scrollIntoView();}
    async function back(){document.body.classList.remove('code-focus');await nextTick();if(origin){document.getElementById(`claim-${origin.n}`).focus({preventScroll:true});scrollTo(0,origin.y);}}
    return ()=>{const w=data.find(w=>w.key===key.value);return [
      h('header',[h('nav',[h('a',{href:'/',onClick:e=>go(e,null)},'Work 목록'),h('a',{href:'/?work=b',onClick:e=>go(e,'b')},'Work B'),h('a',{href:'/?work=click',onClick:e=>go(e,'click')},'Click 조사')]),h('p','작성형 비교 자료 · 과거 실행과 현재 상태 구분 · 사용자 검토 대기')]),
      h('main',w?[
        h('p',{class:'eyebrow'},`${w.key} · ${w.status}`),h('h1',w.units[0].text),h('div',{class:'layout'},[h(Narrative,{work:w,onSelect:select}),h(SourcePanel,{work:w,selected:selected.value,claim:claim.value,failure:new URL(location.href).searchParams.has('failure'),onReturn:back})]),
        h('footer','Authored evaluation · Source readback, no new generation or acceptance')
      ]:key.value?[h('h1','Work unavailable'),h('p',{class:'unavailable'},'요청한 Work가 보존 입력에 없습니다.')]:[
        h('p',{class:'eyebrow'},'출처가 보존된 두 작업'),h('h1','무엇을 했고, 코드는 어떻게 동작하나요?'),...data.map(w=>h('article',{class:'work-card'},[h('h2',h('a',{'data-work':w.key,href:`/?work=${w.key}`,onClick:e=>go(e,w.key)},w.units[0].text)),h('p',w.units[1].text),h('p',w.status)]))
      ])
    ];};
  }}).mount('#app');document.documentElement.dataset.ready='true';
}
boot().catch(()=>{document.getElementById('app').textContent='Source data unavailable — 재시도하거나 offline.html을 여세요.';});
