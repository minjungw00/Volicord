// Factory contracts independent of the publication/navigation browser journeys.
const assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync('rebuild/crates/volicord-viewer/src/reader_runtime.js','utf8');
const callbacks=new Map(),observers=[],document=new EventTarget();
document.body={};let sequence=0;
class Observer {
 constructor(callback){this.callback=callback;this.connected=true;observers.push(this);}
 observe(){} disconnect(){this.connected=false;}
}
const context={AbortController,Map,Set,document,ResizeObserver:Observer,
 requestAnimationFrame:fn=>{const id=++sequence;callbacks.set(id,fn);return id;},
 cancelAnimationFrame:id=>callbacks.delete(id)};
vm.createContext(context);vm.runInContext(source+'\nglobalThis.test={createReaderLifetime,createReaderModal};',context);
const {createReaderLifetime,createReaderModal}=context.test;
const life=createReaderLifetime(),events=new EventTarget();let called=0,disposed=0;
life.on(events,'change',()=>called++);life.cleanup(()=>disposed++);
life.frame(()=>called++);life.observe({isConnected:true},()=>called++);
events.dispatchEvent(new Event('change'));assert.equal(called,1);
life.dispose();life.dispose();events.dispatchEvent(new Event('change'));
assert.equal(called,1);assert.equal(disposed,1);assert.equal(callbacks.size,0);
assert.equal(observers[0].connected,false);observers[0].callback();assert.equal(called,1);

// One inert owner survives another owner's dismissal; original inert state is
// restored exactly, rather than indiscriminately setting every node to false.
function root(parent){return {parentElement:parent,isConnected:true,inert:false,attrs:{},matches:()=>false,
 setAttribute(k,v){this.attrs[k]=v;},removeAttribute(k){delete this.attrs[k];},
 querySelectorAll:()=>[],contains:()=>false,focus(){}};}
const body=document.body;body.children=[];
const drawer=root(body),figure=root(body),evidence=root(body),untouched=root(body);
untouched.inert=true;body.children=[drawer,figure,evidence,untouched];
const modalLife=createReaderLifetime(),modal=createReaderModal(modalLife);
modal.set('drawer',drawer,()=>{});assert.equal(figure.inert,true);
modal.set('evidence',evidence,()=>{});modal.set('drawer',null);
assert.equal(evidence.inert,false);assert.equal(figure.inert,true);
assert.equal(evidence.attrs['aria-modal'],'true');assert.equal(drawer.attrs['aria-modal'],undefined);
modal.set('figure',figure,()=>{});modal.set('evidence',null);
assert.equal(figure.inert,false);assert.equal(figure.attrs['aria-modal'],'true');
modalLife.dispose();assert.equal(drawer.inert,false);assert.equal(figure.inert,false);
assert.equal(evidence.inert,false);assert.equal(untouched.inert,true);
assert.equal(figure.attrs['aria-modal'],undefined);
console.log('Reader lifetime and modal handoff contracts passed');
