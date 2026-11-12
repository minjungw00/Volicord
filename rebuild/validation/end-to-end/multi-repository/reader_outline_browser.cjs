// Component regression over the maintained outline owner; no generated prose or human verdict.
'use strict';
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const config=JSON.parse(fs.readFileSync(process.argv[2],'utf8'));
fs.mkdirSync(config.output,{recursive:false});
const result={kind:'reader_outline_component_control',human_observation:'not_established',checks:[]};
(async()=>{
 const browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox']});
 try{
  for(const locale of ['en','ko'])for(const width of [390,1440]){
   const labels=locale==='ko'?['작업 설명','관련 결정','검증과 한계','원문과 근거']:['Work explanation','Related Decisions','Verification and limits','Original records and evidence'];
   const ids=['work','decisions','checks','records'];
   const page=await browser.newPage({viewport:{width,height:850}});
   try{
    await page.setContent(`<!doctype html><html lang="${locale}"><meta charset="utf-8"><style>body{margin:0}header{height:180px}.document-nav{position:sticky;top:0;background:white;z-index:1}section{min-height:900px}details:not([open])>nav{display:none}</style><article class="doc" data-reading-layout="grouped"><header>Authored component fixture</header><details class="document-nav"><summary><span class="toc-cue">Initial placeholder</span><span class="toc-position"></span></summary><nav>${ids.map((id,i)=>`<div class="work-toc-group"><span class="toc-caption"><a href="#${id}">${labels[i]}</a></span><ol class="toc-primary"></ol></div>`).join('')}</nav></details>${ids.map((id,i)=>`<section id="${id}"><h2>${labels[i]}</h2><p>Separate authored fixture region.</p></section>`).join('')}<details><summary>Closed original</summary><div id="folded">Hidden target</div></details><footer style="height:1000px"></footer></article>`);
    await page.addScriptTag({content:fs.readFileSync(path.resolve('rebuild/crates/volicord-viewer/src/reader_outline.js'),'utf8')+`
      window.outline=createReaderOutline({lifetime:{disposed:false,on:(target,event,handler,options)=>target.addEventListener(event,handler,options),frame:callback=>requestAnimationFrame(callback),observe:(target,callback)=>{const observer=new ResizeObserver(callback);observer.observe(target);return()=>observer.disconnect();}},navigate:hash=>{location.hash=hash},closeDrawer:()=>{}});outline.updateCue();`});
    await page.evaluate(()=>document.fonts.ready);
    assert.equal(await page.locator('.toc-cue').textContent(),labels[0],'plain group link names the initial Work section');
    for(const index of [1,2,3]){
     await page.evaluate(id=>scrollTo(0,document.getElementById(id).offsetTop+80),ids[index]);
     await page.waitForFunction(label=>document.querySelector('.toc-cue').textContent===label,labels[index]);
     assert.equal(await page.locator('.document-nav a[aria-current="location"]').getAttribute('href'),'#'+ids[index]);
    }
    await page.evaluate(()=>{const a=document.createElement('a');a.href='#folded';a.textContent='Hidden original';const li=document.createElement('li');li.append(a);document.querySelector('.toc-primary').append(li);outline.updateCue();});
    assert.equal(await page.locator('.toc-cue').textContent(),labels[3],'folded original cannot replace the visible section');
    await page.screenshot({path:path.join(config.output,`${locale}-${width}.png`)});
    result.checks.push({locale,width,initial_group:labels[0],scrolled_groups:3,closed_target_ignored:true,status:'passed'});
   }finally{await page.close();}
  }
  result.status='passed';
 }finally{await browser.close();}
})().catch(error=>{result.status='failed';result.error=String(error.stack||error);process.exitCode=1;}).finally(()=>{
 fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify(result,null,2),{flag:'wx'});
 console.log(JSON.stringify({status:result.status,checks:result.checks.length,error:result.error}));
});
