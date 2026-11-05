// Compare real consumers with the immutable user-selected package, not with a
// renderer-generated oracle. Content authority and Source lifecycle are checked
// separately by the retained-publication journey drivers.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const crypto=require('node:crypto'),{spawn}=require('node:child_process'),{pathToFileURL}=require('node:url');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
const reference=path.resolve(config.reference),out=path.resolve(config.output);
assert.equal(sha(reference),'295961bcee3df01f9a7c7bfab126b33ce2ad00cc6b9fcdebc34b088e8d794046','selected CE1 bytes');
fs.mkdirSync(out,{recursive:false});
const result={outcome:'failed',rows:[],failures:[],identities:{reference:sha(reference),viewer:sha(config.viewer),native:sha(config.offline),driver:sha(__filename)},unobserved:['independent first reading','human acceptance','assistive technology','physical touch','other browser engines']};
const write=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2),{flag:'wx'});
async function composition(page){assert.equal(await page.locator('.doc[data-work-layout="grouped"]').count(),1,'selected Work reading groups required');return page.locator('.doc[data-work-layout="grouped"]').evaluate(doc=>{
 const style=n=>{if(!n)return null;const s=getComputedStyle(n);return Object.fromEntries(['fontSize','fontWeight','lineHeight','color','backgroundColor','borderTopWidth','paddingTop','paddingBottom','paddingLeft','paddingRight','marginTop','marginBottom','minHeight'].map(k=>[k,s[k]]));};
 const group=k=>doc.querySelector('[data-reading-group="'+k+'"]');
 const summary=n=>n.querySelector(':scope > summary')?.textContent.trim();
 const roots=[...group('records').children].filter(n=>n.matches('details'));
 const categories=[...roots[1].querySelectorAll('details')].filter(n=>n.parentElement.closest('details')===roots[1]);
 const toc=doc.querySelector('.document-nav');
 return {groups:[...doc.querySelectorAll(':scope > .work-reading-group')].map(n=>n.dataset.readingGroup),records:roots.map(summary),categories:categories.map(summary),closed:roots.every(n=>!n.open)&&categories.every(n=>!n.open),toc:{compact:toc.dataset.overflow==='true',links:[...toc.querySelectorAll('nav a')].map(n=>n.textContent.replace(/\s+/g,' ').trim())},checks:[...group('checks').children].map(n=>({heading:n.querySelector(':scope > h2')?.textContent,style:style(n.querySelector(':scope > h2'))})),styles:{title:style(doc.querySelector('h1')),lead:style(doc.querySelector('.lead')),strip:style(doc.querySelector('.record-strip')),badge:style(doc.querySelector('.work-state-badge')),heading:style(doc.querySelector('.article-section h2')),record:style(roots[0].querySelector('summary')),category:style(categories[0]?.querySelector('summary')),action:style(doc.querySelector('.paragraph-evidence-action'))}};
 });}
function differences(expected,actual,prefix=''){
 const errors=[];
 if(expected&&typeof expected==='object'&&!Array.isArray(expected)){
  for(const [k,v]of Object.entries(expected))errors.push(...differences(v,actual?.[k],prefix?prefix+'.'+k:k));
 }else if(JSON.stringify(expected)!==JSON.stringify(actual))errors.push({field:prefix,expected,actual});
 return errors;
}
async function pane(page){return page.locator('.appendix:visible').evaluate(n=>{
 const visible=s=>[...n.querySelectorAll(s)].filter(c=>c.checkVisibility());
 const code=visible(':scope > .cited-code')[0],metadata=visible(':scope > .source-metadata')[0],bar=visible('.reading-panel-header')[0];
 const rect=e=>e?.getBoundingClientRect();
 const pre=code?.querySelector('pre'),numbers=pre?.querySelector('.source-line-numbers>span'),text=pre?.querySelector('code');
 return {oneCode:visible(':scope > .cited-code').length===1,expanded:code?.open===true,metadataFolded:metadata?.open===false,codeBeforeMetadata:!!code&&!!metadata&&!!(code.compareDocumentPosition(metadata)&Node.DOCUMENT_POSITION_FOLLOWING)&&rect(code).bottom<=rect(metadata).top+1,headerHeight:Math.round(rect(bar)?.height),codeFont:text&&getComputedStyle(text).fontSize,lineHeight:text&&getComputedStyle(text).lineHeight,numberHeight:numbers&&getComputedStyle(numbers).height,chooser:visible('select').map(s=>s.options.length),width:Math.round(rect(n).width),claim:visible('.evidence-claim')[0]?.textContent,code:text?.textContent};
 });}
async function settled(page){await page.evaluate(()=>document.fonts.ready);await page.waitForTimeout(150);}
async function main(){
 let browser,child,logs='';
 try{
 child=spawn(config.viewer,['--runtime',config.runtime,'--project',config.project,'--bind','127.0.0.1:0']);child.stderr.on('data',b=>logs+=b);
 const address=await new Promise((ok,no)=>{const timer=setTimeout(()=>no(Error(logs||'listener timeout')),15000);child.once('error',no);child.stderr.on('data',()=>{const m=logs.match(/http:\/\/127\.0\.0\.1:\d+\//);if(m){clearTimeout(timer);ok(m[0]);}});});
 browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 for(const width of config.widths||[1440,1200,1024,768,390,320]){
  let expected;
  for(const mode of ['reference',...(config.modes||['live','live-no-js','native'])]){
   const scripted=['reference','live'].includes(mode),context=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:scripted}),page=await context.newPage();page.setDefaultTimeout(12000);
   const errors=[];page.on('pageerror',e=>errors.push(e.message));
   await context.route('**/*',r=>{const u=r.request().url();return u.startsWith(address)||u.startsWith('file:')||u.startsWith('data:')?r.continue():r.abort();});
   const url=mode==='reference'?pathToFileURL(reference).href+'#work-'+config.work:mode==='native'?pathToFileURL(path.resolve(config.offline)).href+'#work-'+config.work:address+'?view=work&work='+config.work+'&locale=ko&language=ko';
   try{
    await page.goto(url);await settled(page);
    const initial=await composition(page);write(mode+'-'+width+'-composition.json',initial);
    await page.screenshot({path:path.join(out,mode+'-'+width+'-initial.png')});
    if(mode==='reference')expected={initial};
    else{
     const diff=differences(expected.initial,initial);result.failures.push(...diff.map(e=>({mode,width,...e})));
     assert.equal(await page.locator('script').count()>0,['live','live-no-js'].includes(mode),'exports omit scripts; no-JS live disables their execution');
    }
    const records=page.locator('.doc [data-reading-group=records] > details').nth(1);
    await records.locator(':scope > summary').click();await records.scrollIntoViewIfNeeded();await settled(page);
    await page.screenshot({path:path.join(out,mode+'-'+width+'-records.png')});
    const raw=await records.locator('pre').evaluateAll(ns=>ns.every(n=>n.closest('details')!==document.querySelector('.doc [data-reading-group=records] > details:nth-of-type(2)')&&!n.closest('details').open));
    assert(raw,'original record data remains individually folded');await records.locator(':scope > summary').click();
    const figure=page.locator('.doc[data-work-layout="grouped"] .admitted-diagram').first();
    await figure.evaluate(n=>n.scrollIntoView({block:'start'}));await settled(page);
    await page.screenshot({path:path.join(out,mode+'-'+width+'-figure.png')});
    const geometry=await figure.locator('.diagram-viewport svg').evaluate(svg=>({participants:[...svg.querySelectorAll('.world-node,.participant-box')].filter(n=>n.getBoundingClientRect().height>0).map(n=>({width:n.getBBox().width,center:n.getBBox().x+n.getBBox().width/2})),cards:svg.querySelectorAll('.message-label').length,labels:[...svg.querySelectorAll('.world-action,.sequence-message>text')].map(n=>({alignment:n.getAttribute('text-anchor'),text:n.textContent})),world:svg.viewBox.baseVal.width||svg.width.baseVal.value}));
    assert.equal(geometry.cards,0,'selected temporal labels have no white card bands');
    assert.ok(geometry.participants.length>=5&&geometry.participants.every(n=>n.width>=280&&n.width<=360),'readable compact participant cards');
    assert.ok(geometry.participants.slice(1).every((n,i)=>n.center-geometry.participants[i].center>360),'space between participant lanes');
    assert.ok(geometry.labels.length>0&&geometry.labels.every(n=>n.alignment==='start'),'actions sit beside their directed route');
    const action=page.locator('.doc .paragraph-evidence-action:visible').first();await action.click();await settled(page);
    if(mode==='live'){const {citationPoint,citationReady}=require('./product_review_controls.cjs');await citationReady(await citationPoint(page,await action.getAttribute('href')));}
    const selected=await pane(page);write(mode+'-'+width+'-pane.json',selected);
    await page.screenshot({path:path.join(out,mode+'-'+width+'-code.png')});
    assert.ok(selected.oneCode&&selected.expanded&&selected.metadataFolded&&selected.codeBeforeMetadata,'one expanded complete code window precedes folded metadata');
    assert.equal(selected.headerHeight,56,'shared panel header');
    assert.ok(Math.abs(parseFloat(selected.lineHeight)-parseFloat(selected.numberHeight))<.1,'line numbers remain aligned throughout code');
    if(mode==='reference')expected.pane=selected;
    else for(const key of ['headerHeight','codeFont','lineHeight','numberHeight','width','claim','code'])if(selected[key]!==expected.pane[key])result.failures.push({mode,width,field:'pane.'+key,expected:expected.pane[key],actual:selected[key]});
    if(scripted)assert.deepEqual(selected.chooser,[1],'one direct exact-range chooser even for a single range');
    else assert.equal(await page.locator('.appendix select').count(),0,'native uses actual fragment/disclosure links');
    assert.deepEqual(errors,[],'no browser errors');result.rows.push({mode,width,geometry,sourceBytes:Buffer.byteLength(selected.code||''),scripted});
   }catch(error){result.failures.push({mode,width,error:error.stack});await page.screenshot({path:path.join(out,mode+'-'+width+'-failure.png')}).catch(()=>{});if(mode==='reference')throw error;}
   finally{await context.close();}
  }
 }
 assert.ok(result.rows.length>0);assert.equal(result.failures.length,0,'full CE1 composition/pane differences are preserved in result.json');result.outcome='passed';
 }finally{write('result.json',result);fs.writeFileSync(path.join(out,'viewer.stderr'),logs,{flag:'wx'});await browser?.close();child?.kill('SIGTERM');}
 console.log(JSON.stringify({outcome:result.outcome,conditions:result.rows.length,failures:result.failures.length}));
}
main().catch(e=>{console.error(e);process.exitCode=1});
