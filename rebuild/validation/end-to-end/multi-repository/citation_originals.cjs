// Independent validation originals; never a permission or filesystem fallback
// for an actual public Source read.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto'),assert=require('node:assert/strict');
const digest=bytes=>'sha256:'+crypto.createHash('sha256').update(bytes).digest('hex');
const ordered=value=>Array.isArray(value)?value.map(ordered):value&&typeof value==='object'?Object.fromEntries(Object.keys(value).sort().map(key=>[key,ordered(value[key])])):value;
function identity(value){assert(Array.isArray(value)&&value.length===16&&value.every(n=>Number.isInteger(n)&&n>=0&&n<=255),'original identity bytes');return Buffer.from(value).toString('hex');}
function citationBytes(c,q){
 let body;
 if(q.field==='body'){
  assert(['current_code','before_code'].includes(q.use_kind),'file evidence role');
  const root=fs.realpathSync(c.repository),file=fs.realpathSync(path.resolve(root,q.locator)),relative=path.relative(root,file);
  assert(!path.isAbsolute(relative)&&relative!=='..'&&!relative.startsWith('..'+path.sep),'original file escapes repository');body=fs.readFileSync(file);
 }else{
  assert(['stdout','stderr'].includes(q.field),'unsupported original field');assert.equal(q.use_kind,'observed_execution','command evidence role');
  const raw=JSON.parse(fs.readFileSync(c.source_observations[q.source_id])),o=raw.observation,b=raw.citation_basis;
  assert.equal(o.format_kind,'volicord_source_observation');assert.equal(o.format_version,4,'current original observation format');
  assert.equal(identity(o.project_id),c.project,'original Project');assert(c.work,'original Work');assert.equal(identity(o.work_item_id),c.work,'original Work');
  assert.equal(identity(o.source_id),q.source_id,'original Source');assert.equal(identity(o.repository_source_id),q.repository_source_id,'original repository Source');
  for(const key of ['source_id','source_revision','analysis_snapshot_id','repository_snapshot_id','repository_source_id','observation_fingerprint'])assert.equal(b[key],q[key],'original binding '+key);
  assert.equal(o.source_revision,q.source_revision);assert.equal(o.analysis_snapshot_id,q.analysis_snapshot_id);assert.equal(o.repository_snapshot_id,q.repository_snapshot_id);
  assert.equal(digest(Buffer.from(JSON.stringify(ordered(o)))),raw.observation_fingerprint,'original observation fingerprint');assert.equal(raw.observation_fingerprint,q.observation_fingerprint);
  assert.equal(o.detail.kind,'command','original command kind');assert.equal(raw.command_source.command_label,q.locator,'original command label');
  const stream=o.detail[q.field];assert(['whole_stream','bounded_excerpt'].includes(stream.representation));assert.equal(typeof stream.body,'string');body=Buffer.from(stream.body);
 }
 assert(Number.isSafeInteger(q.start_byte)&&Number.isSafeInteger(q.end_byte)&&q.start_byte>=0&&q.start_byte<q.end_byte&&q.end_byte<=body.length,'original byte range');
 const selected=body.subarray(q.start_byte,q.end_byte);new TextDecoder('utf-8',{fatal:true}).decode(selected);assert.equal(digest(selected),q.range_sha256,'original range hash');return selected;
}
module.exports={citationBytes};
