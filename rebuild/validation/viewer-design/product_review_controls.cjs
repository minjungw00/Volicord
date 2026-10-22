// Bounded review admission/measurements; never an oracle for prose meaning.
const fs = require('node:fs'), assert = require('node:assert/strict');
function createOutput(output) {
  // A rerun must have a new identity, including after a failed launch.
  fs.mkdirSync(output, {recursive:false});
}
function publication(caseInput, status) {
  const expected = caseInput.expected_repository_state;
  if (expected !== undefined) {
    assert.ok(['current','missing','stale','unavailable','withheld','deleted','unsupported'].includes(expected), 'explicit repository lifecycle expectation');
    assert.equal(status.repository_explanation?.explanation_state, expected,
      'selected Project repository publication must satisfy its declared case, independently of Work');
  }
  if (caseInput.normal_work) {
    const work = status.work_history.find(w => w.work_item_id === caseInput.normal_work);
    assert.ok(work, 'exact normal Work must exist');
    assert.equal(work.answers.explanation_state, 'current', 'normal article cannot pass as an unavailable control');
  }
  return {repository_state:status.repository_explanation?.explanation_state || 'not_returned',
    work_count:status.work_history.length};
}
function documentMetrics() {
  // Executed in the actual browser; counts include closed secondary layers.
  const codes = [...document.querySelectorAll('code')];
  return {dom_elements:document.querySelectorAll('*').length,
    code_elements:codes.length, token_spans:document.querySelectorAll('code [class^="syn-"]').length,
    code_utf8_bytes:codes.reduce((n,c)=>n+new TextEncoder().encode(c.textContent).length,0),
    visible_text_utf8_bytes:new TextEncoder().encode(document.body.innerText).length};
}
module.exports = {createOutput, publication, documentMetrics};

// CDP SystemInfo CPU is cumulative OS time across every thread of each process.
// Unlike document TaskDuration, these counters survive navigation. Processes
// lost between samples leave an explicit partial measurement.
function processCpuDelta(before, after) {
  const previous = new Map();
  for (const p of before) {
    assert.ok(!previous.has(p.id) && Number.isFinite(p.cpuTime) && p.cpuTime >= 0, 'valid CPU sample');
    previous.set(p.id,p.cpuTime);
  }
  const seen=new Set();let seconds=0;
  for (const p of after) {
    assert.ok(!seen.has(p.id) && Number.isFinite(p.cpuTime) && p.cpuTime >= 0, 'valid CPU sample');
    seen.add(p.id);
    const delta=p.cpuTime-(previous.get(p.id)||0);
    assert.ok(delta>=0,'process CPU counter regressed');seconds+=delta;
  }
  const omitted=[...previous.keys()].filter(id=>!seen.has(id)).length;
  return {browser_process_cpu_ms:seconds*1000,cpu_processes:after.length,
    cpu_omitted_processes:omitted,cpu_state:omitted?'partial':'observed'};
}
module.exports.processCpuDelta=processCpuDelta;

// Frozen task-to-claim bindings precede browser reading. They are technical
// correspondence expectations, never a prose-entailment or importance oracle.
function sourceTasks(caseInput, destination, answers, language=caseInput.language) {
  const configured=caseInput.source_tasks?.[destination];
  const task=configured?.by_language?configured.by_language[language]:configured;
  assert.ok(task,'explicit Source task applicability required for '+destination);
  assert.ok(answers,'exact selected public answers required');
  if(task.state==='absent') {
    assert.ok(['missing','stale','deleted','withheld','unavailable','unsupported','corrupt'].includes(task.explanation_state),'absence requires an exact non-current state');
    assert.equal(answers.explanation_state,task.explanation_state,'negative case lifecycle changed');
    assert.ok(answers.prose.every(u=>!u.citations.length),'unavailable article discloses no cited code');
    return [];
  }
  assert.equal(answers.explanation_state,'current','required publication must be current');
  if(task.state==='record_only') {
    assert.equal(destination,'work','repository normal outcome requires code reading');
    assert.ok(answers.prose.every(u=>!u.citations.length),'record-only applicability cannot hide code journeys');
    return [];
  }
  assert.equal(task.state,'required','typed Source applicability');
  assert.equal(answers.provenance.realization_fingerprint,task.realization_fingerprint,'task publication replaced');
  assert.ok(Array.isArray(task.claims)&&task.claims.length,'required critical claims cannot be empty');
  const sha=text=>'sha256:'+require('node:crypto').createHash('sha256').update(text).digest('hex');
  return task.claims.map(claim=>{
    assert.ok(Number.isSafeInteger(claim.unit_index)&&claim.unit_index>=0&&Number.isSafeInteger(claim.citation_index)&&claim.citation_index>=0,'exact unit/citation indices');
    const unit=answers.prose[claim.unit_index];assert.ok(unit,'critical unit missing');
    assert.equal(sha(unit.text),claim.text_sha256,'critical claim changed');
    const citation=unit.citations[claim.citation_index];
    assert.deepEqual(citation,claim.citation,'critical Source binding changed or missing');
    assert.equal(sha(Buffer.from(unit.text).subarray(citation.claim_start_byte,citation.claim_end_byte)),citation.claim_sha256,'exact claim bytes');
    return {...claim,text:unit.text,realization_fingerprint:task.realization_fingerprint};
  });
}
module.exports.sourceTasks=sourceTasks;

async function criticalLink(article,task) {
  const unit=article.locator(`.unit[id$="-p${task.unit_index+1}"]`);
  assert.equal(await unit.count(),1,'exact critical unit must be present');
  assert.equal(await unit.locator('.unit-text').textContent(),task.text);
  // Verification and supporting evidence are deliberate reader disclosures.
  // Use their actual controls so a task is never "opened" from a hidden link.
  for(const disclosure of await unit.locator('xpath=ancestor::details').all()) {
    if(!await disclosure.evaluate(n=>n.open))await disclosure.locator(':scope > summary').click();
  }
  await unit.scrollIntoViewIfNeeded();
  const link=unit.locator(`a.evidence-link[href$="-c${task.citation_index+1}"]`);
  assert.equal(await link.count(),1,'critical claim must own its exact Source control');
  return link;
}
async function criticalPoint(point,task) {
  assert.equal(await point.getAttribute('data-source-id'),task.citation.source_id,'critical Source identity');
  assert.equal(await point.locator('.claim-quote').textContent(),Buffer.from(task.text).subarray(task.citation.claim_start_byte,task.citation.claim_end_byte).toString('utf8'),'critical claim span');
}
module.exports.criticalLink=criticalLink;
module.exports.criticalPoint=criticalPoint;

function citationResponse(page,task,offset) {
 const pending=page.waitForResponse(r=>{const u=new URL(r.url());return u.pathname==='/citation'&&u.searchParams.get('unit')===String(task.unit_index)&&u.searchParams.get('citation')===String(task.citation_index)&&u.searchParams.get('offset')===String(offset)&&u.searchParams.get('realization')===task.realization_fingerprint;});
 // Awaiters still reject. Handle the promise immediately so a failed preceding
 // UI action cannot crash Node before the driver preserves failure and teardown.
 pending.catch(()=>{});
 return pending;
}
async function criticalBytes(page,point,task,offline,pending) {
 const hash=bytes=>require('node:crypto').createHash('sha256').update(bytes).digest('hex');
 let bytes=0,pages=0;
 if(offline) {
  const code=point.locator('pre.code-region code');assert.equal(await code.count(),1,'offline critical window cannot be omitted');
  const text=await code.textContent();bytes=Buffer.byteLength(text);
  assert.equal(bytes,task.citation.end_byte-task.citation.start_byte,'offline complete critical window');
  assert.equal('sha256:'+hash(Buffer.from(text)),task.citation.range_sha256,'offline exact critical bytes');
 }else{
   const inspection=point.locator('.citation-inspection');
   assert.equal(await inspection.getAttribute('data-realization'),task.realization_fingerprint);
   const chunks=[];let offset=0;const target=await point.getAttribute('id');
   while(true){
    const response=await pending;assert.equal(response.status(),200,'critical permitted read must succeed');
    const value=await response.json(),chunk=value.chunk;
    assert.deepEqual(chunk.citation,task.citation,'HTTP must return the frozen exact binding');
    assert.equal(chunk.displayed_start_byte,task.citation.start_byte+offset);
    assert.ok(Buffer.byteLength(chunk.text)>0&&Buffer.byteLength(chunk.text)<=4096);
    assert.equal(chunk.displayed_end_byte,chunk.displayed_start_byte+Buffer.byteLength(chunk.text));
    await page.waitForFunction(({id,text})=>document.getElementById(id)?.querySelector('.citation-inspection code')?.textContent===text,{id:target,text:chunk.text});
    chunks.push(chunk.text);pages++;bytes+=Buffer.byteLength(chunk.text);
    if(chunk.next_offset===null)break;
    assert.equal(chunk.next_offset,offset+Buffer.byteLength(chunk.text));offset=chunk.next_offset;
    pending=citationResponse(page,task,offset);await inspection.locator('button.citation-read').click();
   }
   assert.equal(bytes,task.citation.end_byte-task.citation.start_byte,'complete critical cited range');
   assert.equal('sha256:'+hash(Buffer.from(chunks.join(''))),task.citation.range_sha256,'complete critical Source hash');

 }
 return {bytes,pages};
}
module.exports.citationResponse=citationResponse;
module.exports.criticalBytes=criticalBytes;
