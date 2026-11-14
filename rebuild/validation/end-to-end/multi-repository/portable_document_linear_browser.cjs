// Complete portable document rows compared to separately retained public meaning.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict'),crypto=require('node:crypto');
const C=JSON.parse(fs.readFileSync(process.argv[2]));
const digest=b=>crypto.createHash('sha256').update(b).digest('hex');
const label=(l,p)=>{
  const claim=Buffer.from(p.prose[l.unit].text);
  assert(Number.isInteger(l.start_byte)&&Number.isInteger(l.end_byte)&&l.start_byte>=0&&l.end_byte>l.start_byte&&l.end_byte<=claim.length,'admitted supporting claim range');
  // Labels are admitted wording supported by this range, not literal excerpts.
  assert.equal(typeof l.text,'string');assert(l.text.length,'admitted label');return l.text;
};
(async()=>{
  const manifest=JSON.parse(fs.readFileSync(C.manifest)),expected=new Map();
  for(const entry of C.publications){
    const pub=JSON.parse(fs.readFileSync(entry.public)),key=entry.journey+'/'+entry.language;
    if(!expected.has(key))expected.set(key,[]);
    for(const d of pub.diagrams)expected.get(key).push({d,pub,title:label(d.title,pub)});
  }
  fs.mkdirSync(C.output,{recursive:false});
  const browser=await require(C.playwright).chromium.launch({executablePath:C.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:C.fontconfig}});
  const rows=[];
  try{
    for(const file of manifest.files){
      const bytes=fs.readFileSync(file.path);assert.equal(digest(bytes),file.sha256,'actual exported bytes');
      if(file.format!=='html')continue;
      for(const width of C.widths||[390,1440]){
        const context=await browser.newContext({viewport:{width,height:900},javaScriptEnabled:false});
        const page=await context.newPage();page.setDefaultTimeout(15000);page.setDefaultNavigationTimeout(15000);
        await context.route('**/*',r=>/^(file|data):/.test(r.request().url())?r.continue():r.abort());
        try{
          await page.goto('file://'+path.resolve(file.path));
          const candidates=expected.get(file.journey+'/'+file.language)||[];
          const paragraphs=page.locator('.claim p');let figures=0;
          for(let i=0;i<await paragraphs.count();i++){
            const p=paragraphs.nth(i),text=await p.textContent();
            if(!/^(Diagram \(linear reading\):|도식 · 순서대로 읽기:)/.test(text))continue;
            const original=candidates.find(x=>text.split('\n')[0].endsWith(x.title));assert(original,'original separately retained diagram');
            const {d,pub}=original,meaning=d.meaning,nodes=meaning.nodes||meaning.participants;
            const identity=id=>{const n=nodes.find(n=>n.id===id);assert(n,'admitted endpoint');return label(n.label,pub)+(n.identifier?' ('+label(n.identifier,pub)+')':'')};
            for(const n of nodes)assert(text.includes(identity(n.id)),'complete named node');
            for(const edge of meaning.relations||meaning.groups.flatMap(g=>g.messages)){
              assert(text.includes(identity(edge.from)+' → '+identity(edge.to)),'actual direction');
              assert(text.includes(label(edge.label,pub)),'complete relationship wording');
              if(edge.condition)assert(text.includes(label(edge.condition,pub)),'actual guard');
            }
            if(meaning.owner)assert(text.includes(label(meaning.owner,pub)),'state owner');
            for(const group of meaning.groups||[]){
              if(group.condition)assert(text.includes(label(group.condition,pub)),'group guard');
              if(group.order==='parallel')assert(/parallel \(no relative order asserted\)|병렬 \(상대 순서 없음\)/.test(text),'parallel is not invented sequence');
            }
            assert(!/\[diagram=|; node=|\[edge=|\[message=/.test(text),'inspection syntax stays out of narrative');
            const layout=await p.evaluate(p=>{
              const opened=[];for(let n=p.parentElement;n;n=n.parentElement)if(n.tagName==='DETAILS'&&!n.open){n.open=true;opened.push(n.className)}
              const node=p.firstChild,lines=p.textContent.split('\n'),positions=[];let offset=0;
              for(const line of lines){if(line.trim()){const r=document.createRange();r.setStart(node,offset);r.setEnd(node,offset+line.length);const rects=[...r.getClientRects()];positions.push({first:rects[0]?.top,last:rects.at(-1)?.bottom})}offset+=line.length+1}
              return {whiteSpace:getComputedStyle(p).whiteSpace,positions,opened};
            });
            assert.equal(layout.whiteSpace,'pre-wrap');
            for(let n=1;n<layout.positions.length;n++)assert(layout.positions[n].first>=layout.positions[n-1].last-.5,'independent rows remain visually separate');
            figures++;rows.push({file:file.path,width,title:original.title,rows:layout.positions.length,opened_supporting:layout.opened});
          }
          if(file.kind==='project-architecture-guide')assert(figures,'repository guide retains its actual diagram reading');
          await page.screenshot({path:path.join(C.output,file.journey+'-'+file.language+'-'+file.kind+'-'+width+'.png')});
        }finally{await context.close()}
      }
    }
    assert(rows.length,'actual diagram occurrences');
    fs.writeFileSync(path.join(C.output,'result.json'),JSON.stringify({status:'passed',observations:rows,scope:'Actual portable HTML linear roles and row geometry; unchanged Markdown/HTML input hashes. No graphical SVG, independent execution, semantic entailment, Human or release verdict.'},null,2),{flag:'wx'});
    console.log(JSON.stringify({status:'passed',observations:rows.length}));
  }finally{await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
