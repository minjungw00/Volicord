// Remove actual tasks while retaining the separately declared public corpus.
// Run the real driver, which must refuse before launching a browser/Viewer.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const {spawnSync}=require('node:child_process');
const {publicationCoverage}=require('./publication_coverage.cjs');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
assert(config.complete_publications && config.publications?.length);
const full=publicationCoverage(config.cases,config.publications);
assert(full.complete,'positive control covers every declared publication');
fs.mkdirSync(config.output,{recursive:false});
const rows=[];
for(const p of full.publications) for(const figure of p.figures) {
  const tag=p.publication+'-'+figure;
  const cases=config.cases.filter(c=>{
    const pub=JSON.parse(fs.readFileSync(c.public));
    return pub.provenance.publication_id!==p.publication || c.diagram!==figure;
  });
  const input=path.join(config.output,tag+'.json');
  const output=path.join(config.output,tag+'-unexpected-browser-output');
  fs.writeFileSync(input,JSON.stringify({...config,cases,output},null,2),{flag:'wx'});
  const result=spawnSync(process.execPath,[path.join(__dirname,'grounded_diagrams_browser.cjs'),input],{encoding:'utf8'});
  fs.writeFileSync(path.join(config.output,tag+'.stderr'),result.stderr,{flag:'wx'});
  assert.equal(result.status,1,'missing actual figure must fail');
  assert(result.stderr.includes('whole-publication reading omitted figures'));
  assert(!fs.existsSync(output),'coverage refusal precedes browser output');
  rows.push({publication:p.publication,omitted:figure,exit:result.status});
}
fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify({status:'passed',positive:full,negative:rows},null,2),{flag:'wx'});
console.log('whole-publication omission controls passed:',rows.length);
