// Actual browser glyph/path checks. Expected wording comes from the original
// admitted publication, never renderer coordinates or generated SVG attributes.
async function inspectDiagram(svg, diagram, prose) {
  const label = l => l.text;
  const m=diagram.meaning;
  const stages=await svg.getAttribute('data-sequence-layout')==='stages';
  const ko=await svg.evaluate(n=>n.ownerDocument.documentElement.lang==='ko');
  const owner=m.owner && `${ko?'상태 소유자:':'State owner:'} ${label(m.owner)}`;
  // Types may be stated once for a homogeneous scope, but must remain attached
  // in a mixed scope. This oracle starts from admitted types, not SVG metadata.
  const kinds={record:['record','기록'],context_role:['context role','맥락 역할'],reading_group:['read grouping','읽기 묶음'],component:['component','구성요소'],participant:['participant','참여자'],process:['process','처리'],branch:['branch','분기'],initial:['initial','시작'],final:['final','종료'],state:['state','상태'],references:['references','참조'],groups:['groups','묶음'],role_of:['role of','역할'],depends_on:['depends on','의존'],produces:['produces','생성'],data_flow:['data flow','데이터 흐름'],control_flow:['control flow','제어 흐름'],transition:['transition','전이'],call:['call','호출'],return:['return','반환'],await:['await','대기'],state_change:['state change','상태 변경']};
  const expected=(m.nodes||m.participants).flatMap(n=>[{id:n.id,text:label(n.label)},...(n.identifier?[{id:n.id+':identifier',text:label(n.identifier)}]:[])]);
  if(m.owner)expected.push({id:'state:owner',text:label(m.owner),prefix:true});
  if(owner)expected.push(...m.nodes.map(n=>({id:n.id,text:owner})));
  for(const r of m.relations||[]) {
    expected.push({id:r.id,text:label(r.label)});
    if(owner)expected.push({id:r.id,text:owner});
    const endpoint=id=>{const n=m.nodes.find(n=>n.id===id);const name=label(n.label);return n.identifier?`${name} (${label(n.identifier)})`:name;};
    expected.push({id:r.id,text:`${endpoint(r.from)} → ${endpoint(r.to)}`,accessibility:true});
    if(r.condition)expected.push({id:r.id+':condition',text:label(r.condition),prefix:true});
  }
  for(const [i,g] of (m.groups||[]).entries()) {
    const order=g.order==='parallel'?(ko?'par · 상대 순서 없음':'par · no relative order'):(ko?'strict · 순차':'strict · sequential');
    const localOrder=g.order==='sequential'?'strict':order;
    const context=localOrder+(g.condition?` · opt · ${ko?'조건:':'condition:'} ${label(g.condition)}`:'');
    expected.push(...g.messages.map(r=>({id:r.id,text:context,prefix:true})));
    if(g.condition)expected.push({id:'guard:'+(i+1),text:label(g.condition),prefix:true});
    expected.push(...g.messages.map(r=>({id:r.id,text:label(r.label)})));
    const endpoint=id=>{const n=m.participants.find(n=>n.id===id);const name=label(n.label);return m.participants.filter(n=>label(n.label)===name).length>1?`${name} (${label(n.identifier)})`:name;};
    expected.push(...g.messages.flatMap(r=>stages?[{id:r.id,text:endpoint(r.from)},...(r.from===r.to?[]:[{id:r.id,text:endpoint(r.to)}])]:[{id:r.id,text:`${endpoint(r.from)} → ${endpoint(r.to)}`}]));
  }
  const edges=m.relations || (m.groups||[]).flatMap(g=>g.messages);
  return svg.evaluate((root,{expected,edges,sequence,stages,groups,kinds,nodes})=> {
    const errors=[];const texts=[];const paths=[];
    const box=b=>({x:b.x,y:b.y,width:b.width,height:b.height});
    const intersects=(a,b)=>a.x<b.x+b.width-1 && a.x+a.width>b.x+1 && a.y<b.y+b.height-1 && a.y+a.height>b.y+1;
    const rendered=n=>{
      for(let at=n;at;at=at.parentElement){const s=getComputedStyle(at);if(s.display==='none'||s.visibility==='hidden'||Number(s.opacity)<.5)return false;if(at===root)break;}
      return true;
    };
    const pick=(n,selector)=>[...n?.querySelectorAll(selector)||[]].find(rendered);
    const view=root.hasAttribute('viewBox')?root.viewBox.baseVal:{x:0,y:0,width:root.clientWidth,height:root.clientHeight};
    // Use displayed SVG coordinates, including ancestor transforms. Local
    // getBBox()/path coordinates can otherwise certify a visibly moved message.
    const matrix=n=>root.getScreenCTM().inverse().multiply(n.getScreenCTM());
    const point=(n,p)=>new DOMPoint(p.x,p.y).matrixTransform(matrix(n));
    const bounds=n=>{
      const b=n.getBBox(),m=matrix(n);
      const ps=[[b.x,b.y],[b.x+b.width,b.y],[b.x,b.y+b.height],[b.x+b.width,b.y+b.height]].map(([x,y])=>new DOMPoint(x,y).matrixTransform(m));
      const xs=ps.map(p=>p.x),ys=ps.map(p=>p.y);
      return {x:Math.min(...xs),y:Math.min(...ys),width:Math.max(...xs)-Math.min(...xs),height:Math.max(...ys)-Math.min(...ys)};
    };
    const anchor=id=>[...root.querySelectorAll('a.diagram-element')].find(a=>a.id===(root.dataset.originNamespace||root.closest('figure').id)+'-'+id);
    const edgePath=id=>pick([...root.querySelectorAll('a.diagram-route')].find(a=>a.dataset.element===(root.dataset.originNamespace||root.closest('figure').id)+'-'+id)||anchor(id),'.relation-line');
    const ko=root.ownerDocument.documentElement.lang==='ko';
    const kindText=k=>kinds[k][ko?1:0];
    const visibleTexts=n=>[...n.querySelectorAll('text')].filter(rendered).map(t=>t.textContent);
    if(!sequence) {
      for(const [items,prefix] of [[nodes,ko?'모든 노드:':'All nodes:'],[edges,ko?'모든 관계:':'All relations:']]) {
        const uniform=items.length && items.every(n=>n.kind===items[0].kind);
        const legend=uniform && visibleTexts(root).includes(prefix+' '+kindText(items[0].kind));
        if(legend){const key=[...root.querySelectorAll('text')].find(t=>rendered(t)&&t.textContent===prefix+' '+kindText(items[0].kind));if(parseFloat(getComputedStyle(key).fontSize)!==12)errors.push({kind:'type-hierarchy',text:key.textContent,fontSize:getComputedStyle(key).fontSize});}
        for(const item of items) if(!legend && !visibleTexts(anchor(item.id)).some(t=>t===kindText(item.kind)||t.startsWith(kindText(item.kind)+' [')))errors.push({kind:'missing-visible-type',element:item.id});
      }
    }
    for(const node of nodes) {
      if(sequence){const header=pick(anchor(node.id),'.participant-box');if(!header||bounds(header).width<124)errors.push({kind:'unreadable-participant-width',element:node.id});}
      const name=expected.find(e=>e.id===node.id).text;
      const primary=pick(anchor(node.id),'.diagram-node-label');
      if(!primary || primary.textContent!==name || parseFloat(getComputedStyle(primary).fontSize)!==16)errors.push({kind:'label-hierarchy',element:node.id});
      if(node.identifier){
        const support=pick(anchor(node.id+':identifier'),'.diagram-label-support text');
        if(!primary || !support || support.textContent!==node.identifier.text || parseFloat(getComputedStyle(support).fontSize)!==12 || bounds(support).y<=bounds(primary).y)errors.push({kind:'identifier-hierarchy',element:node.id});
      }

    }
    const tasks=[];
    for(const e of edges) {
      const p=edgePath(e.id);
      const local=stages?anchor(e.id):null;
      const fromNode=stages?pick(local,'.sequence-from text'):pick(anchor(e.from),'.participant-box');
      const toNode=stages?pick(local,e.from===e.to?'.sequence-from text':'.sequence-to text'):pick(anchor(e.to),'.participant-box');
      const from=fromNode && bounds(fromNode),to=toNode && bounds(toNode);
      if(!p || !from || !to) {errors.push({kind:'missing-endpoints',edge:e.id});continue;}
      const length=p.getTotalLength();const start=point(p,p.getPointAtLength(0));const end=point(p,p.getPointAtLength(length));
      const touches=(pt,b)=>pt.x>=b.x-2 && pt.x<=b.x+b.width+2 && pt.y>=b.y-2 && pt.y<=b.y+b.height+2;
      if(sequence) {
        if(stages){const refs=pick(local,'.sequence-local-route');if(refs?.dataset.from!==e.from||refs?.dataset.to!==e.to)errors.push({kind:'wrong-local-endpoints',edge:e.id});}
        if(Math.abs(start.x-(from.x+from.width/2))>2 || Math.abs(end.x-(to.x+to.width/2))>2)errors.push({kind:'wrong-lifeline-direction',edge:e.id,start,end});
      }else if(!touches(start,from)||!touches(end,to))errors.push({kind:'wrong-edge-direction',edge:e.id,start,end});
      const semantic=bounds(pick(anchor(e.id),'text'));
      const x=Math.min(from.x,to.x,semantic.x),y=Math.min(from.y,to.y,semantic.y);
      const width=Math.max(from.x+from.width,to.x+to.width,semantic.x+semantic.width)-x;
      const height=Math.max(from.y+from.height,to.y+to.height,semantic.y+semantic.height)-y;
      const v=root.closest('.diagram-viewport');
      tasks.push({edge:e.id,from:e.from,to:e.to,bounds:{x,y,width,height},requiredHorizontalTravel:Math.max(0,width-v.clientWidth),requiredVerticalTravel:Math.max(0,height-v.clientHeight)});
    }
    if(sequence) {
      const ko=root.ownerDocument.documentElement.lang==='ko';
      let previousGroupBottom=-Infinity;
      for(const [gi,g] of groups.entries()) {
        const order=g.order==='parallel'?(ko?'par · 상대 순서 없음':'par · no relative order'):(ko?'strict · 순차':'strict · sequential');
        const header=[...root.querySelectorAll('text')].find(t=>rendered(t)&&t.textContent===`${gi+1}. ${order}`);
        if(!header)errors.push({kind:'wrong-group-order',group:gi+1,expected:order});
        // Panning participant lanes must not pan the local order/guard heading
        // out of the initial article reading column.
        const viewport=root.closest('.diagram-viewport');
        if(header && bounds(header).x+bounds(header).width>viewport.clientWidth+1)errors.push({kind:'offscreen-group-context',group:gi+1});
        if(g.condition){const guard=pick(anchor('guard:'+(gi+1)),'text');if(guard && bounds(guard).x+bounds(guard).width>viewport.clientWidth+1)errors.push({kind:'offscreen-group-context',group:gi+1,role:'condition'});}
        for(const message of g.messages) {
          if(!visibleTexts(anchor(message.id)).some(t=>stages?t.startsWith(kindText(message.kind)+' · '):t.includes(' · '+kindText(message.kind)+' · ')))errors.push({kind:'missing-visible-type',element:message.id});
          // The primary action, concise endpoints and local scope are the whole
          // scene text. An extra identifier/type band cannot silently return.
          if(visibleTexts(anchor(message.id)).length!==(stages&&message.from!==message.to?4:3))errors.push({kind:'repeated-message-identity',element:message.id});
        }
        let last=previousGroupBottom;
        const positions=g.messages.flatMap(m=>{const p=edgePath(m.id);return p?[{id:m.id,box:bounds(p)}]:[];});
        for(const m of positions) {
          if(m.box.y<=previousGroupBottom || (g.order==='sequential' && m.box.y<=last))errors.push({kind:'wrong-message-order',group:gi+1,message:m.id});
          if(header && m.box.y<=bounds(header).y+bounds(header).height)errors.push({kind:'message-before-group',group:gi+1,message:m.id});
          last=m.box.y+m.box.height;
        }
        previousGroupBottom=Math.max(...positions.map(m=>m.box.y+m.box.height));
      }
    }
    for(const e of expected) {
      const a=[...root.querySelectorAll('a.diagram-element')].find(a=>a.id===(root.dataset.originNamespace||root.closest('figure').id)+'-'+e.id);
      if(e.accessibility){if(!a?.getAttribute('aria-label')?.includes(e.text))errors.push({kind:'missing-endpoint-binding',element:e.id,expected:e.text});continue;}
      const displayed=a && [...a.querySelectorAll('text')].find(t=>rendered(t)&&(e.prefix?t.textContent.endsWith(e.text):t.textContent===e.text));
      let visible=Boolean(displayed);
      if(displayed) {
        const b=bounds(displayed);visible=b.width>0 && b.height>0;
        for(let at=displayed;at && at!==root.parentElement;at=at.parentElement) {
          const style=getComputedStyle(at);
          if(style.display==='none'||style.visibility==='hidden'||Number(style.opacity)<.5)visible=false;
        }
      }
      if(!visible)errors.push({kind:'missing-visible-meaning',element:e.id,expected:e.text});
    }
    for(const t of [...root.querySelectorAll('text tspan')].filter(rendered)) {
      const r=box(bounds(t));const style=getComputedStyle(t);
      texts.push({text:t.textContent,element:t.closest('a')?.id,box:r,font:style.font,fontSize:parseFloat(style.fontSize)});
      if(r.x<view.x-1 || r.y<view.y-1 || r.x+r.width>view.x+view.width+1 || r.y+r.height>view.y+view.height+1)errors.push({kind:'clipped-text',text:t.textContent,box:r});
      if(parseFloat(style.fontSize)<(t.closest('.diagram-node-kind,.diagram-label-support')?12:t.closest('.relation-wording,.diagram-condition')?13:16))errors.push({kind:'small-text',text:t.textContent});
    }
    for(let i=0;i<texts.length;i++)for(let j=i+1;j<texts.length;j++)if(intersects(texts[i].box,texts[j].box))errors.push({kind:'text-overlap',a:texts[i],b:texts[j]});
    for(const p of [...root.querySelectorAll('.relation-line,.relation-leader,.participant-box')].filter(rendered)) {
      const length=p.getTotalLength();const d=p.getAttribute('d');const element=p.closest('a')?.id || p.closest('a')?.dataset.element;
      const hits=new Set();
      // Sampling <= .75 SVG px is finer than stroke width; no layout-side oracle.
      for(let at=0;at<=length;at+=.75) {
        const pt=point(p,p.getPointAtLength(at));
        for(let i=0;i<texts.length;i++) {
          const b=texts[i].box;
          if(pt.x>b.x+1 && pt.x<b.x+b.width-1 && pt.y>b.y+1 && pt.y<b.y+b.height-1)hits.add(i);
        }
      }
      for(const i of hits)errors.push({kind:'path-through-text',path:element||d,text:texts[i]});
      paths.push({element,d,length});
    }
    // Crossings alone are legal. A long shared run between unrelated
    // directed connections is an ambiguous merged route, even with good labels.
    const runs=new Map();const merges=new Map();
    for(const edge of edges){const p=edgePath(edge.id);if(!p)continue;
      const seen=new Set();
      for(let at=0;at<=p.getTotalLength();at+=1.5){const q=point(p,p.getPointAtLength(at));seen.add(Math.round(q.x/2)+','+Math.round(q.y/2));}
      for(const cell of seen){const previous=runs.get(cell)||[];
        for(const other of previous)if(other.from!==edge.from&&other.to!==edge.to){const key=[other.id,edge.id].sort().join(':');merges.set(key,(merges.get(key)||0)+1);}
        previous.push(edge);runs.set(cell,previous);
      }
    }
    for(const [pair,samples] of merges)if(samples>10)errors.push({kind:'route-merge',pair,samples});
    const v=root.closest('.diagram-viewport');
    return {canvas:{width:view.width,height:view.height},viewport:{width:v.clientWidth,height:v.clientHeight,left:v.scrollLeft,top:v.scrollTop,horizontalTravel:v.scrollWidth-v.clientWidth,verticalTravel:v.scrollHeight-v.clientHeight},expected,tasks,labels:texts,paths,errors};
  },{expected,edges,stages,sequence:m.kind==='sequence',groups:m.groups||[],kinds,nodes:m.nodes||m.participants});
}
module.exports={inspectDiagram};
