// Coupled V11 supporting driver. Automation evaluation never becomes Product JS.
'use strict';
const fs = require('node:fs');
const crypto = require('node:crypto');
const sha256 = bytes => crypto.createHash('sha256').update(bytes).digest('hex');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const mode = process.argv[3];
if (fs.existsSync(path.join(config.output,`${mode}-result.json`))) throw new Error('Browser results are create-only; use a fresh output directory');
const {chromium} = require(config.playwright);
const result = {kind: 'viewer_browser_observations', mode, status: 'not_run', checks: [], requests: [], timings: [], zoom: [], human_acceptance: 'not_established'};
let context, page, worker, attached;
const F = config.fixture;
const idSelector = id => `[id="${id}"]`;
const workSelector = key => idSelector(`work-${F.goals[key]}`);
function requireFact(ok, reason) { if (!ok) throw new Error(reason); }
async function check(id, fn) {
  try { const evidence = await fn(); result.checks.push({id, status: 'passed', evidence: evidence ?? null}); }
  catch (error) {
    result.checks.push({id, status: 'failed', reason: String(error)});
    console.log(JSON.stringify(result.checks.at(-1)));
    if (page && !page.isClosed()) await page.screenshot({path: path.join(config.output, `${mode}-failure-${result.checks.length}.png`)}).catch(() => {});
  }
}
async function frames() { await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)))); }
async function paint() {
  await page.waitForFunction(()=>performance.getEntriesByName('first-contentful-paint').length>0);
  const observed=await page.evaluate(()=>({time_origin_ms:performance.timeOrigin,entries:performance.getEntriesByType('paint').map(e=>({name:e.name,start_ms:e.startTime,duration_ms:e.duration}))}));
  requireFact(observed.entries.some(e=>e.name==='first-contentful-paint' && e.start_ms>0),'browser_paint_timing_unavailable');
  return observed;
}
async function capture(name, target=page.locator('main > section').first()) {
  // Capture the current browser viewport, including native zoom, at the reading
  // surface. Surface screenshots can be blank at deep offsets with tab zoom.
  if (await target.count()) await target.first().evaluate(e=>window.scrollTo({left:0,top:e.getBoundingClientRect().top+scrollY,behavior:'instant'}));
  // Native keyboard focus scrolling may continue after disclosure frames. Wait
  // for a stable viewport before observing; still reject any movement during
  // the screenshot itself. Do not alter browser scrolling or snapshot geometry.
  await page.evaluate(async()=>{
    await document.fonts.ready;
    await new Promise((resolve,reject)=>{
      let previous='',stable=0,frames=0;
      function sample() {
        const geometry=JSON.stringify([scrollX,scrollY,innerWidth,innerHeight,devicePixelRatio]);
        stable=geometry===previous?stable+1:0;
        previous=geometry;
        if(stable>=6)return resolve();
        if(++frames>=120)return reject(new Error('viewport_did_not_settle'));
        requestAnimationFrame(sample);
      }
      requestAnimationFrame(sample);
    });
  });
  const observed=await page.evaluate(()=>{
    const meta=document.querySelector('meta[name="volicord-observation"]');
    return {context:meta?JSON.parse(meta.content):null,dom:document.documentElement.outerHTML};
  });
  if(mode==='offline') {
    requireFact(page.url().startsWith('file:')&&observed.context===null,'snapshot_has_live_observation_context');
  } else {
    requireFact(observed.context?.process?.executable_sha256 === config.viewer_sha256,'displayed_candidate_executable_mismatch');
  }
  const observedUrl=page.url();
  const beforeHash=sha256(observed.dom);
  const geometry=await page.evaluate(()=>({scrollX,scrollY,innerWidth,innerHeight,dpr:devicePixelRatio}));
  const session=await context.newCDPSession(page);
  try {
    const image=await session.send('Page.captureScreenshot',{format:'png',fromSurface:false});
    fs.writeFileSync(path.join(config.output,name),Buffer.from(image.data,'base64'));
  } finally {await session.detach();}
  const afterHash=sha256(await page.evaluate(()=>document.documentElement.outerHTML));
  const afterGeometry=await page.evaluate(()=>({scrollX,scrollY,innerWidth,innerHeight,dpr:devicePixelRatio}));
  requireFact(beforeHash===afterHash&&observedUrl===page.url()&&JSON.stringify(geometry)===JSON.stringify(afterGeometry),'display_changed_during_capture');
  const screenshot=fs.readFileSync(path.join(config.output,name));
  const receipt={kind:'dogfood_viewer_display_capture',schema_version:1,evidence_class:'browser_display_capture',
    candidate_head:config.candidate_head,url:page.url(),context:observed.context,dom_sha256:beforeHash,
    screenshot:{path:name,sha256:sha256(screenshot),bytes:screenshot.length},
    browser:{version:context.browser().version(),geometry,zoom:result.zoom.at(-1)||null}};
  // Offline snapshots have no live process/render authority. Keep their image
  // and stability observation without manufacturing a live display receipt.
  if(mode!=='offline')(result.display_captures ??= []).push(receipt);
  (result.captures ??= []).push({path:name,mechanism:'Page.captureScreenshot fromSurface=false; browser viewport',...geometry});
}
async function go(url) {
  const response = await page.goto(url, {waitUntil: 'load'});
  if (response) requireFact(response.status() === 200, `product_http_${response.status()}`);
  await frames();
  const navigation = await page.evaluate(() => {const n = performance.getEntriesByType('navigation')[0]; return n ? {response_start_ms:n.responseStart,response_end_ms:n.responseEnd,dom_complete_ms:n.domComplete,load_ms:n.loadEventEnd} : null;});
  result.timings.push({kind: 'navigation', url, ...navigation,paint:await paint(), note: 'Native PaintTiming marks are measured from navigation start; response completion includes transport; separate Rust render stage profiles are recorded by the parent'});
}
async function zoom(factor) {
  const before = await page.evaluate(() => ({innerWidth, dpr:devicePixelRatio, visualScale:visualViewport.scale,cssZoom:getComputedStyle(document.documentElement).zoom}));
  const actual = await worker.evaluate(async ({url, factor}) => {
    const tabs = await chrome.tabs.query({});
    const tab = tabs.find(t => t.url === url);
    if (!tab) throw new Error('Cannot identify test-owned tab');
    await chrome.tabs.setZoomSettings(tab.id, {mode:'automatic',scope:'per-tab'});
    await chrome.tabs.setZoom(tab.id, factor);
    return {factor:await chrome.tabs.getZoom(tab.id), settings:await chrome.tabs.getZoomSettings(tab.id)};
  }, {url:page.url(), factor});
  await frames();
  const after = await page.evaluate(() => ({innerWidth, dpr:devicePixelRatio, visualScale:visualViewport.scale, cssZoom:getComputedStyle(document.documentElement).zoom}));
  requireFact(actual.factor === factor && after.cssZoom === '1' && after.visualScale === 1, 'actual_browser_zoom_not_established');
  if (factor === 2) requireFact(Math.abs(after.dpr / before.dpr - 2) < 0.01 && Math.abs(after.innerWidth * 2 - before.innerWidth) <= 1, 'browser_zoom_geometry_mismatch');
  const evidence = {mechanism:'chrome.tabs.setZoom; automatic per-tab browser zoom, not CSS/device emulation',before,after,...actual};
  result.zoom.push(evidence);
  return evidence;
}
async function overflow() {
  const values = await page.evaluate(() => ({client:document.documentElement.clientWidth,scroll:document.documentElement.scrollWidth,
    graphs:[...document.querySelectorAll('.grounded-diagram')].map(e => ({client:e.clientWidth,scroll:e.scrollWidth,overflow:getComputedStyle(e).overflowX}))}));
  requireFact(values.scroll <= values.client + 1, `ordinary_page_overflow:${JSON.stringify(values)}`);
  requireFact(values.graphs.every(g => g.overflow === 'auto' || g.overflow === 'scroll'), 'graph_scroll_is_not_scoped');
  return values;
}
async function fragments() {
  const facts = await page.evaluate(() => {
    const ids = [...document.querySelectorAll('[id]')].map(e => e.id);
    const links = [...document.querySelectorAll('a[href^="#"]')].map(e => e.getAttribute('href').slice(1));
    for(const element of document.querySelectorAll('*'))for(const attr of element.attributes)for(const match of attr.value.matchAll(/url\(#([^)]+)\)/g))links.push(match[1]);
    return {duplicates:ids.filter((id,i) => ids.indexOf(id) !== i),missing:links.filter(id => !document.getElementById(id)),count:links.length};
  });
  requireFact(!facts.duplicates.length && !facts.missing.length, `fragment_target:${JSON.stringify(facts)}`);
  return facts;
}
async function keyboardReach(locator) {
  requireFact(await locator.count() === 1, 'keyboard_target_not_unique');
  // Reach by Tab/Shift+Tab, never locator.focus(). Browser may also tab graph scroll regions.
  await page.keyboard.press('Control+Home');
  for (let n=0;n<900;n++) {
    await page.keyboard.press('Tab');
    if (await locator.evaluate(e => e === document.activeElement)) {
      const focus = await locator.evaluate(e => {const s = getComputedStyle(e); return {outline:s.outlineStyle,width:s.outlineWidth,color:s.outlineColor,rect:e.getBoundingClientRect().toJSON()};});
      requireFact(focus.outline !== 'none' && parseFloat(focus.width) > 0, 'keyboard_focus_not_visible');
      return {tabs:n+1,...focus};
    }
  }
  throw new Error('keyboard_target_unreachable');
}
async function activate(locator, navigation=true) {
  const focus = await keyboardReach(locator);
  const inputEpoch=await page.evaluate(()=>performance.timeOrigin+performance.now());
  const started = performance.now();
  if (navigation) await Promise.all([page.waitForEvent('load'), page.keyboard.press('Enter')]);
  else await page.keyboard.press('Enter');
  await frames();
  const duration = performance.now() - started;
  const timing={kind: navigation ? 'keyboard_navigation_paint' : 'keyboard_disclosure_two_frames',two_frame_duration_ms:duration,note:'Automation input through two requestAnimationFrame callbacks; native navigation PaintTiming is separate. Disclosure frames do not measure incremental paint completion or human responsiveness.'};
  if(navigation) {
    timing.paint=await paint();
    const first=timing.paint.entries.find(e=>e.name==='first-contentful-paint');
    timing.input_to_contentful_paint_ms=timing.paint.time_origin_ms+first.start_ms-inputEpoch;
    requireFact(timing.input_to_contentful_paint_ms>=0,'navigation_paint_precedes_keyboard_input');
  }
  result.timings.push(timing);
  return focus;
}
// Ordinary reading excludes every audit disclosure, hidden node and non-rendered
// text. Off-viewport content is retained because native scrolling can reveal it.
async function ordinaryText(locator) {
  return locator.evaluate(root => {
    const walker=document.createTreeWalker(root,NodeFilter.SHOW_TEXT);
    const text=[];
    while(walker.nextNode()) {
      const node=walker.currentNode;
      let shown=true;
      for(let e=node.parentElement;e;e=e.parentElement) {
        const style=getComputedStyle(e);
        if(e.tagName==='DETAILS'||e.hidden||e.getAttribute('aria-hidden')==='true'||
           style.display==='none'||style.visibility!=='visible'||Number(style.opacity)===0) {shown=false;break;}
      }
      const range=document.createRange();range.selectNodeContents(node);
      if(shown&&range.getClientRects().length)text.push(node.textContent);
    }
    return text.join(' ').replace(/\s+/g,' ').trim();
  });
}
function claimGroups(body, groups, reason) {
  for(const terms of groups)requireFact(terms.some(t=>body.toLowerCase().includes(t.toLowerCase())),`${reason}:${terms}`);
}
async function readingHierarchy(state, locale) {
  requireFact(await page.locator('nav[aria-label="Viewer"] a[aria-current="page"]').count()===1,'current_navigation_ambiguous');
  requireFact(await page.locator('#limitations').count()===0,'global_limitations_dominate_reading');
  const ordinary=await ordinaryText(page.locator('main'));
  requireFact(!ordinary.includes('integrity diagnostics not requested'),'materialization_detail_as_warning');
  if(state==='default') {
    const heading=await page.locator('main > section').first().getAttribute('id');
    requireFact(heading==='overview','project_purpose_not_first');
    for(const card of await page.locator('#overview .work-summary').all()) {
      requireFact(await card.locator('h4 a').count()===1,'work_title_not_distinct');
      requireFact(await card.locator('.work-state .badge').count()===1,'work_status_missing');
      requireFact(await card.locator('.next-action[data-question]').count()===1,'work_next_action_missing');
      requireFact(await card.locator('details').count()===0,'list_expands_audit_or_full_explanation');
    }
  }
  if(state==='analysis') {
    const status=page.locator('#health .analysis-summary');
    requireFact(await status.count()===1,'analysis_reading_missing');
    requireFact(['absent','current','partial','stale','failed','unknown','unavailable'].includes(await status.getAttribute('data-analysis-state')),'analysis_state_unknown');
    requireFact(await status.locator('[data-analysis-freshness]').count()===1,'freshness_conflated_with_coverage');
    requireFact((await ordinaryText(status)).includes('volicord analyze'),'refresh_guidance_missing');
    requireFact(await page.locator('.runtime-diagnostics[open],.global-diagnostics[open]').count()===0,'global_diagnostics_not_disclosed');
  }
  return {hierarchy:state,locale,diagnostics:'explicit_disclosure'};
}
async function overviewFacts(locale) {
  const overview=page.locator('#overview');
  const body=await ordinaryText(overview);
  requireFact(body.includes(config.scenario.purpose_present),'purpose_not_source_grounded');
  const categories=await overview.locator('.category-count').allTextContents();
  requireFact(categories.length===3,'overview_category_counts_missing');
  for(const count of categories) {
    const values=count.match(/\d+/g)?.map(Number);
    requireFact(values?.length===3 && values[0]===values[1]+values[2],'overview_omissions_dishonest');
  }
  for(const key of ['older','same_title','goal_only']) {
    const card=overview.locator(`[data-work-id="${F.goals[key]}"]`).first();
    if(key==='goal_only' && !await card.count()) {requireFact(Number(categories[2].match(/\d+/g)?.[2])>0,'overview_omitted_goal_without_count');continue;}
    requireFact(await card.count()===1,`overview_work_missing:${key}`);
    const text=await ordinaryText(card);
    const expected=config.scenario.works.find(w=>w.key===key);
    const state=expected.checkpoints.at(-1)?.work||'Open';
    const labels=locale==='en'?{Completed:'completed',Paused:'paused',Open:'open',InProgress:'in progress'}:{Completed:'완료',Paused:'일시 중지',Open:'열림',InProgress:'진행 중'};
    requireFact(text.includes(labels[state]),`overview_state_missing:${key}`);
    if(key==='older')requireFact(text.includes(locale==='en'?'historical':'과거'),'overview_promotes_historical_pass');
  }
  return {categories,works:['older','same_title','goal_only']};
}
async function workFacts(key, locale) {
  const section = page.locator(workSelector(key));
  requireFact(await section.count() === 1, 'work_exact_identity');
  const text = await ordinaryText(section);
  const expected = config.scenario.works.find(w => w.key === key);
  requireFact(text.includes(expected.title), 'work_title_source_mismatch');
  const latest = expected.checkpoints.at(-1);
  if (latest) {
    const next=section.locator('details').filter({has:page.locator('summary')}).filter({hasText:latest.next_step});
    requireFact(await next.count()>0, 'cross_work_next_step_substitution');
    const state = await ordinaryText(section.locator('.fact-states').first());
    const labels = locale === 'en'
      ? {Completed:'completed',Paused:'paused',InProgress:'in progress',Passed:'passed',Failed:'failed',NotRun:'not run',Pending:'pending',NotRequested:'not requested',Reviewed:'reviewed',Accepted:'accepted',Rejected:'rejected'}
      : {Completed:'완료',Paused:'일시 중지',InProgress:'진행 중',Passed:'통과',Failed:'실패',NotRun:'실행하지 않음',Pending:'대기 중',NotRequested:'요청하지 않음',Reviewed:'검토됨',Accepted:'수락됨',Rejected:'거부됨'};
    for (const fact of [latest.work, latest.verification, latest.review, latest.acceptance]) requireFact(state.includes(labels[fact]), `work_state_source_mismatch:${fact}`);
    for (const keyName of config.expected.exact_older.excluded_decision_keys) if (key === 'older') requireFact(!await section.locator(`a[href*="decision=${F.decisions[keyName]}"]`).count(), 'cross_work_decision_substitution');
    if (key === 'older') {
      requireFact(text.includes(labels.Failed) && text.includes(labels.Rejected) && text.includes(labels.Pending), 'historical_failure_or_pending_state_hidden');
      requireFact(text.includes(locale === 'en' ? 'Interpretation has not been generated' : '해석이 아직 생성되지'), 'summary_limit_hidden');
      const source = await section.textContent();
      for (const cp of config.expected.exact_older.checkpoint_keys) requireFact(source.includes(F.checkpoints[cp]), 'checkpoint_basis_missing');
      for (const decision of ['explicit','project','unresolved']) requireFact(await section.locator(`a[href*="decision=${F.decisions[decision]}"]`).count() > 0, 'decision_scope_link_missing');
    }
  } else requireFact(text.includes(locale === 'en' ? 'Goal-only Work is open' : 'Goal만 있는 작업은 열림'), 'goal_only_gap_hidden');
  return {identity:F.goals[key],latest:latest ? [latest.work,latest.verification,latest.review,latest.acceptance] : 'Goal only'};
}
async function labels() {
  const facts = await page.evaluate(() => [...document.querySelectorAll('g.diagram-node')].map(e => {
    const rect = e.querySelector('rect').getBBox();
    const texts = [...e.querySelectorAll('text')].map(t => ({text:t.textContent,box:t.getBBox()}));
    return {id:e.dataset.entityId,display:texts.map(t=>t.text).join(''),aria:e.closest('a').getAttribute('aria-label'),fit:texts.every(t=>t.box.x >= rect.x-1 && t.box.x+t.box.width <= rect.x+rect.width+1 && t.box.y >= rect.y-1 && t.box.y+t.box.height <= rect.y+rect.height+1)};
  }));
  requireFact(facts.length > 0, 'no_grounded_nodes');
  for (const suffix of ['alpha','beta']) {
    const node = facts.find(n => n.aria.includes(`distinguishing_labels_${suffix}`));
    requireFact(node && node.display.includes(suffix), `common_prefix_truncation:${suffix}`);
  }
  requireFact(facts.every(n=>n.fit), `svg_label_outside_node:${JSON.stringify(facts.filter(n=>!n.fit))}`);
  return facts;
}
async function graphIdentity() {
  const evidence = await page.evaluate(() => ({nodes:[...document.querySelectorAll('g.diagram-node')].map(e=>({id:e.dataset.entityId,snapshot:e.dataset.analysisSnapshot})),edges:[...document.querySelectorAll('g.diagram-edge')].map(e=>({id:e.dataset.relationId,source:e.dataset.sourceEntity,target:e.dataset.targetEntity})),lists:[...document.querySelectorAll('details.relationship')].map(e=>e.dataset.relationId)}));
  const relations = new Map(F.relations.map(r=>[r.identity,r]));
  for (const n of evidence.nodes) requireFact(F.entities.some(e=>e.id===n.id) && n.snapshot === F.analysis_snapshot, 'graph_entity_basis_mismatch');
  for (const e of evidence.edges) {
    const source = relations.get(e.id);
    requireFact(source && source.source_entity===e.source && source.target.resolution==='resolved_entity' && source.target.target===e.target && evidence.lists.includes(e.id), 'graph_relation_basis_mismatch');
  }
  return evidence;
}
async function evidenceOpen(root=page.locator('main')) {
  // Native keyboard disclosures, including nested original evidence.
  const summaries = root.locator('details > summary');
  const count = await summaries.count();
  for (let n=0;n<Math.min(count,4);n++) {
    const summary = summaries.nth(n);
    if (await summary.isVisible() && !await summary.evaluate(e=>e.parentElement.open)) {
      await activate(summary, false);
      requireFact(await summary.evaluate(e=>e.parentElement.open), 'native_disclosure_failed');
    }
  }
  return {disclosures:count,...await overflow()};
}
async function copyMutation(name, mutate, verify, positiveUrl) {
  await go(positiveUrl);
  await verify();
  await page.evaluate(mutate);
  const file = path.join(config.output, `${mode}-mutation-${name}.html`);
  fs.writeFileSync(file, await page.content());
  await go(pathToFileURL(file).href);
  let failure;
  try { await verify(); } catch (error) { failure = String(error); }
  requireFact(failure, `negative_control_not_detected:${name}`);
  // Assert the intended failure, not merely a navigation/shape failure.
  const expected = {wrapping:'ordinary_page_overflow',prefix:'common_prefix_truncation',fragment:'fragment_target',substitution:'cross_work_next_step_substitution',live_link:'snapshot_live_link',historical:'overview_promotes_historical_pass',flow:'graph_relation_basis_mismatch'}[name];
  requireFact(failure.includes(expected), `wrong_negative_control_reason:${failure}`);
  await page.screenshot({path:path.join(config.output, `${mode}-negative-${name}.png`)});
  await go(positiveUrl);
  await verify();
  return {detected:true,reason:failure,restored_positive:'passed',copy:file};
}
async function snapshotSafety() {
  const facts = await page.evaluate(() => ({active:document.querySelectorAll('script,form,input,button,iframe,object,embed').length,
    links:[...document.querySelectorAll('[href],[src],[action],[srcset],[formaction]')].flatMap(e=>[...e.attributes].filter(a=>['href','src','action','srcset','formaction'].includes(a.name)).map(a=>a.value)).filter(v=>!v.startsWith('#')),
    events:[...document.querySelectorAll('*')].flatMap(e=>[...e.attributes].filter(a=>a.name.startsWith('on')).map(a=>a.name)),token:document.documentElement.outerHTML.includes('request_authenticity')}));
  requireFact(!facts.links.length, `snapshot_live_link:${JSON.stringify(facts.links)}`);
  requireFact(!facts.active && !facts.events.length && !facts.token, 'snapshot_active_content');
  await fragments();
  return facts;
}
async function live() {
  const url = (view, locale, fields={}) => config.url+'?'+new URLSearchParams({view,locale,language:locale,...fields});
  const entity = F.entities.find(e=>e.path==='unsafe<&>.py' && e.name==='unsafe<&>');
  requireFact(entity, 'source_fixture_alpha_not_analyzed');
  const routes = locale => ({default:config.url+'?locale='+locale,work:url('work',locale,{work:F.goals.older}),decision:url('decisions',locale,{decision:F.decisions.explicit}),detail:url('code',locale,{scope:'repository',entity:entity.id}),analysis:url('tools',locale,{tool:'status'}),catalog:url('work',locale)});
  for (const locale of ['en','ko']) {
    for (const width of [390,768,1440]) {
      await page.setViewportSize({width,height:900});
      for (const factor of [1,2]) {
        for (const [state,address] of Object.entries(routes(locale))) {
          const reading=page.locator(state==='default'?'#overview':state==='work'?workSelector('older'):state==='decision'?idSelector(`decision-${F.decisions.explicit}`):state==='analysis'?'#health':state==='catalog'?'#works':'#code');
          await check(`${locale}-${width}-${factor}-${state}`, async()=>{
            await go(address);
            await zoom(1);
            if (factor===2) await zoom(2);
            requireFact(await page.locator('html').getAttribute('lang') === locale, 'bundled_locale_mismatch');
            await fragments();
            requireFact(await page.locator('script,[onclick],[onerror]').count()===0,'escaping_security_boundary');
            await readingHierarchy(state,locale);
            if (state==='default') await overviewFacts(locale);
            if (state==='work') await workFacts('older',locale);
            if (state==='decision') {
              const section = page.locator(idSelector(`decision-${F.decisions.explicit}`));
              const body = await ordinaryText(section);
              requireFact(body.includes(locale==='en'?'Agent recommendation: Remote':'에이전트 권고: Remote'), 'recommendation_attribution_missing');
              requireFact(body.includes(locale==='en' ? 'User rationale is not recorded' : '사용자 근거가 기록되지 않았습니다'), 'missing_user_rationale_fabricated');
              const stateText=await ordinaryText(section);
              requireFact(stateText.includes(locale==='en' ? 'review required' : '검토 필요'), 'review_due_hidden');
              requireFact(stateText.includes('Local') && body.includes('Remote'),'user_choice_and_recommendation_confused');
              requireFact(await section.locator('xpath=..').getAttribute('data-decision-scope')==='work-item' && body.includes(locale==='en'?'Declared Work scope':'선언된 작업 범위') && body.includes('native/query.c'),'decision_scope_source_mismatch');
              const retained=await section.textContent();
              requireFact([F.decision_sources.explicit.user,...F.decision_sources.explicit.recommendation].every(id=>retained.includes(id)),'decision_source_basis_missing');
            }
            if (state==='detail') {await labels(); await graphIdentity();}
            const geometry = await overflow();
            await capture(`${locale}-${width}-${factor}-${state}.png`,reading);
            return geometry;
          });
          await check(`${locale}-${width}-${factor}-${state}-evidence-open`, async()=>{
            const evidence = await evidenceOpen(reading);
            const opened=reading.locator('details[open] > summary').first();
            await capture(`${locale}-${width}-${factor}-${state}-evidence.png`,await opened.count()?opened:reading);
            return evidence;
          });
        }
      }
    }
    await page.setViewportSize({width:768,height:900});
    await check(`keyboard-views-${locale}`, async()=>{
      await go(routes(locale).default);await zoom(1);
      const evidence=[];
      for (const view of ['work','code','decisions','overview']) evidence.push(await activate(page.locator(`nav[aria-label="Viewer"] a[href*="view=${view}"]`)));
      for(const tool of ['status','documents']) evidence.push(await activate(page.locator(`nav[aria-label="Viewer"] a[href*="tool=${tool}"]`)));
      return evidence;
    });
    await check(`keyboard-paged-old-work-${locale}`, async()=>{
      await go(url('work',locale));
      requireFact(!await page.locator(`a[href*="work=${F.goals.older}"]`).count(), 'old_work_not_beyond_list_bound');
      await activate(page.locator('nav a[href*="page=1"]'));
      await activate(page.locator(`a[href*="work=${F.goals.older}"]`).first());
      await workFacts('older',locale);
      await activate(page.locator(`nav[aria-label="Viewer"] a[href*="scope=work"][href*="work=${F.goals.older}"]`));
      requireFact((await page.locator('#code').innerText()).includes('python/worker.py'), 'work_source_scope_missing');
    });
    await check(`distinct-work-${locale}`, async()=>{await go(url('work',locale,{work:F.goals.same_title}));return workFacts('same_title',locale);});
    await check(`goal-only-${locale}`, async()=>{await go(url('work',locale,{work:F.goals.goal_only}));return workFacts('goal_only',locale);});
    await check(`keyboard-graph-source-${locale}`, async()=>{
      await go(routes(locale).detail);
      const anchor=page.locator('svg a').filter({has:page.locator(`g[data-entity-id="${entity.id}"]`)}).first();
      await activate(anchor,false);
      const target=await page.evaluate(()=>location.hash.slice(1));
      requireFact(target && await page.locator(idSelector(target)).count()===1, 'svg_fragment_navigation_failed');
      await activate(page.locator(idSelector(target)+' > summary'),false);
      const relationship=page.locator('details.relationship > summary').first();
      await activate(relationship,false);
      const open=page.locator('details.relationship[open]').first();
      const actual=await open.getAttribute('data-relation-id');
      requireFact(F.relations.some(r=>r.identity===actual),'relationship_identity_missing');
      await activate(open.locator('a').first());
      const source=page.locator('summary').filter({hasText:locale==='en'?'Source locator and retained range':'Source 위치 및 보존된 범위'});
      await activate(source,false);
      const selected=F.entities.find(e=>e.id===new URL(page.url()).searchParams.get('entity'));
      const retained=await source.locator('..').innerText();
      requireFact(selected && retained.includes(F.analysis_snapshot) && retained.includes(selected.source.identity) && retained.includes(selected.path),'source_snapshot_locator_basis_missing');
      requireFact(retained.includes(F.repository_snapshot),'source_repository_snapshot_basis_missing');
      const coordinates={};
      for(const point of ['start','end']) {
        const match=retained.match(new RegExp(`${point}:\\s*\\w+\\s*\\{\\s*line:\\s*(\\d+),\\s*column:\\s*(\\d+)\\s*\\}`));
        requireFact(match && Number(match[1])===selected.range[point].line && Number(match[2])===selected.range[point].column,'source_range_basis_mismatch');
        coordinates[point]={line:Number(match[1]),column:Number(match[2])};
      }
      return {entity:selected.id,source:selected.source.identity,analysis_snapshot:F.analysis_snapshot,repository_snapshot:F.repository_snapshot,path:selected.path,coordinates};
    });
    await check(`graph-local-scroll-${locale}`,async()=>{
      await go(routes(locale).detail);await page.setViewportSize({width:390,height:900});
      const graph=page.locator('.grounded-diagram').first();
      requireFact(await graph.evaluate(e=>e.scrollWidth>e.clientWidth),'narrow_graph_not_scrollable');
      await graph.evaluate(e=>e.scrollIntoView({block:'start'}));
      const box=await graph.boundingBox();await page.mouse.move(box.x+box.width/2,Math.min(850,Math.max(10,box.y+60)));await page.mouse.wheel(350,0);await page.waitForFunction(()=>document.querySelector('.grounded-diagram').scrollLeft>0);await frames();
      requireFact(await graph.evaluate(e=>e.scrollLeft>0),'graph_wheel_scroll_failed');
      return overflow();
    });
  }
  for (const locale of ['en','ko']) {
    await check(`stale-analysis-${locale}`,async()=>{
      const source=path.join(F.repository,'native/query.c');const original=fs.readFileSync(source);
      try {
        fs.appendFileSync(source,'\n/* browser stale fixture */\n');
        await go(url('code',locale,{scope:'work',work:F.goals.older}));
        const body=await page.locator('main').innerText();
        requireFact(/repository has changed|저장소.*변경/.test(body),'stale_state_hidden');
        requireFact((await page.locator('main').textContent()).includes('state: Stale'),'stale_basis_missing');
        await capture(`stale-${locale}.png`);
      } finally {fs.writeFileSync(source,original);}
      await go(routes(locale).work);await workFacts('older',locale);
    });
    await check(`unavailable-analysis-${locale}`,async()=>{
      const saved=F.analysis_directory+'-browser-unavailable';fs.renameSync(F.analysis_directory,saved);
      try {
        await go(routes(locale).detail);
        const body=await page.locator('main').innerText();
        requireFact(/unavailable|unverifiable|이용 불가|사용 불가|검증할 수|없음/i.test(body),'unavailable_state_hidden');
        requireFact(await page.locator('g.diagram-node').count()===0,'unavailable_analysis_fabricates_nodes');
        await capture(`unavailable-${locale}.png`);
      } finally {fs.renameSync(saved,F.analysis_directory);}
      await go(routes(locale).work);await workFacts('older',locale);
    });
  }
  await page.setViewportSize({width:390,height:900});
  await check('negative-historical-summary',()=>copyMutation('historical',()=>document.querySelectorAll('#overview .work-summary [data-question="VerificationCoverage"]').forEach(e=>e.remove()),()=>overviewFacts('en'),routes('en').default));
  await check('negative-fake-flow',()=>copyMutation('flow',()=>{const edge=document.querySelector('g.diagram-edge').cloneNode(true);edge.dataset.relationId='invented-flow-edge';document.querySelector('svg').appendChild(edge);},graphIdentity,routes('en').detail));
  await check('negative-wrapping',()=>copyMutation('wrapping',()=>{document.querySelectorAll('style').forEach(e=>e.textContent=e.textContent.replaceAll('overflow-wrap:anywhere','overflow-wrap:normal').replaceAll('white-space:pre-wrap','white-space:pre'));document.querySelectorAll('details').forEach(e=>e.open=true);},overflow,routes('en').work));
  await check('negative-prefix',()=>copyMutation('prefix',()=>{document.querySelectorAll('g.diagram-node text').forEach(e=>e.textContent=e.textContent.slice(0,12)+'…');},labels,routes('en').detail));
  // Obtain another actual product Work body, retaining older identity for a semantic substitution.
  await go(url('work','en',{work:F.goals.same_title}));
  config.otherWork = await page.locator(workSelector('same_title')).innerHTML();
  await check('negative-cross-work',async()=>{
    await go(routes('en').work);await workFacts('older','en');
    await page.locator(workSelector('older')).evaluate((e,html)=>e.innerHTML=html,config.otherWork);
    const file=path.join(config.output,'live-mutation-substitution.html');fs.writeFileSync(file,await page.content());await go(pathToFileURL(file).href);
    let reason;try{await workFacts('older','en');}catch(e){reason=String(e);}
    requireFact(reason?.includes('cross_work_next_step_substitution'),'semantic_substitution_not_detected_for_expected_reason');
    await page.screenshot({path:path.join(config.output,'live-negative-substitution.png')});await go(routes('en').work);await workFacts('older','en');
    return {detected:true,reason,restored_positive:'passed',copy:file};
  });
}
async function offline() {
  // Server has exited and Runtime was renamed by the parent before this process.
  await check('runtime-unavailable',async()=>{requireFact(!fs.existsSync(F.runtime),'runtime_still_available');const probe=await context.newPage();let unavailable=false;try{await probe.goto(config.url,{timeout:3000});}catch{unavailable=true;}finally{await probe.close();}requireFact(unavailable,'listener_still_available');await context.setOffline(true);});
  for (const locale of ['en','ko']) for (const width of [390,768,1440]) for (const factor of [1,2]) {
    await check(`offline-${locale}-${width}-${factor}`,async()=>{
      await page.setViewportSize({width,height:900});await go(pathToFileURL(config.snapshots[locale]).href);await zoom(1);if(factor===2)await zoom(2);
      await snapshotSafety();await overflow();
      for(const id of ['works','code','decision-reading','overview']) {
        const nav=page.locator(`nav[aria-label="Viewer"] a[href="#${id}"]`);await activate(nav,false);
        requireFact(await page.evaluate(()=>location.hash)==='#'+id,'offline_fragment_navigation_failed');
      }
      await evidenceOpen();await snapshotSafety();
      await capture(`offline-${locale}-${width}-${factor}.png`,page.locator('main details[open] > summary').first());
      return overflow();
    });
  }
  for(const f of config.prefix_snapshots) await check(`history-prefix-${f.prefix}-${f.locale}`,async()=>{
    await go(pathToFileURL(f.snapshot).href);await zoom(1);
    const card=page.locator(idSelector(`work-${f.work}`)).locator(':scope > article.work-item');
    const locale=f.locale;
    const current=await ordinaryText(card.locator('.fact-states [data-question="WorkState"]'));
    const states=locale==='en'?['open','completed','paused','completed']:['열림','완료','일시 중지','완료'];
    requireFact(current.includes(states[f.prefix]),'prefix_state_mismatch');
    const verification=await ordinaryText(card.locator('.fact-states [data-question="VerificationState"]'));
    const labels=locale==='en'?['No verification record','passed','failed','not run']:['검증 기록 없음','통과','실패','실행하지 않음'];
    requireFact(verification.includes(labels[f.prefix]),'prefix_verification_mismatch');
    const result=card.locator('.result-evidence');
    if(f.prefix===0)requireFact(await result.count()===0,'goal_only_invents_result');
    else {
      await result.locator('summary').click();
      const quote=await result.innerText();
      const key=f.prefix===3?'later_change':'change';
      requireFact(quote.includes(f.checkpoints[key]),'prefix_result_basis_mismatch');
      requireFact(quote.includes(f.prefix===3?'Additional changes after the earlier successful check':'The implementation changed'),'prefix_result_content_mismatch');
      await result.locator('summary').click();
    }
    await snapshotSafety();
    return {prefix:f.prefix,visible_state:current,visible_verification:verification,result_role:'explicit_evidence_only; no host generation in this fixture'};
  });
  for(const locale of ['en','ko']) await check(`purpose-absent-${locale}`,async()=>{
    await go(pathToFileURL(config.absent_snapshots[locale]).href);await zoom(1);
    const purpose=page.locator('#overview h3').first();
    requireFact((await purpose.locator('xpath=following-sibling::*[1]').innerText()).includes(locale==='en'?'No source-grounded Project purpose is recorded':'source-grounded 프로젝트 목적이 기록되지 않았습니다'),'goal_promoted_to_purpose');
    requireFact((await page.locator('#works').innerText()).includes('Goal must not replace Project Purpose'),'absent_purpose_goal_unreadable');
    await snapshotSafety();await capture(`purpose-absent-${locale}.png`);
  });
  await check('negative-fragment',()=>copyMutation('fragment',()=>{const href=document.querySelector('nav[aria-label="Viewer"] a').getAttribute('href');document.getElementById(href.slice(1)).removeAttribute('id');},fragments,pathToFileURL(config.snapshots.en).href));
  await check('negative-live-link',()=>copyMutation('live_link',()=>{document.querySelector('nav[aria-label="Viewer"] a').setAttribute('href','http://127.0.0.1:3219/?view=tools');},snapshotSafety,pathToFileURL(config.snapshots.en).href));
}
async function explanationFacts(key,locale) {
  const surface=page.locator(workSelector(key));
  const answer=surface.locator(':scope > article.work-item > .work-explanation');
  requireFact(await answer.count()===1,'current_explanation_missing');
  const visible=await ordinaryText(answer);
  if(key==='checksum')requireFact(visible.includes('ea25e999409939a1e2dc03234f416cf0e8a28a69b8c210b4508edce1105af276'),'meaningful_checksum_removed');
  requireFact(!/audit (?:record|note)|Reviewer log|aabbccddeeff00112233445566778899|bbccee00112233445566778899aabbcc/i.test(visible),'audit_clutter_in_ordinary_answer');
  for(const [question,groups] of Object.entries(config.claim_terms[key][locale]))
    claimGroups(await ordinaryText(answer.locator(`p[data-question="${question}"]`)),groups,`required_claim_missing:${question}`);
  for(const pattern of config.forbidden_patterns?.[key]?.[locale]||[])
    requireFact(!new RegExp(pattern,'i').test(visible),`forbidden_claim:${pattern}`);
  const facts=surface.locator(':scope > article.work-item > .fact-states');
  const state=await ordinaryText(facts.locator('[data-question="WorkState"]'));
  requireFact(state.includes(locale==='en'?'completed':'완료'),'localized_work_state_missing');
  if(key==='queue')requireFact((await ordinaryText(facts.locator('[data-question="VerificationState"]'))).includes(locale==='en'?'failed':'실패'),'current_failure_hidden');
  if(key==='older') {
    requireFact((await ordinaryText(facts.locator('[data-question="VerificationState"]'))).includes(locale==='en'?'not run':'실행하지 않음'),'historical_pass_promoted');
    requireFact((await ordinaryText(facts.locator('[data-question="VerificationCoverage"]'))).includes(locale==='en'?'historical':'과거'),'historical_coverage_hidden');
  }
  const basis=answer.locator('.explanation-grounding');
  await basis.locator('summary').click();
  try {
    const text=await basis.innerText();
    const expected=config.basis[key][locale];
    requireFact(text.includes(expected.fingerprint),'old_basis_presented_as_current');
    for(const e of expected.evidence)requireFact(text.includes(e.identity)&&text.includes(`revision: ${e.revision}`)&&text.includes(e.field),'explanation_exact_basis_missing');
    requireFact(text.includes('self_reported_not_independently_verified'),'generator_identity_overclaimed');
  } finally {await basis.locator('summary').click();}
  return {ordinary_answer:visible};
}
async function semanticMutation(name,key,locale,mutate,expected) {
  const url=`${config.url}?view=work&work=${F.goals[key]}&locale=${locale}&language=${locale}`;
  await go(url);await explanationFacts(key,locale);
  await page.locator(workSelector(key)).evaluate(mutate);
  const file=path.join(config.output,`semantic-${name}-${locale}.html`);
  fs.writeFileSync(file,await page.content());await go(pathToFileURL(file).href);
  let reason;try{await explanationFacts(key,locale);}catch(e){reason=String(e);}
  requireFact(reason?.includes(expected),`negative_control_wrong_reason:${name}:${reason}`);
  await go(url);await explanationFacts(key,locale);
  return {mutation:name,detected:reason,restored_positive:'passed'};
}
async function workExplanations() {
  for (const locale of ['en','ko']) for (const key of Object.keys(config.claim_terms)) {
    await check(`work-explanation-${key}-${locale}`,async()=>{
      await go(`${config.url}?view=work&work=${F.goals[key]}&locale=${locale}&language=${locale}`);
      await explanationFacts(key,locale);
      const surface=page.locator(workSelector(key));
      const answer=surface.locator(':scope > article.work-item > .work-explanation');
      requireFact(await answer.count()===1,'current_explanation_missing');
      requireFact(await answer.getAttribute('data-statement-role')==='generated-interpretation','interpretation_presented_as_fact');
      const visible=await ordinaryText(answer);
      requireFact(!visible.includes('aabbccddeeff00112233445566778899'),'audit_clutter_in_ordinary_answer');
      const observed={};
      for (const [question,groups] of Object.entries(config.claim_terms[key][locale])) {
        const body=await ordinaryText(answer.locator(`p[data-question="${question}"]`));
        claimGroups(body,groups,`required_claim_missing:${key}/${locale}/${question}`);
        observed[question]=body;
      }
      for(const pattern of config.forbidden_patterns?.[key]?.[locale]||[])requireFact(!new RegExp(pattern,'i').test(visible),`forbidden_claim:${key}/${locale}/${pattern}`);
      requireFact(await answer.locator('details[open]').count()===0,'proof_depends_on_open_evidence');
      const grounding=answer.locator('.explanation-grounding');
      await grounding.locator('summary').click();
      requireFact((await grounding.innerText()).includes('self_reported_not_independently_verified'),'generator_identity_overclaimed');
      await grounding.locator('summary').click();
      await capture(`work-explanation-${key}-${locale}.png`,surface);
      await go(`${config.url}?view=overview&locale=${locale}&language=${locale}`);
      const card=page.locator(`.work-summary[data-work-id="${F.goals[key]}"]`).first();
      if(await card.count()) {
        for (const q of ['ReportedChange','Verification']) requireFact((await ordinaryText(card.locator(`p[data-question="${q}"]`))).includes(observed[q]),`overview_answer_diverged:${q}`);
        const expectedAction=config.expected_actions[key];
        const direction=await ordinaryText(card.locator('p[data-question="RecordedNextStep"]'));
        requireFact(direction.includes(expectedAction),`overview_recorded_action_diverged:${key}`);
        requireFact(await card.locator('details').count()===0,'overview_audit_expanded');
      } else requireFact((await page.locator('.category-count').allTextContents()).some(t=>{const n=t.match(/\d+/g)?.map(Number);return n?.[2]>0;}),'overview_missing_without_omission');
      await go(pathToFileURL(config.snapshots[locale]).href);
      const offline=page.locator(workSelector(key)).locator(':scope > article.work-item > .work-explanation');
      for (const [q,body] of Object.entries(observed)) requireFact(await ordinaryText(offline.locator(`p[data-question="${q}"]`))===body,`snapshot_answer_diverged:${q}`);
      requireFact(await page.locator('script,form,iframe,input,button').count()===0,'snapshot_has_active_transport');
      return {work:F.goals[key],ordinary_answers:observed,external_transmission:'none',human_acceptance:'not_established'};
    });
  }
  for(const locale of ['en','ko']) {
    const controls=[
      ['delete-result','relay',e=>e.querySelector('.work-explanation p[data-question="ReportedChange"]').textContent='','required_claim_missing'],
      ['hide-result','relay',e=>e.querySelector('.work-explanation p[data-question="ReportedChange"]').style.display='none','required_claim_missing'],
      ['other-work-result','relay',e=>e.querySelector('.work-explanation p[data-question="ReportedChange"]').textContent='A temporary file replaces the CSV report.','required_claim_missing'],
      ['hide-current-failure','queue',e=>e.querySelector('.fact-states p[data-question="VerificationState"]').hidden=true,'current_failure_hidden'],
      ['promote-historical-pass','older',e=>e.querySelector('.fact-states p[data-question="VerificationState"]').textContent='Automated verification: passed / 통과','historical_pass_promoted'],
      ['audit-primary','relay',e=>{const p=document.createElement('p');p.textContent='Audit record bbccee00112233445566778899aabbcc';e.querySelector('.work-explanation').prepend(p);},'audit_clutter_in_ordinary_answer'],
      ['audit-replaces-result','relay',e=>e.querySelector('.work-explanation p[data-question="ReportedChange"]').textContent='Reviewer log: audit record bbccee00112233445566778899aabbcc','audit_clutter_in_ordinary_answer'],
      ['remove-checksum','checksum',e=>e.querySelector('.work-explanation p[data-question="ReportedChange"]').textContent=e.querySelector('.work-explanation p[data-question="ReportedChange"]').textContent.replace('ea25e999409939a1e2dc03234f416cf0e8a28a69b8c210b4508edce1105af276',''),'meaningful_checksum_removed'],
      ['old-basis','relay',e=>{const p=e.querySelector('.explanation-grounding pre');p.textContent=p.textContent.replace(/fingerprint: "[^"]+"/,'fingerprint: "obsolete"');},'old_basis_presented_as_current'],
    ];
    if(locale==='ko')controls.push(['debug-enum','relay',e=>e.querySelector('.fact-states p[data-question="WorkState"]').textContent='Work: Completed','localized_work_state_missing']);
    for(const [name,key,mutate,expected] of controls)await check(`semantic-${name}-${locale}`,()=>semanticMutation(name,key,locale,mutate,expected));
  }
  for(const phase of config.lifecycle_snapshots||[])await check(`lifecycle-${phase.phase}`,async()=>{
    await go(pathToFileURL(phase.snapshot).href);
    const work=page.locator(idSelector(`work-${phase.work}`)).locator(':scope > article.work-item');
    if(phase.phase==='restart-current'||phase.phase==='regenerated-current') {
      requireFact((await ordinaryText(work.locator('.work-explanation p[data-question="ReportedChange"]')))===phase.reported_change,'lifecycle_current_answer_missing');
    } else {
      requireFact(await work.locator('.work-explanation').count()===0,'lifecycle_stale_or_forgotten_answer_visible');
      const body=await ordinaryText(work);
      if(phase.phase==='stale-correction')requireFact(body.includes('Explanation is stale'),'lifecycle_stale_state_hidden');
      if(phase.phase==='forgotten-result')requireFact(body.includes('No reported result is recorded'),'lifecycle_forgetting_result_gap_hidden');
    }
    const action=work.locator('.fact-states p[data-question="RecordedNextStep"]');
    if(phase.recorded_action!==null) {
      requireFact(await action.count()===1,'lifecycle_recorded_action_missing');
      requireFact(await ordinaryText(action)===`Recorded next action quotation (original language): ${phase.recorded_action}`,'lifecycle_recorded_action_changed');
      requireFact(await action.isVisible(),'lifecycle_recorded_action_hidden');
    } else {
      requireFact(await action.count()===0,'lifecycle_forgotten_action_revived');
      requireFact(await work.locator('.fact-states p[data-question="NextStepAvailability"]').count()===1,'lifecycle_missing_action_gap_hidden');
    }
    await snapshotSafety();return {phase:phase.phase,reading:'ordinary_before_disclosure'};
  });
  for (const locale of ['en','ko']) for (const key of Object.keys(config.decision_terms)) {
    await check(`decision-answer-${key}-${locale}`,async()=>{
      await go(`${config.url}?view=decisions&decision=${F.decisions[key]}&locale=${locale}&language=${locale}`);
      const answer=page.locator('.work-explanation').first();
      requireFact(await answer.count()===1,'current_decision_explanation_missing');
      requireFact(!(await answer.innerText()).includes('aabbccddeeff00112233445566778899'),'decision_audit_primary');
      const observed={};
      for (const [question,groups] of Object.entries(config.decision_terms[key][locale])) {
        const body=await ordinaryText(answer.locator(`p[data-question="${question}"]`));
        claimGroups(body,groups,`decision_claim_missing:${key}/${locale}/${question}`);
        observed[question]=body;
      }
      requireFact(await answer.locator('details[open]').count()===0,'decision_requires_evidence_disclosure');
      await capture(`decision-answer-${key}-${locale}.png`,answer);
      await go(pathToFileURL(config.snapshots[locale]).href);
      const offline=page.locator(idSelector(`decision-${F.decisions[key]}`)).locator(':scope > .work-explanation');
      for (const [q,body] of Object.entries(observed)) requireFact(await ordinaryText(offline.locator(`p[data-question="${q}"]`))===body,`decision_snapshot_answer_diverged:${q}`);
      return {decision:F.decisions[key],ordinary_answers:observed,human_acceptance:'not_established'};
    });
  }

}

// Labeled structural generation verifies displayed lifecycle identity, not prose quality.
async function contextLifecycle() {
  const {spawnSync}=require('node:child_process');
  let sequence=0;
  function operation(args) {
    const argv=[config.cli,'--runtime',F.runtime,'--project',F.project,'--json',...args];
    const start=process.hrtime.bigint();
    const completed=spawnSync(argv[0],argv.slice(1),{cwd:F.repository,encoding:'utf8',timeout:30000,maxBuffer:32<<20});
    const label=`context-operation-${++sequence}`;
    fs.writeFileSync(path.join(config.output,label+'.stdout'),completed.stdout||'');
    fs.writeFileSync(path.join(config.output,label+'.stderr'),completed.stderr||'');
    (result.operations??=[]).push({argv,exit_code:completed.status,signal:completed.signal,error:completed.error?.code||null,duration_ns:Number(process.hrtime.bigint()-start),stdout:label+'.stdout',stderr:label+'.stderr'});
    requireFact(completed.status===0&&!completed.error,'explanation_product_operation_failed');
    return JSON.parse(completed.stdout);
  }
  function record(kind,identity,locale) {
    const selector=kind==='work'?'--work':'--decision';
    const plan=operation([kind,'explain','prepare',selector,identity,'--language',locale]).plan;
    const questions=kind==='work'?['purpose','reported_change','expected_effect','verification','next_step']:['user_rationale','recommendation','consequences','applicability'];
    const response={format_kind:'volicord_explanation',format_version:1,plan_fingerprint:plan.fingerprint,language:locale,
      generator:{host:'display-context-fixture',session:'synthetic-structural-support',agent:null,model:null},
      paragraphs:questions.map(question=>({question,text:`Labeled structural fixture ${locale} ${question}`,evidence_keys:plan.evidence.map(e=>e.key)}))};
    const input=path.join(config.output,`response-${sequence}.json`);fs.writeFileSync(input,JSON.stringify(response));
    operation([kind,'explain','record',selector,identity,'--language',locale,'--input',input]);
    return plan;
  }
  const work=F.goals.relay,decision=F.decisions.project;
  const phases={};
  async function display(phase,kind,identity,locale,state) {
    const view=kind==='work'?'work':'decisions',selector=kind==='work'?'work':'decision';
    await go(`${config.url}?view=${view}&${selector}=${identity}&locale=${locale}&language=${locale}`);
    await zoom(1);
    const surface=page.locator(idSelector(`${kind}-${identity}`));
    const reading=surface.locator('.work-explanation,.answer-unavailable').first();
    requireFact(await reading.count()===1&&await reading.isVisible(),'actual_reading_surface_unavailable');
    if(state==='current')requireFact((await ordinaryText(reading)).includes(`Labeled structural fixture ${locale}`),'current_answer_not_displayed');
    await capture(`${phase}-${kind}-${locale}.png`,reading);
    const observed=result.display_captures.at(-1),answer=observed.context.explanations.find(e=>e.kind===kind&&e.identity===identity);
    requireFact(answer?.state===state,`displayed_explanation_state:${phase}:${locale}`);
    requireFact(observed.context.locale===locale&&observed.context.language===locale&&observed.context[`selected_${kind}`]===identity,'displayed_locale_subject_mismatch');
    phases[`${phase}-${kind}-${locale}`]=observed;
    fs.writeFileSync(path.join(config.output,`${phase}-${kind}-${locale}.json`),JSON.stringify(observed,null,2));
    return observed;
  }
  await check('actual_display_lifecycle',async()=>{
    for(const locale of ['en','ko'])await display('absent','work',work,locale,'unavailable');
    let plan;
    for(const locale of ['en','ko']){plan=record('work',work,locale);await display('current','work',work,locale,'current');}
    const goal=plan.evidence.find(e=>e.key==='goal');
    operation(['advanced','records','correct-context',work,'--revision',String(goal.revision),'--source',goal.sources[0],'--text',goal.content+'.']);
    for(const locale of ['en','ko'])await display('stale','work',work,locale,'stale');
    for(const locale of ['en','ko']){record('work',work,locale);await display('regenerated','work',work,locale,'current');}
    for(const locale of ['en','ko']){await display('absent','decision',decision,locale,'unavailable');record('decision',decision,locale);await display('current','decision',decision,locale,'current');}
    for(const locale of ['en','ko']) {
      const before=phases[`current-work-${locale}`],after=phases[`regenerated-work-${locale}`];
      requireFact(before.context.canonical_read_fingerprint!==after.context.canonical_read_fingerprint&&before.dom_sha256!==after.dom_sha256,'changed_basis_reused');
      const a=before.context.explanations.find(e=>e.identity===work),b=after.context.explanations.find(e=>e.identity===work);
      requireFact(a.plan_fingerprint!==b.plan_fingerprint&&a.generated_at_unix_micros!==b.generated_at_unix_micros&&a.realization_sha256!==b.realization_sha256,'generation_identity_reused');
    }
    return {states:['unavailable','current','stale','regenerated-current'],locales:['en','ko'],fixture_generation:'labeled structural only',human_judgment:'not_established'};
  });
  await check('native_zoom_and_existing_tab_attachment',async()=>{
    await go(`${config.url}?view=work&work=${work}&locale=en&language=en`);await zoom(1);await zoom(2);await capture('native-200-en.png',page.locator(workSelector('relay')).locator('.work-explanation').first());
    const render=await page.evaluate(()=>JSON.parse(document.querySelector('meta[name="volicord-observation"]').content).render_id);
    const argv=[config.capture_tool,'--binary',config.viewer,'--runtime',F.runtime,'--project',F.project,'--cdp-url',`http://127.0.0.1:${config.debug_port}`,'--page-url',page.url(),'--playwright-module',config.playwright,'--output',path.join(config.output,'attached-en')];
    const completed=spawnSync('python3',argv,{encoding:'utf8',timeout:40000,maxBuffer:32<<20,env:{...process.env,PYTHONDONTWRITEBYTECODE:'1'}});
    fs.writeFileSync(path.join(config.output,'attachment.stdout'),completed.stdout||'');fs.writeFileSync(path.join(config.output,'attachment.stderr'),completed.stderr||'');
    result.attachment_process={exit_code:completed.status,signal:completed.signal,error:completed.error?.code||null};
    requireFact(completed.status===0&&!completed.error,'existing_tab_capture_failed');
    const captured=JSON.parse(fs.readFileSync(path.join(config.output,'attached-en/display-context.json'),'utf8'));
    requireFact(captured.context.render_id===render,'attachment_changed_rendered_page');
    requireFact(!page.isClosed()&&await page.evaluate(()=>document.body.dataset.viewerMode)==='live','observer_closed_browser');
    const wrong=[...argv];wrong[wrong.indexOf('--binary')+1]=config.cli;wrong[wrong.indexOf('--output')+1]=path.join(config.output,'wrong-executable');
    const rejected=spawnSync('python3',wrong,{encoding:'utf8',timeout:40000,maxBuffer:32<<20,env:{...process.env,PYTHONDONTWRITEBYTECODE:'1'}});
    fs.writeFileSync(path.join(config.output,'wrong-executable.stdout'),rejected.stdout||'');fs.writeFileSync(path.join(config.output,'wrong-executable.stderr'),rejected.stderr||'');
    result.wrong_executable_process={exit_code:rejected.status,signal:rejected.signal,error:rejected.error?.code||null};
    requireFact(rejected.status===1&&!rejected.error&&!fs.existsSync(path.join(config.output,'wrong-executable/display-context.json')),'wrong_executable_observation_accepted');
    await page.keyboard.press('Home');await frames();
    await go(`${config.url}?view=work&work=${work}&locale=ko&language=ko`);await zoom(1);await zoom(2);await capture('native-200-ko.png',page.locator(workSelector('relay')).locator('.work-explanation').first());
    return {observer_detached_browser_still_usable:true,wrong_executable_rejected:true,zoom:2,locales:['en','ko'],attached_render_id:render};
  });
}

(async()=>{
  try {
    if(mode==='observe') {
      attached=await chromium.connectOverCDP(config.cdp_url,{timeout:10000});
      const matching=attached.contexts().flatMap(c=>c.pages()).filter(p=>p.url()===config.url);
      requireFact(matching.length===1,'displayed_viewer_tab_missing_or_ambiguous');
      page=matching[0];context=page.context();
      await check('current_display_context',async()=>{
        requireFact(await page.evaluate(()=>document.body.dataset.viewerMode==='live'),'live_viewer_required');
        await capture('display.png',page.locator('#no-scroll-target-for-observation'));
      });
      result.status=result.checks.some(c=>c.status!=='passed')?'failed':'passed';
      return;
    }
    try {
      context=await chromium.launchPersistentContext(path.join(config.output,`${mode}-profile`),{executablePath:config.chromium,headless:true,ignoreDefaultArgs:mode==='work-explanation'?[]:['--disable-extensions'],args:mode==='work-explanation'?[]:[`--disable-extensions-except=${config.extension}`,`--load-extension=${config.extension}`,...(config.debug_port?[`--remote-debugging-port=${config.debug_port}`]:[])],viewport:{width:390,height:900}});
      if(mode!=='work-explanation')worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker',{timeout:15000});
    } catch(error) {result.status='browser_launch_blocked';result.detail=String(error);return;}
    result.browser_version=context.browser().version();
    page=context.pages()[0]||await context.newPage();page.setDefaultTimeout(5000);
    context.on('request',request=>{const address=request.url();if(address.startsWith('http'))result.requests.push({url:address,method:request.method(),resource:request.resourceType()});});
    await context.route('**/*',async route=>{
      const request=route.request();const address=request.url();
      if(address.startsWith('http')&&(!address.startsWith(config.url)||request.method()!=='GET')) {
        // Offline deliberately probes only the dead local listener once.
        result.checks.push({id:'unexpected_external_or_mutation_request',status:'failed',reason:address});await route.abort();
      } else await route.continue();
    });
    if(mode==='live')await live();else if(mode==='offline')await offline();else if(mode==='work-explanation')await workExplanations();else if(mode==='context-lifecycle')await contextLifecycle();else throw new Error('Unknown driver mode');
    requireFact(result.checks.length>0,'no_browser_execution');
    result.status=result.checks.some(c=>c.status!=='passed')?'failed':'passed';
  } catch(error) {result.status='failed';result.detail=String(error);}
  finally {
    if(attached)await attached.close();else if(context)await context.close();
    fs.writeFileSync(path.join(config.output,`${mode}-result.json`),JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify({status:result.status,mode,checks:result.checks.length,failures:result.checks.filter(c=>c.status!=='passed'),detail:result.detail}));
    if(result.status!=='passed')process.exitCode=1;
  }
})();
