// Coupled V11 supporting driver. Automation evaluation never becomes Product JS.
'use strict';
const fs = require('node:fs');
const path = require('node:path');
const {pathToFileURL} = require('node:url');
const config = JSON.parse(fs.readFileSync(process.argv[2], 'utf8'));
const mode = process.argv[3];
if (fs.existsSync(path.join(config.output,`${mode}-result.json`))) throw new Error('Browser results are create-only; use a fresh output directory');
const {chromium} = require(config.playwright);
const result = {kind: 'viewer_browser_observations', mode, status: 'not_run', checks: [], requests: [], timings: [], zoom: [], human_acceptance: 'not_established'};
let context, page, worker;
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
  await frames();
  const geometry=await page.evaluate(()=>({scrollX,scrollY,innerWidth,innerHeight,dpr:devicePixelRatio}));
  const session=await context.newCDPSession(page);
  try {
    const image=await session.send('Page.captureScreenshot',{format:'png',fromSurface:false});
    fs.writeFileSync(path.join(config.output,name),Buffer.from(image.data,'base64'));
  } finally {await session.detach();}
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
  const before = await page.evaluate(() => ({innerWidth, dpr:devicePixelRatio, visualScale:visualViewport.scale}));
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
async function workFacts(key, locale) {
  const section = page.locator(workSelector(key));
  requireFact(await section.count() === 1, 'work_exact_identity');
  const text = await section.innerText();
  const expected = config.scenario.works.find(w => w.key === key);
  requireFact(text.includes(expected.title), 'work_title_source_mismatch');
  const latest = expected.checkpoints.at(-1);
  if (latest) {
    requireFact(text.includes(latest.next_step), 'cross_work_next_step_substitution');
    const state = await section.locator('.fact-states').first().innerText();
    for (const fact of [latest.work, latest.verification, latest.review, latest.acceptance]) requireFact(state.includes(fact), `work_state_source_mismatch:${fact}`);
    for (const keyName of config.expected.exact_older.excluded_decision_keys) if (key === 'older') requireFact(!await section.locator(`a[href*="decision=${F.decisions[keyName]}"]`).count(), 'cross_work_decision_substitution');
    if (key === 'older') {
      requireFact(text.includes('Failed') && text.includes('Rejected') && text.includes('Pending'), 'historical_failure_or_pending_state_hidden');
      requireFact(text.includes(locale === 'en' ? 'Semantic summary unavailable' : '의미 요약 없음'), 'summary_limit_hidden');
      const source = await section.textContent();
      for (const cp of config.expected.exact_older.checkpoint_keys) requireFact(source.includes(F.checkpoints[cp]), 'checkpoint_basis_missing');
      for (const decision of ['explicit','project','unresolved']) requireFact(await section.locator(`a[href*="decision=${F.decisions[decision]}"]`).count() > 0, 'decision_scope_link_missing');
    }
  } else requireFact(text.includes(locale === 'en' ? 'Goal only' : '목표만'), 'goal_only_gap_hidden');
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
  const expected = {wrapping:'ordinary_page_overflow',prefix:'common_prefix_truncation',fragment:'fragment_target',substitution:'cross_work_next_step_substitution',live_link:'snapshot_live_link'}[name];
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
  const routes = locale => ({default:config.url+'?locale='+locale,work:url('work',locale,{work:F.goals.older}),decision:url('decisions',locale,{decision:F.decisions.explicit}),detail:url('code',locale,{scope:'repository',entity:entity.id})});
  for (const locale of ['en','ko']) {
    for (const width of [390,768,1440]) {
      await page.setViewportSize({width,height:900});
      for (const factor of [1,2]) {
        for (const [state,address] of Object.entries(routes(locale))) {
          const reading=page.locator(state==='default'?'#overview':state==='work'?workSelector('older'):state==='decision'?idSelector(`decision-${F.decisions.explicit}`):'#code');
          await check(`${locale}-${width}-${factor}-${state}`, async()=>{
            await go(address);
            await zoom(1);
            if (factor===2) await zoom(2);
            requireFact(await page.locator('html').getAttribute('lang') === locale, 'bundled_locale_mismatch');
            await fragments();
            requireFact(await page.locator('script,[onclick],[onerror]').count()===0,'escaping_security_boundary');
            if (state==='default') requireFact((await page.locator('#overview').innerText()).includes(config.scenario.purpose_present), 'purpose_not_source_grounded');
            if (state==='work') await workFacts('older',locale);
            if (state==='decision') {
              const section = page.locator(idSelector(`decision-${F.decisions.explicit}`));
              const body = await section.innerText();
              requireFact(body.includes(config.scenario.decisions[0].recommendation_rationale), 'recommendation_rationale_source_mismatch');
              requireFact(body.includes(locale==='en' ? 'Summary unavailable: no explanatory text is recorded' : '설명 문장이 기록되지 않아 요약을 제공할 수 없습니다'), 'missing_user_rationale_fabricated');
              const stateText=await section.locator('.state').first().innerText();
              requireFact(stateText.includes(locale==='en' ? 'review required' : '검토 필요'), 'review_due_hidden');
              requireFact(stateText.includes('[local]') && body.includes('[remote]'),'user_choice_and_recommendation_confused');
              requireFact(await section.locator('xpath=..').getAttribute('data-decision-scope')==='work-item' && body.includes(locale==='en'?'one Work Item':'특정 작업 항목') && body.includes('native/query.c'),'decision_scope_source_mismatch');
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
      return evidence;
    });
    await check(`keyboard-paged-old-work-${locale}`, async()=>{
      await go(url('work',locale));
      requireFact(!await page.locator(`a[href*="work=${F.goals.older}"]`).count(), 'old_work_not_beyond_list_bound');
      await activate(page.locator('nav a[href*="page=1"]'));
      await activate(page.locator(`a[href*="work=${F.goals.older}"]`).first());
      await workFacts('older',locale);
      await activate(page.locator(`a[href*="scope=work"][href*="work=${F.goals.older}"]`));
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
        requireFact(body.includes('the repository has changed since this analysis; refresh analysis before using current code facts'),'stale_state_hidden');
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
(async()=>{
  try {
    try {
      context=await chromium.launchPersistentContext(path.join(config.output,`${mode}-profile`),{executablePath:config.chromium,headless:true,ignoreDefaultArgs:['--disable-extensions'],args:[`--disable-extensions-except=${config.extension}`,`--load-extension=${config.extension}`],viewport:{width:390,height:900}});
      worker=context.serviceWorkers()[0]||await context.waitForEvent('serviceworker',{timeout:15000});
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
    if(mode==='live')await live();else if(mode==='offline')await offline();else throw new Error('Unknown driver mode');
    requireFact(result.checks.length>0,'no_browser_execution');
    result.status=result.checks.some(c=>c.status!=='passed')?'failed':'passed';
  } catch(error) {result.status='failed';result.detail=String(error);}
  finally {
    if(context)await context.close();
    fs.writeFileSync(path.join(config.output,`${mode}-result.json`),JSON.stringify(result,null,2)+'\n');
    console.log(JSON.stringify({status:result.status,mode,checks:result.checks.length,failures:result.checks.filter(c=>c.status!=='passed'),detail:result.detail}));
    if(result.status!=='passed')process.exitCode=1;
  }
})();
