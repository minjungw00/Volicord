// Disposable, read-only comparison builder. Never opens a Runtime or records a publication.
// All case-specific coordinates and draft wording belong to ignored, hash-bound input.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const assert = require('node:assert/strict');
const {pathToFileURL} = require('node:url');
const hash = bytes => crypto.createHash('sha256').update(bytes).digest('hex');

async function build(config) {
  const out = path.resolve(config.output);
  assert.ok(out.includes('/rebuild/.local/'), 'outputs must remain ignored');
  assert.ok(!fs.existsSync(out), 'exclusive output directory required');
  for (const [file, sha] of Object.entries(config.input_hashes)) {
    assert.equal(hash(fs.readFileSync(file)), sha, 'frozen input changed: '+file);
  }
  fs.mkdirSync(out);
  const {chromium} = require(config.playwright);
  const browser = await chromium.launch({executablePath:config.chromium,
    args:['--no-sandbox'], env:{...process.env, FONTCONFIG_FILE:config.fontconfig}});
  const css = fs.readFileSync(path.join(__dirname,'spatial_reader.css'),'utf8');
  const script = fs.readFileSync(path.join(__dirname,'spatial_reader.js'),'utf8');
  const candidates = [];
  try {
    for (const c of config.cases) {
      const original = fs.readFileSync(c.offline,'utf8');
      const publicData = JSON.parse(fs.readFileSync(c.public));
      const diagram = publicData.diagrams.find(d=>d.id===c.diagram);
      assert.ok(diagram, 'exact diagram required');
      for (const v of c.variants) {
        const page = await browser.newPage({viewport:{width:1440,height:1000}});
        try {
          await page.route('**/*', route=>route.abort());
          await page.setContent(original);
          assert.equal(await page.locator('script').count(),0,'only script-free exports are accepted');
          await page.evaluate(({c, v, diagram, css, script})=>{
            const style=document.createElement('style');style.textContent=css;document.head.append(style);
            document.body.dataset.comparisonNavigation=v.navigation;
            const figure=document.querySelector(`.admitted-diagram[data-semantic-fingerprint="${diagram.semantic_fingerprint}"]`);
            if (!figure) throw Error('exact figure absent');
            // Freeze originals before replacing geometry. Anchors/claims/ranges stay original.
            if(v.navigation!=='baseline') {
              const ns='http://www.w3.org/2000/svg';
              const make=(tag,attrs={},text)=>{const n=document.createElementNS(ns,tag);for(const[k,val]of Object.entries(attrs))n.setAttribute(k,String(val));if(text!==undefined)n.textContent=text;return n;};
              const originalSVG=figure.querySelector('svg');
              const anchor=id=>{
                const old=document.getElementById(figure.id+'-'+id);
                if(!old)throw Error('missing role '+id);
                const a=old.cloneNode(false);a.replaceChildren();
                a.dataset.element=id;
                const title=make('title',{},old.getAttribute('aria-label'));a.append(title);
                return a;
              };
              const svg=make('svg',{xmlns:ns, width:c.world[0],height:c.world[1],viewBox:`0 0 ${c.world.join(' ')}`,
                role:'group','aria-label':diagram.title.text,class:'spatial-world','data-world-width':c.world[0],'data-world-height':c.world[1]});
              const defs=make('defs');const marker=make('marker',{id:figure.id+'-spatial-arrow',viewBox:'0 0 10 10',refX:9,refY:5,markerWidth:7,markerHeight:7,orient:'auto'});
              marker.append(make('path',{d:'M0 0L10 5L0 10Z',fill:'#087bb8'}));defs.append(marker);svg.append(defs);
              function text(parent,x,y,value,cls='world-name',width=260,center=false){
                const t=make('text',{x,y,class:cls,'text-anchor':center?'middle':'start'});
                // Conservative Unicode advances; full wording remains present and selectable.
                let line='',advance=0,lines=[];
                for(const char of value){const a=char.charCodeAt(0)>255?16:9;if(advance+a>width&&line){lines.push(line);line='';advance=0;}line+=char;advance+=a;}
                lines.push(line);lines.forEach((line,i)=>t.append(make('tspan',{x,dy:i?22:0},line)));parent.append(t);return lines.length*22;
              }
              const m=diagram.meaning,nodes=m.nodes||m.participants;
              const localized={reading_group:'읽기 묶음',context_role:'맥락 역할',record:'기록',participant:'참여자',state:'상태',initial:'시작',final:'종료',groups:'묶음',role_of:'역할',references:'참조',transition:'전이',call:'호출',await:'대기',return:'반환',state_change:'상태 변경'};
              const edges=m.relations||m.groups.flatMap(g=>g.messages);
              const drawPath=(parent,d,extra={})=>parent.append(make('path',{d,class:'world-edge','marker-end':`url(#${figure.id}-spatial-arrow)`,...extra}));
              if(m.kind==='sequence') {
                const lanes=c.lanes;
                const end=c.world[1]-30;
                for(const n of nodes)svg.append(make('path',{d:`M${lanes[n.id]} 130V${end}`,class:'world-lifeline'}));
                let row=190,ordinal=0;
                m.groups.forEach((g,gi)=>{
                  const start=row-35,height=g.messages.length*132+70;
                  svg.append(make('rect',{x:25,y:start,width:c.world[0]-50,height,rx:8,class:'world-group'}));
                  text(svg,45,start+24,`${gi+1}. ${g.order==='parallel'?'병렬 · 상대 순서 없음':'순차'}`,'world-support',500);
                  if(g.condition){const guard=anchor('guard:'+(gi+1));text(guard,410,start+24,g.condition.text,'world-guard',850);svg.append(guard);}
                  g.messages.forEach(e=>{
                    ordinal++;const a=anchor(e.id),from=lanes[e.from],to=lanes[e.to];
                    const labelX=e.from===e.to?from+100:Math.min(from,to)+25;
                    const labelWidth=e.from===e.to?460:Math.max(260,Math.abs(to-from)-50);
                    const h=text(a,labelX,row+26,e.label.text,'world-action',labelWidth);
                    text(a,labelX,row+26+h,`${ordinal} · ${nodes.find(n=>n.id===e.from).label.text} → ${nodes.find(n=>n.id===e.to).label.text} · ${localized[e.kind]||e.kind}`,'world-support',labelWidth);
                    const y=row+78;
                    drawPath(a,from===to?`M${from} ${y-35}H${from+65}V${y+12}H${from}`:`M${from} ${y}H${to}`,e.kind==='return'?{'stroke-dasharray':'7 5'}:{});
                    svg.append(a);row+=132;
                  });row+=92;
                });
                for(const n of nodes){
                  const x=lanes[n.id];const a=anchor(n.id);a.append(make('rect',{x:x-140,y:24,width:280,height:100,rx:8,class:'world-node'}));
                  text(a,x,52,n.label.text,'world-name',250,true);text(a,x,109,localized[n.kind]||n.kind,'world-support',250,true);svg.append(a);
                  if(n.identifier){const id=anchor(n.id+':identifier');text(id,x,80,n.identifier.text,'world-identifier',250,true);svg.append(id);}
                }
              } else {
                for(const e of edges){
                  const a=anchor(e.id),r=c.routes[e.id];if(!r)throw Error('manual route absent: '+e.id);
                  drawPath(a,r.path);const h=text(a,r.label[0],r.label[1],e.label.text,'world-action',r.width||320);
                  const f=nodes.find(n=>n.id===e.from),t=nodes.find(n=>n.id===e.to);
                  text(a,r.label[0],r.label[1]+h,`${f.label.text} → ${t.label.text} · ${localized[e.kind]||e.kind}`,'world-support',r.width||320);
                  svg.append(a);
                  if(e.condition){const guard=anchor(e.id+':condition');text(guard,r.label[0],r.label[1]+h+44,e.condition.text,'world-guard',r.width||320);svg.append(guard);}
                }
                for(const n of nodes){const [x,y]=c.positions[n.id],a=anchor(n.id);
                  a.append(make('rect',{x,y,width:250,height:100,rx:8,class:'world-node'}));
                  text(a,x+125,y+30,n.label.text,'world-name',225,true);text(a,x+125,y+86,localized[n.kind]||n.kind,'world-support',225,true);svg.append(a);
                  if(n.identifier){const id=anchor(n.id+':identifier');text(id,x+125,y+58,n.identifier.text,'world-identifier',225,true);svg.append(id);}
                }
                if(m.owner){const owner=anchor('state:owner');text(owner,30,c.world[1]-30,m.owner.text,'world-guard',1000);svg.append(owner);}
              }
              const viewport=originalSVG.closest('.diagram-viewport');viewport.replaceChildren(svg);
              viewport.classList.add('spatial-viewport');viewport.setAttribute('aria-label','관계 읽기 공간 · 방향키로 이동 · Tab으로 요소 선택');
              figure.classList.add('spatial-figure');
              const nav=document.createElement('div');nav.className='spatial-navigation';
              const start=document.createElement('span');start.textContent='시작: '+nodes[0].label.text;nav.append(start);
              const chooser=document.createElement('details'),summary=document.createElement('summary');summary.textContent='요소로 이동';chooser.append(summary);
              const list=document.createElement('ul');
              for(const a of svg.querySelectorAll('a.diagram-element')){
                const li=document.createElement('li'),link=document.createElement('a');link.href='#'+a.id;link.dataset.worldTarget=a.id;link.textContent=a.getAttribute('aria-label');li.append(link);list.append(li);
              }
              chooser.append(list);nav.append(chooser);
              const controls=document.createElement('span');controls.className='spatial-enhancement';controls.hidden=true;
              const names=v.navigation==='camera'?[['left','←'],['right','→'],['up','↑'],['down','↓'],['out','−'],['in','+'],['fit','전체 위치'],['reset','읽기 100%'],['focus','선택 관계'],['expand','크게 보기']]:[['expand','크게 보기']];
              for(const[action,label]of names){const b=document.createElement('button');b.type='button';b.dataset.worldAction=action;b.textContent=label;b.setAttribute('aria-label',({left:'왼쪽으로 이동',right:'오른쪽으로 이동',up:'위로 이동',down:'아래로 이동',out:'축소',in:'확대'})[action]||label);controls.append(b);}
              const status=document.createElement('output');status.className='spatial-scale';status.textContent='100%';controls.append(status);nav.append(controls);
              const help=document.createElement('p');help.className='spatial-help spatial-native-help';help.textContent='스크롤 또는 요소로 이동으로 관계를 따라 읽으세요. Tab으로 요소를 고르고 Enter로 근거를 엽니다.';
              viewport.before(nav,help);
              if(v.navigation==='camera'){
                const cameraHelp=document.createElement('p');cameraHelp.className='spatial-help spatial-camera-help';cameraHelp.hidden=true;cameraHelp.textContent='빈 배경 끌기·방향 버튼/키로 이동합니다. 전체 위치는 방향 확인용이며 글은 읽기 100%에서 읽으세요. Tab으로 요소를 고르고 Enter로 근거를 엽니다. Esc로 돌아갑니다.';viewport.before(cameraHelp);
              }
              // Original full timeline and exhaustive text remain independently accessible.
              figure.dataset.worldGeometry=c.geometry;
            }
            if(v.editorial==='edited') {
              const doc=document.querySelector(`#work-${c.work} .doc`);
              if(!doc)throw Error('Work required for editorial comparison');
              const intro=doc.querySelector('.doc-head');
              const notice=document.createElement('p');notice.className='source-note draft-notice';notice.textContent='편집 초안 · 원래 Goal과 보고는 유지됩니다. 새 발행물이나 새 검증 실행이 아닙니다.';intro.prepend(notice);
              // Independent list summary is drafted from canonical facts, never the article.
              for(const card of document.querySelectorAll(`.work-card[data-work-id="${c.work}"]`)){
                const s=card.querySelector('[data-question="Synopsis"]');if(s){s.textContent=c.editorial.summary;s.removeAttribute('data-reader-publication');s.dataset.draft='independent-summary';}
                card.removeAttribute('data-summary-realization');card.dataset.summaryState='Draft';
                const note=document.createElement('p');note.className='source-note';note.textContent='요약 편집 초안 · 이름과 상태는 원래 기록';card.append(note);
              }
              const lead=intro.querySelector('[data-question="Synopsis"]');
              const retained=document.createElement('details');retained.className='reading-section';
              const retainedHeading=document.createElement('summary');retainedHeading.textContent='원래 도입과 편집 근거';retained.append(retainedHeading);
              if(lead){const original=lead.parentElement;const draft=document.createElement('p');draft.className='lead';draft.textContent=c.editorial.lead;draft.dataset.draft='article-lead';original.before(draft);retained.append(original);}
              const reading=doc.querySelector('[id$="-reading"]');
              // Move complete authored blocks, preserving claim/range origins and original text.
              const support=[...reading.children].find(n=>n.querySelector('[data-question="SectionHeading"]')?.textContent==='호출 연결과 원래 기록의 근거');
              const next=[...reading.children].find(n=>n.querySelector('[data-question="SectionHeading"]')?.textContent==='남은 단계');
              const verification=doc.querySelector('.verification-report');if(verification){
                verification.open=true;reading.after(verification);
                for(const execution of verification.querySelectorAll(':scope > article.verification-execution')){
                  const header=execution.querySelector(':scope > header'),label=header?.querySelector('h3');
                  if(!label)throw Error('verification command label absent');
                  const details=document.createElement('details');
                  for(const attribute of execution.attributes)details.setAttribute(attribute.name,attribute.value);
                  const summary=document.createElement('summary');
                  label.style.display='inline';label.style.margin='0';summary.append(label);
                  details.append(summary,...header.childNodes);header.remove();
                  details.append(...execution.childNodes);execution.replaceWith(details);
                }
              }
              const limits=doc.querySelector('.limits-section');if(limits)verification?.before(limits);
              if(next)verification?.after(next);
              if(support)(next||verification||reading).after(support);
              (support||next||verification||reading).after(retained);
              const applicability=intro.querySelector('.applicability');if(applicability)retained.append(applicability);
              const bindings=document.createElement('ul');
              for(const id of [5,6,7,8,9,16,17,19]){
                const claim=doc.querySelector(`[id$="-p${id}"]`);if(!claim)continue;
                const li=document.createElement('li'),a=document.createElement('a');a.href='#'+claim.id;a.textContent=claim.querySelector('.unit-text')?.textContent||claim.textContent;li.append(a);bindings.append(li);
              }
              retained.append(bindings);
              const history=doc.querySelector('.recorded-timeline');if(history)doc.querySelector('.evidence-index')?.before(history);
              const digest=doc.querySelector('.verification-digest');if(digest)verification?.querySelector('summary')?.after(digest);
              // Capture details remain in supporting provenance, not a list-summary role.
              const oldSummary=[...doc.querySelectorAll('[data-question="Synopsis"]')].find(n=>!intro.contains(n));
              if(oldSummary){oldSummary.dataset.originalSummary='true';const box=document.createElement('details');const s=document.createElement('summary');s.textContent='원래 독립 요약과 캡처 범위';box.append(s);oldSummary.before(box);box.append(oldSummary);}
              doc.dataset.editorialDraft='true';
            }
            const s=document.createElement('script');s.type='application/x-local-reader';s.textContent=script;document.body.append(s);
          },{c,v,diagram,css,script});
          const file=c.key+'-'+v.id+'.html';
          const html='<!DOCTYPE html>\n'+(await page.locator('html').evaluate(n=>n.outerHTML)).replace('type="application/x-local-reader"','type="text/javascript"');
          fs.writeFileSync(path.join(out,file),html,{flag:'wx'});
          const geometry=await page.locator(`.admitted-diagram[data-semantic-fingerprint="${diagram.semantic_fingerprint}"] > .diagram-full-view svg`).first().evaluate(n=>n.outerHTML);
          const entry=c.work?'works':'repository-explanation';
          candidates.push({id:c.key+'-'+v.id,file,entry,sha256:hash(html),geometry_sha256:geometry?hash(geometry):null,
            navigation:v.navigation,editorial:v.editorial,diagram:c.diagram,project:publicData.provenance.project_id,
            publication:publicData.provenance.publication_id,source_html_sha256:hash(original),manual_geometry:v.navigation!=='baseline',
            authority:v.editorial==='edited'?'unadmitted editorial draft over frozen export':'frozen export; no live lifecycle enforcement'});
        } finally {await page.close();}
      }
    }
  } finally {await browser.close();}
  const manifest={kind:'local-reading-comparison',selection:null,candidates,inputs:config.input_hashes,
    tooling:Object.fromEntries(['spatial_comparison.cjs','spatial_reader.css','spatial_reader.js'].map(n=>[n,hash(fs.readFileSync(path.join(__dirname,n)))]))};
  fs.writeFileSync(path.join(out,'manifest.json'),JSON.stringify(manifest,null,2),{flag:'wx'});
  const escape=s=>String(s).replaceAll('&','&amp;').replaceAll('<','&lt;').replaceAll('"','&quot;');
  const rows=candidates.map(c=>`<tr><td><a href="${c.file}#${c.entry}">${escape(c.id)}</a></td><td>${escape(c.navigation)}</td><td>${escape(c.editorial)}</td><td><code>${c.sha256}</code></td></tr>`).join('');
  fs.writeFileSync(path.join(out,'index.html'),`<!doctype html><html lang="ko"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>읽기 비교</title><style>body{font:16px/1.7 system-ui;max-width:1000px;margin:40px auto;padding:20px;color:#152e40}table{border-collapse:collapse;width:100%}td,th{padding:12px;border-bottom:1px solid #cddfe9;text-align:left}code{font-size:11px;overflow-wrap:anywhere}a{color:#087bb8}li{margin:10px 0}table{display:block;overflow:auto}</style><h1>읽기 비교</h1><p>A는 현재 폭 제한 배치, B는 넓은 배치와 스크롤, C는 B와 같은 배치에 카메라를 더한 후보입니다. A/B/C의 내용은 같습니다. E0/E1은 A의 고정 배치에서 현재/편집 정보를 비교합니다. CE1은 상호작용 확인용 결합안입니다.</p><p>이 자료는 고정된 과거 발행물의 로컬 비교입니다. 좌표는 수동 참조 배치이며 일반 배치 엔진의 성공을 주장하지 않습니다. 편집안은 미발행 초안입니다. 실제 Runtime·권한·보존 수명을 시험하지 않으며, 기기 검사·사용자 승인·새 분석을 추가하지 않습니다.</p><h2>중립 과제</h2><ol>${config.tasks.map(t=>`<li>${escape(t)}</li>`).join('')}</ol><p>처음 답, 망설임, 잘못 간 곳, 반복 추적, 놓친 조작, 맥락 손실을 그대로 기록하세요. 읽은 순서와 이미 아는 내용을 함께 남기세요. 실패한 후보도 유지합니다. 작은 비교는 통계적 우월성의 근거가 아닙니다.</p><table><thead><tr><th>후보</th><th>이동</th><th>편집</th><th>SHA-256</th></tr></thead><tbody>${rows}</tbody></table><h2>선택할 항목</h2><p>이 패키지의 manifest 해시, 각 실제 사례의 A/B/C 선택, Work의 E0/E1 선택, 적용 범위와 필요한 수정을 명시해 주세요. 권고·테스트 통과·응답 없음은 선택이 아닙니다. 선택 전에는 기본 화면을 바꾸지 않습니다.</p><p>HTTP: 이 디렉터리에서 <code>python3 -m http.server 8765 --bind 127.0.0.1</code> 후 localhost:8765/index.html. 파일 URL도 지원하며 JavaScript를 끄면 읽을 수 있는 원래 크기 SVG·스크롤·근거 링크가 남습니다.</p></html>`,{flag:'wx'});
  console.log(JSON.stringify({output:out,candidates:candidates.length,manifest_sha256:hash(fs.readFileSync(path.join(out,'manifest.json')))}));
}
if(require.main===module)build(JSON.parse(fs.readFileSync(process.argv[2]))).catch(e=>{console.error(e);process.exitCode=1;});
module.exports={build};
