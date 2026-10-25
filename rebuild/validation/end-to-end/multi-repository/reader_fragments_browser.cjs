const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const {fragmentErrors}=require('./reader_fragments.cjs');
const C=JSON.parse(fs.readFileSync(process.argv[2]));
const sha=p=>crypto.createHash('sha256').update(fs.readFileSync(p)).digest('hex');
(async()=>{
 fs.mkdirSync(C.output,{recursive:false});const browser=await require(C.playwright).chromium.launch({executablePath:C.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:C.fontconfig}});const rows=[];
 try{for(const c of C.cases){
  const context=await browser.newContext({viewport:{width:390,height:900},javaScriptEnabled:false});const page=await context.newPage();
  await context.route('**/*',r=>/^(file|data):/.test(r.request().url())?r.continue():r.abort());
  await page.goto('file://'+path.resolve(c.offline)+'#'+(c.work?'work-'+c.work:'repository-explanation'));await page.evaluate(()=>document.fonts.ready);
  await page.screenshot({path:path.join(C.output,c.key+'-initial.png')});
  assert.deepEqual(await page.evaluate(fragmentErrors),[],'actual complete public export');
  const link=page.locator('svg a.diagram-element').first();assert.equal(await link.count(),1);
  const href=await link.getAttribute('href');await link.evaluate(a=>a.setAttribute('href','#missing-native-svg-origin'));
  const broken=await page.evaluate(fragmentErrors);assert(broken.includes('bad link #missing-native-svg-origin'),'broken SVG link must fail');
  await link.evaluate((a,href)=>a.setAttribute('href',href),href);
  const id=await link.getAttribute('id');await page.evaluate(id=>{const n=document.createElement('span');n.id=id;n.dataset.fragmentCounterexample='true';document.body.append(n);},id);
  const duplicate=await page.evaluate(fragmentErrors);assert(duplicate.includes(id+': 2 targets'),'ambiguous SVG identity must fail');
  await page.locator('[data-fragment-counterexample]').evaluate(n=>n.remove());assert.deepEqual(await page.evaluate(fragmentErrors),[]);
  rows.push({case:c.key,html_sha256:sha(c.offline),broken,duplicate});await context.close();
 }}finally{await browser.close();}
 fs.writeFileSync(path.join(C.output,'result.json'),JSON.stringify({status:'passed',rows,driver:sha(__filename),helper:sha(path.join(__dirname,'reader_fragments.cjs'))},null,2),{flag:'wx'});
})().catch(e=>{console.error(e);process.exitCode=1});
