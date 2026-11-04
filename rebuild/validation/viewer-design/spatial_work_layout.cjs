// Browser-side rearrangement of a frozen Work draft. No authoring or Runtime access.
function arrangeWork({work}) {
  const doc=document.querySelector(`#work-${work} .doc`);
  if(!doc?.dataset.editorialDraft)throw Error('an isolated editorial draft is required');
  const make=(tag,cls,text)=>{const n=document.createElement(tag);if(cls)n.className=cls;if(text)n.textContent=text;return n;};
  const reading=doc.querySelector('[id$="-reading"]'),prefix=reading.id.replace(/-reading$/,'');
  const required=selector=>{const n=doc.querySelector(selector);if(!n)throw Error('missing Work section: '+selector);return n;};
  const wrap=(title,id)=>{const d=make('details','reading-section work-record');if(id)d.id=id;d.append(make('summary','',title));return d;};
  const head=required('.doc-head'),verification=required('.verification-report'),limits=required('.limits-section');
  // Reuse the sidebar's typed state and icon styling in list and detail facts.
  for(const card of document.querySelectorAll('.work-card[data-work-state]')){
    const status=card.querySelector('.work-status');if(!status)continue;
    status.dataset.state=card.dataset.workState;status.classList.add('work-badge','work-state-badge');
    const dot=make('span','state-dot');dot.setAttribute('aria-hidden','true');status.prepend(dot);
  }
  const card=document.querySelector(`.work-card[data-work-id="${work}"]`),badge=card?.querySelector('.work-state-badge');
  if(!badge)throw Error('exact Work status required');
  const strip=head.querySelector('.record-strip');strip.replaceChildren(make('span','record-status-label','기록된 상태'),badge.cloneNode(true));
  const audit=required('.audit'),original=required('.record-box:has([id$="-original"])');
  const evidence=required('.record-box:has([id$="-audit"])'),evidenceBody=evidence.querySelector('[id$="-audit"]');
  const next=[...doc.querySelectorAll('.article-section')].find(n=>n.querySelector('[data-question="SectionHeading"]')?.textContent==='남은 단계');
  const support=[...doc.querySelectorAll('.article-section')].find(n=>n.querySelector('[data-question="SectionHeading"]')?.textContent==='호출 연결과 원래 기록의 근거');
  if(!next||!support)throw Error('frozen authored sections missing');

  const main=make('section','work-reading-group');main.dataset.readingGroup='work';
  const checks=make('section','work-reading-group');checks.dataset.readingGroup='checks';checks.setAttribute('aria-label','검증과 설명 한계');
  const records=make('section','work-reading-group');records.dataset.readingGroup='records';records.setAttribute('aria-label','원문과 근거');
  doc.querySelector('.breadcrumb').after(main,checks,records);
  main.append(head,reading);reading.append(next);reading.classList.add('work-section-stack');
  checks.append(verification,limits);
  const limitsHeading=limits.querySelector('h2');limitsHeading.textContent='설명 한계';limitsHeading.classList.add('reading-layer-heading');
  const guard=limits.querySelector(':scope > .guard');if(guard){const body=make('div','reading-layer-body');body.append(...guard.childNodes);guard.replaceWith(body);}
  limits.prepend(limitsHeading);limits.append(required('.reading-warning'));
  original.querySelector(':scope > summary').textContent='원래 지시';
  evidence.querySelector(':scope > summary').textContent='전체 근거';
  records.append(original,evidence);
  // Superseded headings are navigation aliases, not archived explanatory prose.
  // Baseline publications retain the old wording; this edited draft uses its new titles.
  function retireHeading(unit){
    for(const heading of unit.querySelectorAll('.unit-text')){
      const anchor=make('span');if(heading.id)anchor.id=heading.id;heading.replaceWith(anchor);
    }
    unit.className='unit-anchor';unit.tabIndex=-1;
  }

  // The three actual call connections explain the mechanism. Original Goal/report
  // provenance belongs to inspection. Every original unit and citation is moved intact.
  const mechanism=reading.querySelector('.article-section');
  for(const unit of [...support.querySelectorAll(':scope > .unit')]){
    if(unit.querySelector('[data-question="Narrative"]')&&!unit.querySelector('.unit-text').textContent.startsWith('원래 Goal'))mechanism.append(unit);
  }
  const editorial=wrap('편집 근거',prefix+'-editorial-basis');
  retireHeading(support.querySelector('.unit:has([data-question="SectionHeading"])'));
  editorial.append(support);
  const retained=[...doc.querySelectorAll('details')].find(n=>n.querySelector(':scope > summary')?.textContent==='원래 도입과 편집 근거');
  if(retained)editorial.append(retained);
  const summaryBasis=wrap('요약 근거',prefix+'-summary-basis');
  const summaryPublication=evidence.querySelector('[data-publication-scope="summary"]');
  if(summaryPublication)summaryBasis.append(summaryPublication);
  const index=required('.evidence-index'),history=required('.recorded-timeline'),codeScope=required('.code-reading-scope');
  const historyBox=wrap('기록의 흐름',prefix+'-record-history');historyBox.append(history);records.append(historyBox);
  const historyHeading=history.querySelector(':scope > h2');if(historyHeading){const anchor=make('span');if(historyHeading.id)anchor.id=historyHeading.id;historyHeading.replaceWith(anchor);}
  codeScope.querySelector(':scope > summary').textContent='포함된 코드 범위';
  evidenceBody.prepend(editorial,summaryBasis,index,codeScope);

  // The report explains results and scope. Original commands, reports and capture
  // metadata have one home under raw verification, never a second report card UI.
  const raw=wrap('검증 원문',prefix+'-verification-originals');
  const executions=[...verification.querySelectorAll(':scope > .verification-execution')];
  const reportList=wrap('개별 검증 기록 · '+executions.length+'개 항목',prefix+'-verification-records');
  reportList.classList.add('verification-report-list');
  for(const paragraph of [...verification.querySelectorAll(':scope > p')])paragraph.remove();
  for(const heading of [...verification.querySelectorAll('[data-question="SectionHeading"]')])retireHeading(heading.closest('.unit'));
  for(const execution of executions){
    const source=execution.dataset.executionSource;
    execution.id=prefix+'-execution-'+source;
    execution.classList.add('verification-report-row','record-item');execution.open=false;reportList.append(execution);
    execution.querySelector(':scope > summary h3').append(make('small','record-item-meta','명령 Source · '+source));
  }
  const reportSection=make('section',verification.className);
  for(const attribute of verification.attributes)if(attribute.name!=='open')reportSection.setAttribute(attribute.name,attribute.value);
  const reportHeading=make('h2','reading-layer-heading',verification.querySelector(':scope > summary').textContent);
  verification.querySelector(':scope > summary').replaceWith(reportHeading);
  reportSection.append(...verification.childNodes);verification.replaceWith(reportSection);
  const json=required('[id$="-full-verification"]'),recordsLink=make('a','paragraph-evidence-action verification-records-return','검증 원문에서 개별 기록 확인');recordsLink.href='#'+reportList.id;
  reportSection.append(recordsLink);
  raw.append(make('p','record-intro','Checkpoint에 기록된 보고를 명령 Source별로 묶었습니다. 항목을 열면 보고 원문, 실행 메타데이터와 캡처 근거를 확인할 수 있습니다. 보고 건수는 테스트 수나 새 실행 횟수가 아닙니다.'),reportList,json);records.append(raw);
  audit.remove();

  // Each evidence category has one purpose. Do not retain copied body paragraphs,
  // repeated test reports or a mixed bag of publication and state observations.
  if(retained){
    const lead=retained.querySelector('.lead'),leadAliases=new Set();
    for(let n=lead?.parentElement;n&&n!==retained;n=n.parentElement)if(n.id)leadAliases.add(n.id);
    const intro=lead?wrap('원래 도입문'):null;if(intro){intro.append(lead);editorial.append(intro);}
    const applicability=retained.querySelector('.applicability');if(applicability){applicability.querySelector(':scope > summary').textContent='저장소 비교 기준';evidenceBody.append(applicability);}
    for(const old of retained.querySelectorAll('[id]')){
      const anchor=make('span');anchor.id=old.id;
      if(leadAliases.has(old.id))intro.querySelector(':scope > summary').after(anchor);else editorial.append(anchor);
    }
    retained.remove();
  }
  const publication=wrap('발행 정보',prefix+'-publication-data'),recordData=wrap('기록 상태와 근거 한계',prefix+'-record-data');
  const observations=wrap('상태 관측 원문',prefix+'-state-observations');
  for(const child of [...evidenceBody.children]){
    if([editorial,summaryBasis,index,codeScope].includes(child)||child.matches('.applicability'))continue;
    if(child.matches('.state-observation')){observations.append(child);continue;}
    if(child.matches('pre.publication-provenance')||child.matches('pre')&&child.textContent.startsWith('ExplanationProvenance ')){
      child.dataset.rawLabel=child.textContent.includes('question: "work_name"')?'작업 이름 발행 정보':child.textContent.includes('question: "work_summary"')?'작업 요약 발행 정보':'작업 설명 발행 정보';publication.append(child);continue;
    }
    if(child.matches('p')){publication.append(child);continue;}
    recordData.append(child);
  }
  if(observations.children.length>1)historyBox.append(observations);
  const facts=recordData.querySelector('.record-facts');
  if(facts)for(const p of [...facts.querySelectorAll(':scope > p')]){
    if(/^(검증 보고 \(|보고된 검증 상태:|기록된 검증 인용:)/.test(p.textContent)||[...limits.querySelectorAll('p')].some(n=>n.textContent===p.textContent))p.remove();
  }
  const limitOriginal=wrap('기록된 한계 원문',prefix+'-limit-originals');
  for(const p of limits.querySelectorAll(':scope > .reading-layer-body > p'))limitOriginal.append(p);
  const sourceWarning=limits.querySelector(':scope > .reading-warning');if(sourceWarning)limitOriginal.append(sourceWarning);
  if(limitOriginal.children.length>1){
    const content=make('div','limit-original-body');content.id=limitOriginal.id+'-content';
    for(const child of [...limitOriginal.childNodes])if(child.nodeName!=='SUMMARY')content.append(child);limitOriginal.append(content);
    raw.append(limitOriginal);
    const link=make('a','paragraph-evidence-action verification-records-return','기록된 한계 원문 보기');link.href='#'+content.id;limits.append(link);
  }
  evidenceBody.append(publication,recordData);
  // Update index navigation labels to the current composition, keeping bindings.
  for(const entry of index.querySelectorAll('.evidence-entry')){
    for(const node of entry.querySelectorAll('summary *'))if(!node.children.length){
      if(node.textContent==='무엇을 확인했다고 보고했나')node.textContent='검증 보고';
      if(node.textContent==='호출 연결과 원래 기록의 근거')node.textContent='편집 근거';
    }
    const context=entry.querySelector(':scope > summary small'),label=entry.querySelector(':scope > summary > span > span');
    if(context&&label&&context.textContent===label.textContent)context.textContent='섹션 제목';
    entry.classList.add('record-item');
  }
  // The summary already has a category disclosure; avoid another identical layer.
  const summaryEvidence=summaryBasis.querySelector('.publication-evidence');
  if(summaryEvidence){summaryEvidence.querySelector(':scope > summary')?.remove();summaryEvidence.replaceWith(...summaryEvidence.childNodes);}
  for(const pre of records.querySelectorAll('pre')){
    const parent=pre.parentElement;
    if(parent.matches('details')&&parent.children.length===2&&parent.querySelector(':scope > summary')){parent.classList.add('raw-disclosure');continue;}
    const label=pre.dataset.rawLabel||(pre.closest('.state-observation')?'관측 데이터':pre.textContent.startsWith('Work: ')?'작업 기록 원문':'원문 데이터');
    const disclosure=wrap(label,pre.id?pre.id+'-disclosure':'');
    disclosure.classList.add('raw-disclosure');pre.before(disclosure);disclosure.append(pre);
  }
  for(const details of doc.querySelectorAll('.work-reading-group details')){
    if(details.closest('.admitted-diagram'))continue;
    details.classList.add('work-disclosure');
    let level=1;for(let parent=details.parentElement;parent&&parent!==doc;parent=parent.parentElement)if(parent.tagName==='DETAILS')level++;
    details.dataset.disclosureLevel=String(level);
    if(records.contains(details))details.open=false;
  }

  // Open a selected range directly, with its complete original citation family
  // available in the pane. No intermediate citation-list page or code copy.
  const appendix=document.querySelector('.appendix');
  function rangePicker(links,family){
    for(let i=0;i<links.length;i++){
      const target=document.getElementById(links[i].getAttribute('href').slice(1));if(!target)throw Error('citation target missing');
      const picker=make('div','paragraph-source-picker'),label=make('label'),select=make('select','evidence-range-select');
      label.append(make('span','','근거 선택 · '+links.length+'개 범위'));select.setAttribute('aria-label','이 설명의 근거 선택');
      select.dataset.citationFamily=family;
      const fallback=make('details','native-range-list'),list=make('ul','paragraph-evidence-list');fallback.append(make('summary','','다른 근거 선택'));
      for(let j=0;j<links.length;j++){
        const link=links[j],point=document.getElementById(link.getAttribute('href').slice(1));
        const sourcePath=point.querySelector('.source-path')||point.closest('.source-point')?.querySelector('.source-path');
        const option=make('option','',(j+1)+'. '+(sourcePath?.textContent||link.getAttribute('title')||link.textContent));
        option.value=link.getAttribute('href');option.defaultSelected=j===i;select.append(option);
        const li=make('li'),copy=link.cloneNode(true);copy.removeAttribute('id');li.append(copy);if(sourcePath)li.append(make('span','source-note',sourcePath.textContent));list.append(li);
      }
      label.append(select);picker.append(label);fallback.append(list);picker.append(fallback);
      for(const previous of target.querySelectorAll(':scope > .native-range-selector,:scope > .range-selector')){
        const old=[...previous.querySelectorAll('a[href]')].map(n=>n.getAttribute('href')),current=links.map(n=>n.getAttribute('href'));
        if(old.every(href=>current.includes(href))&&current.every(href=>old.includes(href)))previous.remove();else fallback.append(previous);
      }
      const after=target.querySelector(':scope > .evidence-selection-context')||target.querySelector(':scope > .evidence-claim');
      if(after)after.after(picker);else target.prepend(picker);
    }
  }
  function evidenceAction(actions,origin,links){
    const action=make('a','evidence-link paragraph-evidence-action','근거 보기');action.href=links[0].getAttribute('href');action.dataset.origin=origin;
    action.dataset.citationTargets=JSON.stringify(links.map(n=>n.getAttribute('href')));
    action.setAttribute('aria-label','이 설명의 근거 '+links.length+'개 보기');actions.replaceChildren(action);
  }
  for(const caption of doc.querySelectorAll('.diagram-caption-claims')){
    const actions=caption.querySelector(':scope > .evidence-actions');
    if(!actions)throw Error('diagram caption citation missing');
    caption.replaceWith(actions);
  }
  for(const actions of doc.querySelectorAll('.unit > .evidence-actions')){
    const unit=actions.parentElement,text=unit.querySelector(':scope > p.unit-text');
    const links=[...actions.querySelectorAll('a.evidence-link')];
    if(!text||!links.length)continue;
    rangePicker(links,unit.id);
    evidenceAction(actions,unit.id,links);
  }
  for(const claim of doc.querySelectorAll('.diagram-claim')){
    const links=[...claim.querySelectorAll('a.evidence-link')];
    if(links.length){rangePicker(links,claim.id);evidenceAction(claim.querySelector('.evidence-actions'),claim.id,links);}
    const context=claim.querySelector('.diagram-claim-context');
    if(context){const body=make('div','diagram-claim-context');for(const child of [...context.childNodes])if(child.nodeName!=='SUMMARY')body.append(child);context.replaceWith(body);}
    const header=make('header','diagram-claim-header'),back=claim.querySelector('.diagram-return');
    if(back){back.className='diagram-return';back.textContent='도식으로 돌아가기';header.append(back);}
    header.prepend(claim.querySelector(':scope > h3'));claim.prepend(header);
  }
  for(const bar of appendix.querySelectorAll('.point-bar')){
    bar.classList.add('reading-panel-header');
    bar.prepend(make('strong','','근거'));
    const back=bar.querySelector('a.return');if(back){back.classList.add('panel-close');back.textContent='×';back.setAttribute('aria-label','근거 닫고 설명으로 돌아가기');}
    const reset=make('button','panel-width-reset','↔');reset.type='button';reset.setAttribute('aria-label','근거 창 기본 너비');reset.title='기본 너비';bar.querySelector('strong').after(reset);
  }
  // Keep one selected explanation, with distinct supporting context only when it
  // differs. Common observation metadata uses the same disclosure hierarchy as Work.
  const normalize=text=>text.replace(/\s+/g,' ').trim();
  const sharedContextStyles=[];
  for(const point of appendix.querySelectorAll('.source-point,.source-origin-target')){
    const heading=point.querySelector(':scope > .evidence-claim');if(!heading)continue;
    heading.before(make('p','evidence-context-label','선택한 설명'));
    for(const original of point.querySelectorAll(':scope > details')){
      const label=original.querySelector(':scope > summary');if(!label||!['원래 문장','원래 주장'].includes(label.textContent.trim()))continue;
      const text=[...original.children].filter(n=>n!==label).map(n=>n.textContent).join('');
      const same=normalize(text)===normalize(heading.textContent);
      const aliases=same?[...point.querySelectorAll(':scope > .source-origin-target')].filter(n=>n.dataset.selectedPoint===point.id&&normalize(n.querySelector(':scope > .evidence-claim')?.textContent||'')!==normalize(text)):[];
      if(same&&!aliases.length)original.remove();
      else{
        label.textContent='연결된 본문 문장';original.classList.remove('evidence-original');original.classList.add('evidence-connected-context','evidence-disclosure');
        if(same){original.classList.add('source-shared-context');sharedContextStyles.push('#'+CSS.escape(point.id)+':has(:is('+aliases.map(n=>'#'+CSS.escape(n.id)+':target').join(',')+')) > .source-shared-context{display:block}');}
      }
    }
    const metadata=make('details','source-metadata evidence-disclosure');metadata.append(make('summary','',point.classList.contains('source-point')?'관측 정보와 한계':'설명 연결 정보'));
    for(const child of [...point.children]){
      if(child.matches('.point-bar,.point-title,.evidence-context-label,.evidence-claim,.evidence-selection-context,.paragraph-source-picker,.source-path,.evidence-correspondence,.source-origin-target,.shared-source-target,.evidence-connected-context,[id],[data-statement-role]'))continue;
      if(!point.dataset.citedCode&&child.matches('.source-note'))continue;
      metadata.append(child);
    }
    if(metadata.children.length>1)point.append(metadata);
    for(const detail of point.querySelectorAll('details'))if(!detail.closest('.paragraph-source-picker'))detail.classList.add('evidence-disclosure');
  }
  const contextStyle=make('style');contextStyle.textContent=sharedContextStyles.join('\n');document.head.append(contextStyle);
  for(const code of appendix.querySelectorAll(':scope > .cited-code')){
    code.open=true;code.dataset.defaultExpanded='true';code.classList.add('evidence-disclosure');code.querySelector(':scope > summary').textContent='인용된 전체 코드';
  }
  const resizer=make('div','evidence-resize-handle');resizer.tabIndex=0;resizer.setAttribute('role','separator');resizer.setAttribute('aria-orientation','vertical');resizer.setAttribute('aria-label','근거 창 너비');resizer.title='끌어서 너비 조절 · 좌우 방향키 · Enter 기본 너비';appendix.prepend(resizer);

  // Use explicit reading-order numbers; nested section counter scopes previously
  // restarted at 01. Verification and original records have no article numbers.
  const headings=[...main.querySelectorAll('.article-section > .unit > h2.unit-text')];
  headings.forEach((h,i)=>h.parentElement.dataset.sectionNumber=String(i+1).padStart(2,'0'));
  const nav=required('.document-nav nav');nav.replaceChildren();
  nav.parentElement.dataset.overflow='true';
  function navList(title,items,fold=false){
    const group=make('div','work-toc-group');group.append(make('p','toc-caption',title));
    const list=make('ol',fold?'toc-records':'toc-primary');
    for(const [target,label,number] of items){
      if(!target.id){target.id=prefix+'-navigation-'+nav.querySelectorAll('a').length+'-'+list.children.length;target.tabIndex=-1;}
      const li=make('li'),a=make('a');a.href='#'+target.id;
      if(number)a.append(make('span','toc-number',number));a.append(make('span','',label));li.append(a);list.append(li);
    }
    group.append(list);nav.append(group);
  }
  navList('작업 설명',headings.map(h=>[h,h.textContent,h.parentElement.dataset.sectionNumber]));
  navList('검증과 한계',[[reportSection,'검증 보고'],[limits,'설명 한계']]);
  navList('원문과 근거',[[original,'원래 지시'],[evidence,'전체 근거'],[historyBox,'기록의 흐름'],[raw,'검증 원문']],true);

  // One SVG per scene; a native radio control also works without JavaScript.
  for(const figure of main.querySelectorAll('.admitted-diagram')){
    const svg=figure.querySelector('.diagram-full-view svg');if(!svg)continue;
    // Retire duplicate renderings and instructions. Existing source aliases return
    // to the same semantic element in the sole primary drawing.
    const timeline=figure.querySelector(':scope > .diagram-timeline');
    if(timeline){
      const redirects=new Map([[timeline.id,figure.id]]);
      for(const a of timeline.querySelectorAll('a.diagram-element[id]')){
        const primary=a.id.replace(':timeline-','-');
        if(!svg.querySelector('[id="'+primary+'"]'))throw Error('primary timeline role missing: '+primary);
        redirects.set(a.id,primary);
      }
      const retired=new Set([timeline,...timeline.querySelectorAll('[id]')].map(n=>n.id));
      for(const link of document.querySelectorAll('a[href^="#"]')){
        const id=link.getAttribute('href').slice(1);
        if(redirects.has(id))link.href='#'+redirects.get(id);
        else if(retired.has(id)&&!timeline.contains(link))throw Error('unmapped timeline reference: '+id);
      }
    }
    for(const extra of figure.querySelectorAll(':scope > .diagram-timeline,:scope > .diagram-relations,:scope > .diagram-alternative,.spatial-help'))extra.remove();
    figure.classList.add('work-overview');
    const viewport=svg.closest('.diagram-viewport');
    viewport.style.setProperty('--overview-width',svg.getAttribute('width')+'px');
    viewport.style.setProperty('--overview-height',svg.getAttribute('height')+'px');
    if(svg.classList.contains('connected-overview')){
      figure.classList.add('work-native-overview');
      for(const mode of ['wide','medium','narrow'])for(const axis of ['width','height'])viewport.style.setProperty('--overview-'+mode+'-'+axis,svg.style.getPropertyValue('--map-'+mode+'-'+axis));
    }
    const scales=make('fieldset','native-overview-controls');scales.append(make('legend','','도식 보기'));
    for(const [value,label] of [['fit','가로 맞춤'],['read','읽기 75%'],['full','100%']]){
      const l=make('label'),input=make('input');input.type='radio';input.name=figure.id+'-scale';input.dataset.nativeScale=value;input.defaultChecked=value==='fit';l.append(input,document.createTextNode(label));scales.append(l);
    }
    viewport.before(scales);
    const stage=make('div','diagram-stage');viewport.before(stage);stage.append(viewport);
    stage.prepend(scales);
    const navigation=figure.querySelector('.spatial-navigation');if(navigation)stage.prepend(navigation);
    const toggle=make('input','native-expand-toggle');toggle.type='checkbox';toggle.id=figure.id+'-expand';
    const expand=make('label','native-expand-control','↗');expand.htmlFor=toggle.id;expand.title='도식 크게 보기';expand.setAttribute('aria-label','도식 크게 보기');
    scales.append(toggle,expand);
    const header=make('header','reading-panel-header figure-panel-header'),close=make('label','panel-close','×');close.htmlFor=toggle.id;close.dataset.panelClose='';close.setAttribute('aria-label','도식 닫기');header.append(make('strong','','도식 크게 보기'),close);figure.prepend(header);
    const reset=figure.querySelector('[data-world-action="reset"]');if(reset){reset.textContent='읽기 75%';reset.setAttribute('aria-label','읽기 75%');}
    const fit=figure.querySelector('[data-world-action="fit"]');if(fit){fit.textContent='가로 맞춤';fit.setAttribute('aria-label','가로 맞춤');}
    if(navigation){
      const chooser=navigation.querySelector('details');chooser.querySelector('summary').textContent='이동';
      const menu=make('div','diagram-jump-menu'),pan=make('div','diagram-pan-controls');
      for(const action of ['left','right','up','down','focus']){const button=navigation.querySelector('[data-world-action="'+action+'"]');if(button)pan.append(button);}
      menu.append(pan,chooser.querySelector('ul'));chooser.append(menu);
      for(const [action,label] of [['fit','가로'],['reset','75%'],['expand','↗']]){const button=navigation.querySelector('[data-world-action="'+action+'"]');if(button){button.textContent=label;button.dataset.compactLabel=label;}}
    }
    for(const detail of figure.querySelectorAll(':scope > details'))detail.classList.add('work-disclosure');
  }
  doc.dataset.workLayout='grouped';
  document.body.classList.add('grouped-reading');
}
module.exports={arrangeWork};
