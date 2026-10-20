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
