// Browser-side rearrangement of a frozen Work draft. No authoring or Runtime access.
function arrangeWork({work}) {
  const doc=document.querySelector(`#work-${work} .doc`);
  if(!doc?.dataset.editorialDraft)throw Error('an isolated editorial draft is required');
  const make=(tag,cls,text)=>{const n=document.createElement(tag);if(cls)n.className=cls;if(text)n.textContent=text;return n;};
  const reading=doc.querySelector('[id$="-reading"]'),prefix=reading.id.replace(/-reading$/,'');
  const required=selector=>{const n=doc.querySelector(selector);if(!n)throw Error('missing Work section: '+selector);return n;};
  const wrap=(title,id)=>{const d=make('details','reading-section work-record');if(id)d.id=id;d.append(make('summary','',title));return d;};
  const head=required('.doc-head'),verification=required('.verification-report'),limits=required('.limits-section');
  const audit=required('.audit'),original=required('.record-box:has([id$="-original"])');
  const evidence=required('.record-box:has([id$="-audit"])'),evidenceBody=evidence.querySelector('[id$="-audit"]');
  const next=[...doc.querySelectorAll('.article-section')].find(n=>n.querySelector('[data-question="SectionHeading"]')?.textContent==='남은 단계');
  const support=[...doc.querySelectorAll('.article-section')].find(n=>n.querySelector('[data-question="SectionHeading"]')?.textContent==='호출 연결과 원래 기록의 근거');
  if(!next||!support)throw Error('frozen authored sections missing');

  const main=make('section','work-reading-group');main.dataset.readingGroup='work';
  const checks=make('section','work-reading-group');checks.dataset.readingGroup='checks';checks.setAttribute('aria-label','검증과 설명 한계');
  const records=make('section','work-reading-group');records.dataset.readingGroup='records';records.setAttribute('aria-label','원문과 근거');
  doc.querySelector('.breadcrumb').after(main,checks,records);
  main.append(head,reading,next);
  checks.append(verification,limits);
  const limitsHeading=limits.querySelector('h2');limitsHeading.textContent='설명 한계';
  const limitsHeader=limitsHeading.parentElement===limits?limitsHeading:limitsHeading.parentElement;
  limitsHeader.after(required('.reading-warning'));
  original.querySelector(':scope > summary').textContent='원래 지시';
  evidence.querySelector(':scope > summary').textContent='전체 근거';
  records.append(original,evidence);

  // The three actual call connections explain the mechanism. Original Goal/report
  // provenance belongs to inspection. Every original unit and citation is moved intact.
  const mechanism=reading.querySelector('.article-section');
  for(const unit of [...support.querySelectorAll(':scope > .unit')]){
    if(unit.querySelector('[data-question="Narrative"]')&&!unit.querySelector('.unit-text').textContent.startsWith('원래 Goal'))mechanism.append(unit);
  }
  const editorial=wrap('편집 근거',prefix+'-editorial-basis');
  editorial.append(support);
  const retained=[...doc.querySelectorAll('details')].find(n=>n.querySelector(':scope > summary')?.textContent==='원래 도입과 편집 근거');
  if(retained)editorial.append(retained);
  const summaryBasis=wrap('요약 근거',prefix+'-summary-basis');
  const summaryPublication=evidence.querySelector('[data-publication-scope="summary"]');
  if(summaryPublication)summaryBasis.append(summaryPublication);
  const index=required('.evidence-index'),history=required('.recorded-timeline'),codeScope=required('.code-reading-scope');
  const historyBox=wrap('기록의 흐름',prefix+'-record-history');historyBox.append(history);
  evidenceBody.prepend(editorial,summaryBasis,index,historyBox,codeScope);

  // Report facts stay in the report. Command/capture/identifier disclosures and
  // the original structured read model are available together in the third group.
  const raw=wrap('검증 원문',prefix+'-verification-originals');
  const reportList=make('div','verification-report-list');
  for(const execution of [...verification.querySelectorAll(':scope > .verification-execution')]){
    const source=execution.dataset.executionSource;
    execution.id=prefix+'-execution-'+source;
    const row=make('section','verification-report-row');
    row.append(make('h3','',execution.querySelector('summary h3').textContent));
    for(const check of execution.querySelectorAll('.verification-check')){
      // Readable result, purpose, date, assertion and coverage are original facts.
      const summary=make('div','verification-check-summary');
      for(const child of check.children)if(child.matches('h4, dl, blockquote'))summary.append(child.cloneNode(true));
      row.append(summary);
    }
    const link=make('a','','명령·캡처 원문 보기');link.href='#'+execution.id;row.append(link);
    reportList.append(row);execution.open=false;raw.append(execution);
  }
  verification.append(reportList);
  const json=required('[id$="-full-verification"]');raw.append(json);records.append(raw);
  audit.remove();

  // Keep one clear level of categories under full evidence. Capture objects and
  // original data require their own deliberate disclosure, even after opening it.
  if(retained){
    const retainedSummary=retained.querySelector(':scope > summary');retainedSummary.remove();
    editorial.append(...retained.childNodes);retained.remove();
  }
  const recordData=wrap('기록 상태와 발행 정보',prefix+'-record-data');
  for(const child of [...evidenceBody.children])if(![editorial,summaryBasis,index,historyBox,codeScope].includes(child))recordData.append(child);
  evidenceBody.append(recordData);
  for(const pre of records.querySelectorAll('pre')){
    const disclosure=wrap(pre.classList.contains('publication-provenance')?'발행 정보 원문':'원문 데이터',pre.id?pre.id+'-disclosure':'');
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
  for(const actions of doc.querySelectorAll('.unit > .evidence-actions')){
    const unit=actions.parentElement,text=unit.querySelector(':scope > p.unit-text');
    const links=[...actions.querySelectorAll('a.evidence-link')];
    if(!text||!links.length)continue;
    rangePicker(links,unit.id);
    const action=make('a','evidence-link paragraph-evidence-action','근거 보기');action.href=links[0].getAttribute('href');action.dataset.origin=unit.id;
    action.dataset.citationTargets=JSON.stringify(links.map(n=>n.getAttribute('href')));
    action.setAttribute('aria-label','이 문단의 근거 '+links.length+'개 보기');actions.replaceChildren(action);
  }
  for(const claim of doc.querySelectorAll('.diagram-claim')){
    const links=[...claim.querySelectorAll('a.evidence-link')];if(links.length)rangePicker(links,claim.id);
  }
  for(const bar of appendix.querySelectorAll('.point-bar')){
    bar.classList.add('reading-panel-header');
    bar.prepend(make('strong','','근거'));
    const back=bar.querySelector('a.return');if(back){back.classList.add('panel-close');back.textContent='×';back.setAttribute('aria-label','근거 닫고 설명으로 돌아가기');}
  }

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
  navList('검증과 한계',[[verification,'검증 보고'],[limits,'설명 한계']]);
  navList('원문과 근거',[[original,'원래 지시'],[evidence,'전체 근거'],[raw,'검증 원문']],true);

  // One SVG per scene; a native radio control also works without JavaScript.
  for(const figure of main.querySelectorAll('.admitted-diagram')){
    const svg=figure.querySelector('.diagram-full-view svg');if(!svg)continue;
    figure.classList.add('work-overview');
    const viewport=svg.closest('.diagram-viewport');
    viewport.style.setProperty('--overview-width',svg.getAttribute('width')+'px');
    viewport.style.setProperty('--overview-height',svg.getAttribute('height')+'px');
    if(svg.classList.contains('connected-overview')){
      figure.classList.add('work-native-overview');
      for(const mode of ['wide','medium','narrow'])for(const axis of ['width','height'])viewport.style.setProperty('--overview-'+mode+'-'+axis,svg.style.getPropertyValue('--map-'+mode+'-'+axis));
    }
    const scales=make('fieldset','native-overview-controls');scales.append(make('legend','','도식 보기'));
    for(const [value,label] of [['fit','전체 보기'],['read','읽기 75%'],['full','100%']]){
      const l=make('label'),input=make('input');input.type='radio';input.name=figure.id+'-scale';input.dataset.nativeScale=value;input.defaultChecked=value==='fit';l.append(input,document.createTextNode(label));scales.append(l);
    }
    const help=make('p','spatial-help work-overview-help','전체 보기로 시작합니다. 작은 글은 읽기 75%나 확대 기능으로 확인하세요.');
    viewport.before(scales,help);
    const stage=make('div','diagram-stage');viewport.before(stage);stage.append(viewport);
    stage.prepend(scales);
    const navigation=figure.querySelector('.spatial-navigation');if(navigation)stage.prepend(navigation);
    const toggle=make('input','native-expand-toggle');toggle.type='checkbox';toggle.id=figure.id+'-expand';
    const expand=make('label','native-expand-control','↗');expand.htmlFor=toggle.id;expand.title='도식 크게 보기';expand.setAttribute('aria-label','도식 크게 보기');
    scales.append(toggle,expand);
    const header=make('header','reading-panel-header figure-panel-header'),close=make('label','panel-close','×');close.htmlFor=toggle.id;close.dataset.panelClose='';close.setAttribute('aria-label','도식 닫기');header.append(make('strong','','도식 크게 보기'),close);figure.prepend(header);
    const reset=figure.querySelector('[data-world-action="reset"]');if(reset){reset.textContent='읽기 75%';reset.setAttribute('aria-label','읽기 75%');}
    const fit=figure.querySelector('[data-world-action="fit"]');if(fit){fit.textContent='전체 보기';fit.setAttribute('aria-label','전체 보기');}
    const cameraHelp=figure.querySelector('.spatial-camera-help');if(cameraHelp)cameraHelp.textContent='빈 배경 끌기·방향 버튼/키로 이동합니다. 전체 보기에서 위치를 파악하고, 읽기 75% 또는 선택 관계로 확대해 읽으세요. Tab과 Enter로 근거를 엽니다. Esc로 돌아갑니다.';
    const helpBox=make('details','diagram-help work-disclosure');helpBox.append(make('summary','','도식 조작 안내'));for(const p of figure.querySelectorAll('.spatial-help'))helpBox.append(p);figure.querySelector('.diagram-full-view').after(helpBox);
    if(navigation){
      const chooser=navigation.querySelector('details');chooser.querySelector('summary').textContent='이동';
      const menu=make('div','diagram-jump-menu'),pan=make('div','diagram-pan-controls');
      for(const action of ['left','right','up','down','focus']){const button=navigation.querySelector('[data-world-action="'+action+'"]');if(button)pan.append(button);}
      menu.append(pan,chooser.querySelector('ul'));chooser.append(menu);
      for(const [action,label] of [['fit','전체'],['reset','75%'],['expand','↗']]){const button=navigation.querySelector('[data-world-action="'+action+'"]');if(button){button.textContent=label;button.dataset.compactLabel=label;}}
    }
    for(const detail of figure.querySelectorAll(':scope > details'))detail.classList.add('work-disclosure');
  }
  doc.dataset.workLayout='grouped';
  document.body.classList.add('grouped-reading');
}
module.exports={arrangeWork};
