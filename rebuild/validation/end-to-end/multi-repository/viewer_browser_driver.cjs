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
async function capture(name, target=page.locator('main > section').first(), stress=false) {
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
  if(mode!=='offline' && !stress)(result.display_captures ??= []).push(receipt);
  (result.captures ??= []).push({path:name,mechanism:'Page.captureScreenshot fromSurface=false; browser viewport',...(stress?{evidence_class:'dom_stress; not a renderer display receipt'}:{}),...geometry});
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
  const expected = {wrapping:'ordinary_page_overflow',prefix:'common_prefix_truncation',fragment:'fragment_target',substitution:'cross_work_next_step_substitution',live_link:'snapshot_live_link',historical:'overview_promotes_historical_pass',flow:'graph_relation_basis_mismatch',work_history:'work_history_disclosure_missing',code_expanded:'code_diagnostics_initially_expanded',code_count:'code_diagnostic_counts_mismatch',code_warning:'material_source_blocker_hidden'}[name];
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
async function readerBoundary() {
  const nav=page.locator('nav[aria-label="Viewer"]');
  requireFact(await nav.count()===1&&await nav.locator('a').count()===3,'three_destinations_required');
  requireFact(await page.locator('.tools-menu,.inspection-query,#code,#repository-code,#decision-reading,#analysis-tool,#documents-tool,#memory-tool,#evidence-tool').count()===0,'retired_surface_present');
  const names=await nav.locator('a').allTextContents();
  requireFact(JSON.stringify(names)===JSON.stringify(await page.evaluate(()=>document.documentElement.lang==='ko')?['개요','저장소 설명','작업']:['Overview','Repository explanation','Work']),'primary_destinations_changed');
  requireFact(await page.locator('form').count()===0,'reader_mutation_form');
  await overflow();await fragments();
}
async function live() {
  for(const locale of ['en','ko'])for(const width of [1440,768,390,320])await check(`reader-${locale}-${width}`,async()=>{
    await page.setViewportSize({width,height:900});
    for(const view of ['overview','repository','work']) {
      await go(`${config.url}?view=${view}&locale=${locale}&language=${locale}`);
      await readerBoundary();await capture(`reader-${view}-${locale}-${width}.png`);
    }
    await go(`${config.url}?view=work&work=${F.goals.older}&locale=${locale}&language=${locale}`);
    const work=page.locator(workSelector('older'));
    requireFact(await work.count()===1,'exact_work_missing');
    requireFact(await work.locator(`[data-decision-id="${F.decisions.explicit}"]`).count()===1,'related_decision_missing');
    requireFact(await work.locator(`[data-decision-id="${F.decisions.other_work}"]`).count()===0,'foreign_work_decision');
    requireFact((await work.textContent()).includes('recommendation_rationale'),'decision_reason_missing');
    await readerBoundary();await capture(`reader-detail-${locale}-${width}.png`,work);
  });
  await check('retired_urls_and_authority',async()=>{
    for(const target of ['?view=tools','?view=decisions','?view=code&scope=repository','inspection','inspection.json'])requireFact((await context.request.get(config.url+target)).status()===404,'retired_route_not_denied');
    requireFact((await context.request.get(config.url,{headers:{Host:'foreign.invalid'}})).status()===421,'wrong_host_admitted');
    requireFact((await context.request.get(config.url,{headers:{Origin:'https://foreign.invalid'}})).status()===403,'wrong_origin_admitted');
    for(const target of ['?view=work&work=bad','?view=overview&work=bad'])requireFact((await context.request.get(config.url+target)).status()===400,'invalid_selector_fallback');
  });
  for(const locale of ['en','ko'])await check(`reader-native-zoom-${locale}`,async()=>{
    await page.setViewportSize({width:1440,height:900});await go(`${config.url}?view=repository&locale=${locale}`);await zoom(1);await zoom(2);await readerBoundary();await capture(`reader-native-zoom-${locale}.png`);await zoom(1);
  });
}
async function offline() {
  await context.setOffline(true);
  for(const locale of ['en','ko'])for(const width of [1440,768,390,320])await check(`offline-reader-${locale}-${width}`,async()=>{
    await page.setViewportSize({width,height:900});await go(pathToFileURL(config.snapshots[locale]).href);await snapshotSafety();await readerBoundary();
    for(const id of ['overview','repository-explanation','works']) {
      await activate(page.locator(`nav[aria-label="Viewer"] a[href="#${id}"]`),false);
      requireFact(await page.locator('#'+id).isVisible(),'offline_destination_hidden');
      await capture(`offline-reader-${id}-${locale}-${width}.png`,page.locator('#'+id));
    }
    const link=page.locator(`#works a[href="#work-${F.goals.older}"]`);
    if(await link.count()){await link.click();requireFact(await page.locator(workSelector('older')).isVisible(),'offline_work_hidden');}
    await snapshotSafety();await readerBoundary();
  });
  await check('negative-fragment',()=>copyMutation('fragment',()=>{const href=document.querySelector('nav[aria-label="Viewer"] a').getAttribute('href');document.getElementById(href.slice(1)).removeAttribute('id');},fragments,pathToFileURL(config.snapshots.en).href));
  await check('negative-live-link',()=>copyMutation('live_link',()=>{document.querySelector('nav[aria-label="Viewer"] a').setAttribute('href','http://127.0.0.1:3219/');},snapshotSafety,pathToFileURL(config.snapshots.en).href));
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
      requireFact(key==='explicit','unassociated_decision_requires_non_viewer_consumer');
      await go(`${config.url}?view=work&work=${F.goals.older}&locale=${locale}&language=${locale}`);
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
      const offline=page.locator(idSelector(`work-${F.goals.older}-decision-${F.decisions[key]}`)).locator(':scope > .work-explanation');
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
  const work=F.goals.relay;
  const phases={};
  async function display(phase,kind,identity,locale,state) {
    requireFact(kind==='work','only_contextual_work_display');
    const view='work',selector='work';
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
