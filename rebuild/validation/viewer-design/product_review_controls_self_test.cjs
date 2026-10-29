const {test} = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), os = require('node:os'), path = require('node:path');
const {createOutput,publication} = require('./product_review_controls.cjs');
test('label dictionaries and known alias padding cannot replace necessary explanation',()=>{
 const {explanatoryObligations}=require('../end-to-end/multi-repository/reader_hierarchy.cjs');
 const expected={mechanisms:[['submitted','allSettled','rejection'],['token','/player-closed']],forbidden:['Sync service is Sync service']};
 assert.deepEqual(explanatoryObligations(['Removal captures submitted promises and awaits allSettled; rejection settles too.','With a token, attempt POST /player-closed.'],expected),[]);
 assert.equal(explanatoryObligations(['Sync service; Sync A; Sync B; Token; Closed'],expected).length,2);
 assert(explanatoryObligations(['Removal captures submitted promises and awaits allSettled; rejection settles too.','With a token, attempt POST /player-closed.','Sync service is Sync service'],expected).some(e=>e.startsWith('label-dictionary')));
});
test('complete reading requires independently declared case, language and figure membership',()=>{
 const {corpusObligations}=require('../end-to-end/multi-repository/reader_hierarchy.cjs');
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'reading-corpus-'));
 try{
  const file=path.join(root,'public.json'),c={key:'example',public:file,language:'en'};
  fs.writeFileSync(file,JSON.stringify({diagrams:[{id:'flow',meaning:{kind:'flowchart'}}],prose:[{text:'input leads to output'}]}));
  const expected={cases:{example:{language:'en',figures:{flowchart:['flow']},mechanisms:[['input','output']],forbidden:[]}}};
  corpusObligations([c],expected);
  for(const candidates of [[],[{...c,language:'ko'}],[c,{...c,key:'extra'}]])assert.throws(()=>corpusObligations(candidates,expected));
  for(const diagrams of [[],[{id:'flow',meaning:{kind:'sequence'}}]]){
   fs.writeFileSync(file,JSON.stringify({diagrams,prose:[{text:'input leads to output'}]}));assert.throws(()=>corpusObligations([c],expected));
  }
 }finally{fs.rmSync(root,{recursive:true});}
});
test('queued native success retains its caller guard rather than deleting a reachable edge',()=>{
 const {relationObligations}=require('../end-to-end/multi-repository/reader_hierarchy.cjs');
 // Independent command-service facts: acknowledgement requires delivered, but
 // a resolved unpaired open launch with the same generation can finish queued.
 const expected={relations:[{diagram:'requests',id:'native-success',from:'queued',to:'succeeded',kind:'transition',condition_terms:['unpaired','open','launch','generation']}]};
 const relation={id:'native-success',from:'queued',to:'succeeded',kind:'transition',condition:{text:'unpaired open · launch resolved · same generation'}};
 const diagrams=relations=>[{id:'requests',meaning:{kind:'state',relations}}];
 assert.deepEqual(relationObligations(diagrams([relation]),expected),[]);
 for(const relations of [[],[{...relation,condition:null}],[{...relation,from:'delivered'}],
  ...expected.relations[0].condition_terms.map(term=>[{...relation,condition:{text:relation.condition.text.replace(term,'')}}])]){
  assert(relationObligations(diagrams(relations),expected).length,'omitting the transition or a material guard fails');
 }
});
test('same Source coordinates in Repository and Work retain separate live/offline captures',()=>{
  const {sourceCaptureName}=require('./product_review_controls.cjs');
  const names=[];
  for(const destination of ['repository','work'])for(const offline of [false,true])
    for(const task of [null,{unit_index:5,citation_index:1}])names.push(sourceCaptureName(destination,offline,task));
  assert.equal(new Set(names).size,8);
  assert.throws(()=>sourceCaptureName('unscoped',false,null),/destination/);
});
test('failed and successful observations cannot be overwritten by reruns',()=>{
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'reader-review-'));
  try {const out=path.join(root,'attempt');createOutput(out);fs.writeFileSync(path.join(out,'result.json'),'original failure');
    assert.throws(()=>createOutput(out),/EEXIST/);assert.equal(fs.readFileSync(path.join(out,'result.json'),'utf8'),'original failure');
    createOutput(path.join(root,'retry'));
  } finally {fs.rmSync(root,{recursive:true});}
});
// Exercise the actual CLI entry point, including its failure/finally writes.
// These authored process controls require no installed browser or Product Runtime.
test('citation entry point preserves attempts before setup and across teardown failure',()=>{
 const {spawnSync}=require('node:child_process');
 const root=fs.mkdtempSync(path.join(os.tmpdir(),'citation-attempt-'));
 const driver=path.resolve(__dirname,'../end-to-end/multi-repository/citation_reading_browser.cjs');
 const snapshot=out=>Object.fromEntries(fs.readdirSync(out).sort().map(n=>[n,fs.readFileSync(path.join(out,n))]));
 try {
  const fixture=path.join(root,'fixture');fs.mkdirSync(fixture);
  fs.writeFileSync(path.join(fixture,'fixture.json'),JSON.stringify({runtime:fixture,project:'authored-control',originals:[],answers:{source_excerpts:[]}}));
  fs.writeFileSync(path.join(fixture,'offline.html'),'authored offline control');
  const fonts=path.join(root,'fc-list');fs.writeFileSync(fonts,'#!/bin/sh\necho authored-font\n',{mode:0o700});
  const viewer=path.join(root,'viewer');fs.writeFileSync(viewer,`#!${process.execPath}\nconsole.error('http://127.0.0.1:1/');setInterval(()=>{},1000);\n`,{mode:0o700});
  const playwright=path.join(root,'playwright.cjs');
  fs.writeFileSync(playwright,`const fs=require('node:fs'),path=require('node:path');
   fs.writeFileSync(path.join(__dirname,'loaded'),'setup reached');
   exports.chromium={launch:async()=>({version:()=> 'authored process control',
    newContext:async()=>{const c=JSON.parse(fs.readFileSync(process.argv[2]));
     fs.writeFileSync(path.join(c.output,'live-en-1440.html'),'original partial HTML');
     fs.writeFileSync(path.join(c.output,'en-1440.png'),Buffer.from([0,255,1]));
     throw Error('first controlled reading failure');},close:async()=>{throw Error('secondary teardown failure');}})};`);
  let index=0;
  function invoke(output,overrides={}) {
   const config=path.join(root,`config-${index++}.json`);
   fs.writeFileSync(config,JSON.stringify({output,fixture,viewer,playwright,cli:viewer,...overrides}));
   const run=spawnSync(process.execPath,[driver,config],{encoding:'utf8',timeout:15000,env:{...process.env,PATH:root+path.delimiter+process.env.PATH}});
   assert.ifError(run.error);return run;
  }
  for(const outcome of ['passed','failed']) {
   const output=path.join(root,outcome);fs.mkdirSync(output);
   for(const name of ['result.json','viewer.stdout','viewer.stderr','korean-fonts.stdout','korean-fonts.stderr','delete.stdout','delete.stderr','live-en-1440.html','en-1440.png'])
    fs.writeFileSync(path.join(output,name),name==='result.json'?JSON.stringify({outcome}):Buffer.from([0,255,2]));
   const original=snapshot(output);const run=invoke(output);
   assert.equal(run.status,1);assert.match(run.stderr,/EEXIST/);assert.deepEqual(snapshot(output),original);
  }
  // Existing directory symlinks and dangling symlinks follow mkdir's existing
  // EEXIST policy; a link cannot redirect a retry into preserved output.
  for(const target of ['passed','missing']) {
   const output=path.join(root,'link-'+target);fs.symlinkSync(path.join(root,target),output);
   const original=snapshot(path.join(root,'passed'));const run=invoke(output);
   assert.equal(run.status,1);assert.match(run.stderr,/EEXIST/);
   assert.deepEqual(snapshot(path.join(root,'passed')),original);assert.equal(fs.readlinkSync(output),path.join(root,target));
  }
  const file=path.join(root,'existing-file');fs.writeFileSync(file,'original file');
  assert.match(invoke(file).stderr,/EEXIST/);assert.equal(fs.readFileSync(file,'utf8'),'original file');
  const empty=path.join(root,'empty');fs.mkdirSync(empty);
  assert.match(invoke(empty).stderr,/EEXIST/);assert.deepEqual(fs.readdirSync(empty),[]);
  assert.equal(invoke(path.join(root,'missing-parent','attempt')).status,1);
  assert.equal(fs.existsSync(path.join(root,'missing-parent')),false,'no recursive parent creation');
  assert.equal(fs.existsSync(path.join(root,'loaded')),false,'refusal precedes even dependency setup');
  const failed=path.join(root,'first');assert.equal(invoke(failed).status,1);
  const result=JSON.parse(fs.readFileSync(path.join(failed,'result.json')));
  assert.match(result.error,/first controlled reading failure/);
  assert.match(result.teardown_errors[0],/secondary teardown failure/);
  assert.equal(result.listener.signal,'SIGTERM');
  assert.match(fs.readFileSync(path.join(failed,'viewer.stderr'),'utf8'),/127.0.0.1/);
  assert.ok(result.controls_sha256);assert.equal(result.attempt_output,failed);
  const original=snapshot(failed);assert.match(invoke(failed).stderr,/EEXIST/);assert.deepEqual(snapshot(failed),original);
  const retry=path.join(root,'retry');assert.equal(invoke(retry).status,1);
  assert.notEqual(JSON.parse(fs.readFileSync(path.join(retry,'result.json'))).attempt_output,result.attempt_output);
  assert.deepEqual(snapshot(failed),original,'new attempt preserves first failure and partial bytes');
  for(const overrides of [{playwright:path.join(root,'missing-module')},{viewer:path.join(root,'missing-viewer')},{fontconfig:path.join(root,'missing-fonts')}]) {
   const output=path.join(root,`setup-${index}`);assert.equal(invoke(output,overrides).status,1);
   assert.equal(JSON.parse(fs.readFileSync(path.join(output,'result.json'))).outcome,'failed');
   const before=snapshot(output);assert.match(invoke(output).stderr,/EEXIST/);assert.deepEqual(snapshot(output),before);
  }
 } finally {fs.rmSync(root,{recursive:true});}
});
test('populated and genuinely workless positives each require their own current repository article',()=>{
  for (const work_history of [[],[{work_item_id:'work',answers:{explanation_state:'current'}}]]) {
    const c={expected_repository_state:'current',...(work_history.length?{normal_work:'work'}:{})};
    assert.equal(publication(c,{repository_explanation:{explanation_state:'current'},work_history}).work_count,work_history.length);
    for (const state of ['missing','stale','withheld'])assert.throws(()=>publication(c,{repository_explanation:{explanation_state:state},work_history}),/selected Project/);
  }
});
test('unavailable controls remain legitimate but cannot supply normal Work success',()=>{
  const status={repository_explanation:{explanation_state:'missing'},work_history:[{work_item_id:'work',answers:{explanation_state:'stale'}}]};
  assert.equal(publication({expected_repository_state:'missing'},status).repository_state,'missing');
  assert.throws(()=>publication({normal_work:'work'},status),/normal article/);
  assert.throws(()=>publication({normal_work:'foreign'},status),/exact normal Work/);
});
const {processCpuDelta}=require('./product_review_controls.cjs');
test('CPU compares process counters across navigation, retaining new and lost processes',()=>{
  const full=processCpuDelta([{id:1,cpuTime:3}],[{id:1,cpuTime:3.125},{id:2,cpuTime:.25}]);
  assert.deepEqual(full,{browser_process_cpu_ms:375,cpu_processes:2,cpu_omitted_processes:0,cpu_state:'observed'});
  assert.deepEqual(processCpuDelta([{id:1,cpuTime:3}],[]),{browser_process_cpu_ms:0,cpu_processes:0,cpu_omitted_processes:1,cpu_state:'partial'});
  for (const input of [[{id:1,cpuTime:2}],[{id:1,cpuTime:NaN}],[{id:1,cpuTime:3},{id:1,cpuTime:3}]])
    assert.throws(()=>processCpuDelta([{id:1,cpuTime:3}],input));
});
const {sourceTasks}=require('./product_review_controls.cjs');
const crypto=require('node:crypto');
const sha=text=>'sha256:'+crypto.createHash('sha256').update(text).digest('hex');
const citation={source_id:'important',source_revision:1,locator:'engine.py',field:'body',start_byte:20,end_byte:40,range_sha256:sha('code'),claim_start_byte:0,claim_end_byte:9,claim_sha256:sha('mechanism')};
const answers={explanation_state:'current',provenance:{realization_fingerprint:'exact-publication'},prose:[{text:'mechanism',citations:[citation]},{text:'unrelated last link',citations:[{...citation,source_id:'other'}]}]};
const task={state:'required',realization_fingerprint:'exact-publication',claims:[{unit_index:0,citation_index:0,text_sha256:sha('mechanism'),citation}]};
test('critical claim cannot pass on an unrelated working last link',()=>{
 const c={source_tasks:{repository:task}};
 assert.equal(sourceTasks(c,'repository',answers).length,1);
 for(const mutate of [a=>a.prose[0].citations=[],a=>a.prose[0].citations[0].source_id='other',a=>a.prose[0].citations[0].start_byte=0,a=>a.prose[0].text='different',a=>a.provenance.realization_fingerprint='replacement']) {
  const a=structuredClone(answers);mutate(a);assert.throws(()=>sourceTasks(c,'repository',a));
 }
 assert.throws(()=>sourceTasks({},'repository',answers),/explicit Source task/);
 assert.throws(()=>sourceTasks({source_tasks:{repository:{...task,claims:[]}}},'repository',answers),/critical claims/);
});
test('truthful absence and record-only scope need explicit applicability and cannot replace required code',()=>{
 const missing={explanation_state:'missing',prose:[]};
 assert.deepEqual(sourceTasks({source_tasks:{repository:{state:'absent',explanation_state:'missing'}}},'repository',missing),[]);
 assert.throws(()=>sourceTasks({source_tasks:{repository:task}},'repository',missing));
 assert.throws(()=>sourceTasks({source_tasks:{repository:{state:'absent',explanation_state:'missing'}}},'repository',answers));
 const records={explanation_state:'current',prose:[{text:'reported',citations:[]}]};
 assert.deepEqual(sourceTasks({source_tasks:{work:{state:'record_only'}}},'work',records),[]);
 assert.throws(()=>sourceTasks({source_tasks:{repository:{state:'record_only'}}},'repository',records));
 assert.throws(()=>sourceTasks({source_tasks:{work:{state:'record_only'}}},'work',answers));
});
test('multilingual article tasks retain exact language-specific publication identities',()=>{
 const other=structuredClone(answers);other.provenance.realization_fingerprint='other-language';
 const c={source_tasks:{work:{by_language:{en:task,ko:{...task,realization_fingerprint:'other-language'}}}}};
 assert.equal(sourceTasks(c,'work',answers,'en').length,1);
 assert.equal(sourceTasks(c,'work',other,'ko').length,1);
 assert.throws(()=>sourceTasks(c,'work',answers,'ko'),/publication replaced/);
 assert.throws(()=>sourceTasks(c,'work',answers,'fr'),/explicit Source task/);
});
const {citationResponse}=require('./product_review_controls.cjs');
test('response failure stays awaitable after a UI failure without an unhandled crash',async()=>{
 const pending=citationResponse({waitForResponse:()=>Promise.reject(Error('missing critical HTTP response'))},{},0);
 await new Promise(resolve=>setImmediate(resolve));
 await assert.rejects(pending,/missing critical HTTP response/);
});
