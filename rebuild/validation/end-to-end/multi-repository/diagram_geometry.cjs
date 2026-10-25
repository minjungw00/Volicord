// Actual browser glyph/path checks. Expected wording comes from the original
// admitted publication, never renderer coordinates or generated SVG attributes.
async function inspectDiagram(svg, diagram, prose) {
  const label = l => Buffer.from(prose[l.unit].text).subarray(l.start_byte,l.end_byte).toString('utf8');
  const m=diagram.meaning;
  const expected=(m.nodes||m.participants).map(n=>({id:n.id,text:label(n.label)}));
  if(m.owner)expected.push({id:'state:owner',text:label(m.owner),prefix:true});
  for(const r of m.relations||[]) {
    expected.push({id:r.id,text:label(r.label)});
    if(r.condition)expected.push({id:r.id+':condition',text:label(r.condition),prefix:true});
  }
  for(const [i,g] of (m.groups||[]).entries()) {
    if(g.condition)expected.push({id:'guard:'+(i+1),text:label(g.condition),prefix:true});
    expected.push(...g.messages.map(r=>({id:r.id,text:label(r.label)})));
  }
  const edges=m.relations || (m.groups||[]).flatMap(g=>g.messages);
  return svg.evaluate((root,{expected,edges,sequence,groups})=> {
    const errors=[];const texts=[];const paths=[];
    const box=b=>({x:b.x,y:b.y,width:b.width,height:b.height});
    const intersects=(a,b)=>a.x<b.x+b.width-1 && a.x+a.width>b.x+1 && a.y<b.y+b.height-1 && a.y+a.height>b.y+1;
    const view=root.viewBox.baseVal;
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
    const anchor=id=>[...root.querySelectorAll('a.diagram-element')].find(a=>a.id===root.closest('figure').id+'-'+id);
    const tasks=[];
    for(const e of edges) {
      const p=anchor(e.id)?.querySelector('.relation-line');
      const fromNode=anchor(e.from)?.querySelector('.participant-box');
      const toNode=anchor(e.to)?.querySelector('.participant-box');
      const from=fromNode && bounds(fromNode),to=toNode && bounds(toNode);
      if(!p || !from || !to) {errors.push({kind:'missing-endpoints',edge:e.id});continue;}
      const length=p.getTotalLength();const start=point(p,p.getPointAtLength(0));const end=point(p,p.getPointAtLength(length));
      const touches=(pt,b)=>pt.x>=b.x-2 && pt.x<=b.x+b.width+2 && pt.y>=b.y-2 && pt.y<=b.y+b.height+2;
      if(sequence) {
        if(Math.abs(start.x-(from.x+from.width/2))>2 || Math.abs(end.x-(to.x+to.width/2))>2)errors.push({kind:'wrong-lifeline-direction',edge:e.id,start,end});
      }else if(!touches(start,from)||!touches(end,to))errors.push({kind:'wrong-edge-direction',edge:e.id,start,end});
      const semantic=bounds(anchor(e.id).querySelector('text'));
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
        const header=[...root.querySelectorAll('text')].find(t=>t.textContent===`${gi+1}. ${order}`);
        if(!header)errors.push({kind:'wrong-group-order',group:gi+1,expected:order});
        let last=previousGroupBottom;
        const positions=g.messages.flatMap(m=>{const p=anchor(m.id)?.querySelector('.relation-line');return p?[{id:m.id,box:bounds(p)}]:[];});
        for(const m of positions) {
          if(m.box.y<=previousGroupBottom || (g.order==='sequential' && m.box.y<=last))errors.push({kind:'wrong-message-order',group:gi+1,message:m.id});
          if(header && m.box.y<=bounds(header).y+bounds(header).height)errors.push({kind:'message-before-group',group:gi+1,message:m.id});
          last=m.box.y+m.box.height;
        }
        previousGroupBottom=Math.max(...positions.map(m=>m.box.y+m.box.height));
      }
    }
    for(const e of expected) {
      const a=[...root.querySelectorAll('a.diagram-element')].find(a=>a.id===root.closest('figure').id+'-'+e.id);
      const displayed=a && [...a.querySelectorAll('text')].find(t=>e.prefix?t.textContent.endsWith(e.text):t.textContent===e.text);
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
    for(const t of root.querySelectorAll('text tspan')) {
      const r=box(bounds(t));const style=getComputedStyle(t);
      texts.push({text:t.textContent,element:t.closest('a')?.id,box:r,font:style.font,fontSize:parseFloat(style.fontSize)});
      if(r.x<view.x-1 || r.y<view.y-1 || r.x+r.width>view.x+view.width+1 || r.y+r.height>view.y+view.height+1)errors.push({kind:'clipped-text',text:t.textContent,box:r});
      if(parseFloat(style.fontSize)<16)errors.push({kind:'small-text',text:t.textContent});
    }
    for(let i=0;i<texts.length;i++)for(let j=i+1;j<texts.length;j++)if(intersects(texts[i].box,texts[j].box))errors.push({kind:'text-overlap',a:texts[i],b:texts[j]});
    for(const p of root.querySelectorAll('.relation-line,.participant-box')) {
      const length=p.getTotalLength();const d=p.getAttribute('d');const element=p.closest('a')?.id;
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
    const v=root.closest('.diagram-viewport');
    return {canvas:{width:view.width,height:view.height},viewport:{width:v.clientWidth,height:v.clientHeight,left:v.scrollLeft,top:v.scrollTop,horizontalTravel:v.scrollWidth-v.clientWidth,verticalTravel:v.scrollHeight-v.clientHeight},expected,tasks,labels:texts,paths,errors};
  },{expected,edges,sequence:m.kind==='sequence',groups:m.groups||[]});
}
module.exports={inspectDiagram};
