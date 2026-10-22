const {test} = require('node:test'), assert = require('node:assert/strict');
const fs = require('node:fs'), os = require('node:os'), path = require('node:path');
const {createOutput,publication} = require('./product_review_controls.cjs');
test('failed and successful observations cannot be overwritten by reruns',()=>{
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'reader-review-'));
  try {const out=path.join(root,'attempt');createOutput(out);fs.writeFileSync(path.join(out,'result.json'),'original failure');
    assert.throws(()=>createOutput(out),/EEXIST/);assert.equal(fs.readFileSync(path.join(out,'result.json'),'utf8'),'original failure');
    createOutput(path.join(root,'retry'));
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
