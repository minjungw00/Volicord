// Browser-side rearrangement of a frozen Work draft. No authoring or Runtime access.
function arrangeWork({work}) {
  const doc=document.querySelector(`#work-${work} .doc`);
  if(!doc?.dataset.editorialDraft)throw Error('an isolated editorial draft is required');
  const make=(tag,cls,text)=>{const n=document.createElement(tag);if(cls)n.className=cls;if(text)n.textContent=text;return n;};
  const reading=doc.querySelector('[id$="-reading"]'),prefix=reading.id.replace(/-reading$/,'');
  const required=selector=>{const n=doc.querySelector(selector);if(!n)throw Error('missing Work section: '+selector);return n;};
  const wrap=(title,id)=>{const d=make('details','reading-section work-record');d.id=id;d.append(make('summary','',title));return d;};
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

  // A single paragraph action opens its complete citation set, rather than
  // treating the first citation as evidence for the entire paragraph.
  const appendix=document.querySelector('.appendix');
  for(const actions of doc.querySelectorAll('.unit > .evidence-actions')){
    const unit=actions.parentElement,text=unit.querySelector(':scope > p.unit-text');
    const links=[...actions.querySelectorAll('a.evidence-link')];
    if(!text||!links.length)continue;
    const group=make('section','paragraph-evidence-target');group.id=unit.id+'-evidence';group.tabIndex=-1;
    const bar=make('div','point-bar'),back=make('a','return','← 설명으로 돌아가기');back.href='#'+unit.id;bar.append(back);
    group.append(bar,make('h2','evidence-claim',text.textContent));
    const list=make('ol','paragraph-evidence-list');
    for(const link of links){
      const target=document.getElementById(link.getAttribute('href').slice(1));if(!target)throw Error('citation target missing');
      const item=make('li'),path=target.querySelector('.source-path')||target.closest('.source-point')?.querySelector('.source-path');
      item.append(link);if(path)item.append(make('p','source-note',path.textContent));list.append(item);
      const pointBar=target.querySelector(':scope > .point-bar');
      if(pointBar){const switcher=make('a','paragraph-evidence-switch','이 문단의 근거 목록');switcher.href='#'+group.id;pointBar.append(switcher);}
    }
    group.append(list);appendix.append(group);
    const action=make('a','evidence-link paragraph-evidence-action','근거 보기');action.href='#'+group.id;action.dataset.origin=unit.id;
    action.setAttribute('aria-label','이 문단의 근거 '+links.length+'개 보기');actions.replaceChildren(action);
  }

  // Use explicit reading-order numbers; nested section counter scopes previously
  // restarted at 01. Verification and original records have no article numbers.
  const headings=[...main.querySelectorAll('.article-section > .unit > h2.unit-text')];
  headings.forEach((h,i)=>h.parentElement.dataset.sectionNumber=String(i+1).padStart(2,'0'));
  const nav=required('.document-nav nav');nav.replaceChildren();
  function navList(title,items,fold=false){
    const group=make(fold?'details':'div','work-toc-group');group.append(make(fold?'summary':'p','toc-caption',title));
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
    const reset=figure.querySelector('[data-world-action="reset"]');if(reset){reset.textContent='읽기 75%';reset.setAttribute('aria-label','읽기 75%');}
    const fit=figure.querySelector('[data-world-action="fit"]');if(fit){fit.textContent='전체 보기';fit.setAttribute('aria-label','전체 보기');}
    const cameraHelp=figure.querySelector('.spatial-camera-help');if(cameraHelp)cameraHelp.textContent='빈 배경 끌기·방향 버튼/키로 이동합니다. 전체 보기에서 위치를 파악하고, 읽기 75% 또는 선택 관계로 확대해 읽으세요. Tab과 Enter로 근거를 엽니다. Esc로 돌아갑니다.';
  }
  doc.dataset.workLayout='grouped';
}
module.exports={arrangeWork};
