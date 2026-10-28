// Negative controls use an actual public export, in separate disposable pages.
// These mutations are counterexamples, never authored repository facts.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {observeOverview,installEntryObserver}=require('./overview_entry.cjs');
const {inspectDiagram}=require('./diagram_geometry.cjs');
const C=JSON.parse(fs.readFileSync(process.argv[2]));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
(async()=>{
 fs.mkdirSync(C.output,{recursive:false});
 const browser=await require(C.playwright).chromium.launch({executablePath:C.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:C.fontconfig}});
 const rows=[];
 try{
 for(const c of C.cases){const pub=JSON.parse(fs.readFileSync(c.public));const d=pub.diagrams.find(d=>d.id===c.diagram);
  for(const control of ['hidden-card','miniature','assisted-entry','incomplete-label','reversed-edge','merged-routes','invisible-focus']){
   // The export itself is script-free. Enable evaluation callbacks for the
   // observer's event/MutationObserver negative control; no-JS product journeys
   // run separately through grounded_diagrams_browser.cjs.
   const context=await browser.newContext({viewport:{width:320,height:900},javaScriptEnabled:true});const page=await context.newPage();
   await context.route('**/*',r=>/^(file|data):/.test(r.request().url())?r.continue():r.abort());
   await installEntryObserver(page);
   await page.goto('file://'+path.resolve(c.offline));await page.evaluate(()=>document.fonts.ready);await page.evaluate(h=>location.hash=h,c.work?'work-'+c.work:'repository-explanation');
   const figure=page.locator('.admitted-diagram[data-semantic-fingerprint="'+d.semantic_fingerprint+'"]');
   await figure.evaluate(n=>n.scrollIntoView({block:'start'}));
   await page.screenshot({path:path.join(C.output,c.key+'-'+control+'-initial.png')});
   await installEntryObserver(page);assert.deepEqual((await observeOverview(figure,d)).errors,[],'unmodified actual entry');
   if(['hidden-card','assisted-entry'].includes(control)){
    await figure.evaluate((n,showCard)=>{const viewport=n.querySelector('.diagram-viewport');const details=document.createElement('details');details.className='counterexample-hidden-map';const summary=document.createElement('summary');summary.textContent='Full map';viewport.replaceWith(details);details.append(summary,viewport);if(showCard)n.querySelector('.diagram-claim:has(.diagram-connection)').dataset.active='true';},control==='hidden-card');
    if(control==='assisted-entry'){
     await figure.locator('.counterexample-hidden-map > summary').click();
     await installEntryObserver(page); // Cannot reset evidence of assisted entry.
    }
    const observed=await observeOverview(figure,d);
    fs.writeFileSync(path.join(C.output,c.key+'-'+control+'-observation.json'),JSON.stringify(observed,null,2),{flag:'wx'});
    assert(observed.errors.includes(control==='hidden-card'?'connected map hidden or absent':'orientation was assisted before its initial observation'));
    rows.push({case:c.key,control,observed});
   }else if(control==='miniature'){
    await figure.locator('svg').evaluate(n=>{n.style.transform='scale(.15)';n.style.transformOrigin='0 0';});
    const observed=await observeOverview(figure,d);assert(observed.errors.includes('essential text scaled below native reading size'));rows.push({case:c.key,control,observed});
   }else if(control==='invisible-focus'){
    const {inspectTextOriginFocus}=require('./diagram_focus.cjs');
    const before=await inspectTextOriginFocus(figure);
    assert.deepEqual(before.errors,[],'unmodified focus');
    assert(before.observations.length,'select a real figure with a guard or state owner');
    await page.addStyleTag({content:'.connected-overview .diagram-element:focus text {text-decoration:none!important}'});
    const observed=await inspectTextOriginFocus(figure);
    assert(observed.errors.length,'a missing focus cue must fail after correct orientation');
    rows.push({case:c.key,control,observed});
   }else{
    const edges=d.meaning.relations||d.meaning.groups.flatMap(g=>g.messages);const edge=edges.find(e=>e.from!==e.to);
    const anchor=figure.locator('a[id="'+await figure.getAttribute('id')+'-'+edge.id+'"]');
    if(control==='incomplete-label'){
     const label=edge.label.text;
     await anchor.evaluate((n,label)=>{const t=[...n.querySelectorAll('text')].find(t=>t.textContent===label&&getComputedStyle(t.parentElement).display!=='none');t.replaceChildren(document.createTextNode('DOM snapshot을'));},label);
    }else if(control==='reversed-edge'){
     await anchor.evaluate(n=>{const p=[...n.querySelectorAll('.relation-line')].find(p=>getComputedStyle(p.parentElement).display!=='none');const a=p.getPointAtLength(0),b=p.getPointAtLength(p.getTotalLength());p.setAttribute('transform',`rotate(180 ${(a.x+b.x)/2} ${(a.y+b.y)/2})`);});
    }else{
     const other=edges.find(e=>e.from!==edge.from&&e.to!==edge.to);assert(other,'distinct routes challenge merger');
     const otherAnchor=figure.locator('a[id="'+await figure.getAttribute('id')+'-'+other.id+'"]');
     const route=await anchor.evaluate(n=>[...n.querySelectorAll('.relation-line')].find(p=>getComputedStyle(p.parentElement).display!=='none').getAttribute('d'));
     await otherAnchor.evaluate((n,d)=>[...n.querySelectorAll('.relation-line')].find(p=>getComputedStyle(p.parentElement).display!=='none').setAttribute('d',d),route);
    }
    const geometry=await inspectDiagram(figure.locator('svg'),d,pub.prose);
    const expected=control==='incomplete-label'?'missing-visible-meaning':control==='reversed-edge'?(d.meaning.kind==='sequence'?'wrong-lifeline-direction':'wrong-edge-direction'):'route-merge';
    assert(geometry.errors.some(e=>e.kind===expected),control+' must fail independently');
    rows.push({case:c.key,control,errors:geometry.errors});
   }
   await figure.evaluate(n=>n.scrollIntoView({block:'start'}));await page.screenshot({path:path.join(C.output,c.key+'-'+control+'-counterexample.png')});
   await context.close();
  }
 }
 fs.writeFileSync(path.join(C.output,'result.json'),JSON.stringify({rows,viewer:sha(C.viewer),browser:sha(C.chromium),status:'passed',scope:'negative technical controls; no comprehension or human acceptance'},null,2),{flag:'wx'});
 }finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1});
