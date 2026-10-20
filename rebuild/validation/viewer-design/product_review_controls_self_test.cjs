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
