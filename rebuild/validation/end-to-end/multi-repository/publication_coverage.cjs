const fs = require('node:fs');
const assert = require('node:assert/strict');

// The complete public publication, not the selected browser tasks, defines the
// figure inventory. A partial diagnostic cannot certify whole-corpus coverage.
function publicationCoverage(cases, required = []) {
  const publications = new Map();
  const add = c => {
    const p = JSON.parse(fs.readFileSync(c.public, 'utf8'));
    assert.equal(p.explanation_state, 'current', 'coverage requires a current public publication');
    const id = p.provenance.publication_id;
    assert(id, 'coverage requires publication identity');
    const meaning = p.diagrams.map(d => ({id: d.id, fingerprint: d.semantic_fingerprint}));
    const existing = publications.get(id);
    if (existing) {
      assert.equal(existing.realization, p.provenance.realization_fingerprint);
      assert.deepEqual(existing.meaning, meaning, 'one publication cannot have different figure inventories');
    } else {
      publications.set(id, {publication: id, realization: p.provenance.realization_fingerprint,
        language: c.language, meaning, selected: new Set()});
    }
    return id;
  };
  for (const p of required) add(p);
  for (const c of cases) {
    const id = add(c);
    assert(publications.get(id).meaning.some(d => d.id === c.diagram), 'selected figure belongs to the complete publication');
    publications.get(id).selected.add(c.diagram);
  }
  assert(publications.size, 'coverage requires actual public publications');
  const rows = [...publications.values()].map(p => ({
    publication: p.publication, realization: p.realization, language: p.language,
    figures: p.meaning.map(d => d.id), selected: [...p.selected],
    missing: p.meaning.filter(d => !p.selected.has(d.id)).map(d => d.id)
  }));
  return {publications: rows, complete: rows.every(p => p.missing.length === 0)};
}
module.exports = {publicationCoverage};
