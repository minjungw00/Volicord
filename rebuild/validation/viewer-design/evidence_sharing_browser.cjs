// Fixed-publication bytes, all exact bindings, and actual script-free navigation.
// Runtime Sources/public records are independent oracles; no quality verdict.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {pathToFileURL}=require('node:url');
const {fragmentErrors}=require('../end-to-end/multi-repository/reader_fragments.cjs');
const {nativeEvidenceContext,openEmbeddedCode,citationPoint}=require('./product_review_controls.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2])),out=path.resolve(config.output);
fs.mkdirSync(out,{recursive:false});
const hash=b=>'sha256:'+crypto.createHash('sha256').update(b).digest('hex');
const write=(name,value)=>fs.writeFileSync(path.join(out,name),JSON.stringify(value,null,2),{flag:'wx'});
const result={outcome:'failed',identities:{driver:hash(fs.readFileSync(__filename)),config:hash(fs.readFileSync(process.argv[2])),inputs:config.cases.map(c=>({key:c.key,html:hash(fs.readFileSync(c.offline)),public:hash(fs.readFileSync(c.public))}))},rows:[],unobserved:['human comprehension','assistive technology','model authorship cost','other browser engines']};
async function main(){
 const browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
 try{for(const c of config.cases){
  const pub=JSON.parse(fs.readFileSync(c.public)),publicId=pub.provenance.publication_id;
  for(const width of config.widths||[1440,390]){
   const context=await browser.newContext({viewport:{width,height:1000},javaScriptEnabled:false}),page=await context.newPage();
   page.setDefaultTimeout(10000);await context.route('**/*',r=>r.request().url().startsWith('file:')?r.continue():r.abort());
   const address=pathToFileURL(c.offline).href;const start=performance.now();await page.goto(address+'#'+(c.work?'work-'+c.work:'repository-explanation'));await page.evaluate(()=>document.fonts.ready);
   const firstDisplayMs=performance.now()-start;
   assert.equal(await page.locator('script').count(),0,'export has no script dependency');
   assert.deepEqual(await page.evaluate(fragmentErrors),[],'unique native targets and complete returns');
   const initial=await page.evaluate(()=>({elements:document.querySelectorAll('*').length,styles:document.querySelectorAll('style').length,codeBytes:[...document.querySelectorAll('.cited-code code')].reduce((n,c)=>n+new TextEncoder().encode(c.textContent).length,0),navigation:performance.getEntriesByType('navigation').map(n=>({domContentLoaded:n.domContentLoadedEventEnd,load:n.loadEventEnd,transferSize:n.transferSize,encodedBodySize:n.encodedBodySize}))}));
   const points=await page.locator('.source-point').evaluateAll((ns,id)=>ns.filter(n=>n.dataset.readerPublication===id).map(n=>({id:n.id,unit:Number(n.dataset.unitIndex),citation:Number(n.dataset.citationIndex),binding:JSON.parse(n.dataset.citationBinding),code:n.dataset.citedCode})),publicId);
   assert.equal(points.length,pub.prose.reduce((n,p)=>n+p.citations.length,0));
   let completeBytes=0;
   for(const p of points){
    const expected=pub.prose[p.unit].citations[p.citation];assert.deepEqual(p.binding,expected,'complete independent claim/Source/revision/range binding');
    const original=fs.readFileSync(path.join(c.repository,expected.locator)).subarray(expected.start_byte,expected.end_byte);assert.equal(hash(original),expected.range_sha256);
    const code=page.locator('.cited-code[id="'+p.code+'"] code');
    const included=pub.source_excerpts.some(e=>e.citation.source_id===expected.source_id&&e.citation.start_byte===expected.start_byte&&e.citation.end_byte===expected.end_byte&&e.citation.field===expected.field);
    assert.equal(await code.count(),included?1:0,'exact available embedding, no union/fallback');
    if(included){assert.equal(await code.textContent(),original.toString('utf8'));completeBytes+=original.length;}
   }
   let origins=0,choices=0;
   for(const d of pub.diagrams){
    const figure=page.locator('.admitted-diagram[data-semantic-fingerprint="'+d.semantic_fingerprint+'"]'),figureId=await figure.getAttribute('id');
    const m=d.meaning,nodes=m.nodes||m.participants,edges=m.relations||m.groups?.flatMap(g=>g.messages)||[];
    const inventory=[{id:'title',label:d.title},...nodes,...edges,...nodes.filter(n=>n.identifier).map(n=>({id:n.id+':identifier',label:n.identifier})),...(m.relations||[]).filter(e=>e.condition).map(e=>({id:e.id+':condition',label:e.condition})),...(m.groups||[]).flatMap((g,i)=>g.condition?[{id:'guard:'+(i+1),label:g.condition}]:[]),...(m.owner?[{id:'state:owner',label:m.owner}]:[])];
    for(const role of inventory){
     // Title is a supported origin only when the diagram's public role table exposes it.
     const element=page.locator('[id="'+figureId+'-'+role.id+'"]');if(!await element.count()){assert.equal(role.id,'title');continue;}
     const claim=figureId+'-'+role.id+'-claim',unit=pub.prose[role.label.unit];
     const ranges=unit.citations.map((q,n)=>({q,n})).filter(({q})=>q.claim_start_byte<=role.label.start_byte&&q.claim_end_byte>=role.label.end_byte);
     assert(ranges.length);origins++;
     for(const {q,n} of ranges.slice(0,config.all_ranges?ranges.length:2)){
      const alias=claim+'-source-c'+(n+1);await page.goto(address+'#'+alias);
      const point=await citationPoint(page,'#'+alias),native=await nativeEvidenceContext(point);assert.equal(await native.isVisible(),true);
      assert.equal(await native.locator(':scope > .evidence-claim').textContent(),role.label.text);
      assert.equal(await point.getAttribute('data-source-id'),q.source_id);
      const chooser=native.locator('.native-range-links a');assert.equal(await chooser.count(),ranges.length>1?ranges.length:0,'all legitimate contextual choices');
      if(ranges.length>1)assert.deepEqual(await chooser.evaluateAll(ns=>ns.map(n=>n.getAttribute('href'))),ranges.map(({n})=>'#'+claim+'-source-c'+(n+1)));
      const code=await openEmbeddedCode(point);if(await code.count()){const original=fs.readFileSync(path.join(c.repository,q.locator)).subarray(q.start_byte,q.end_byte).toString('utf8');assert.equal(await code.textContent(),original);assert.equal(await code.isVisible(),true);}
      const back=native.locator('a.return');assert.equal(await back.getAttribute('href'),'#'+figureId+'-'+role.id);await back.focus();await back.press('Enter');assert.equal(new URL(page.url()).hash,'#'+figureId+'-'+role.id);assert.equal(await element.isVisible(),true);assert.equal(await element.evaluate(n=>n===document.activeElement),true,'native keyboard return focuses the exact element');
      choices++;
     }
    }
   }
   await page.screenshot({path:path.join(out,c.key+'-'+width+'-native.png')});result.rows.push({key:c.key,width,publicId,firstDisplayMs,initial,points:points.length,claimBoundCodeBytes:completeBytes,origins,choices});write('progress-'+result.rows.length+'.json',result);
   await context.close();
  }
 } result.outcome='passed';}finally{await browser.close();write('result.json',result);}
 console.log(JSON.stringify(result.rows));
}
main().catch(e=>{console.error(e);process.exitCode=1;});
