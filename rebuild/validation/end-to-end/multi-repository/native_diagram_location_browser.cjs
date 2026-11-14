// Actual script-free exports: native location must clear the sticky document UI.
// This measures navigation geometry, never comprehension or Source completeness.
const fs=require('node:fs'),path=require('node:path'),assert=require('node:assert/strict');
const config=JSON.parse(fs.readFileSync(process.argv[2]));
(async()=>{
  fs.mkdirSync(config.output,{recursive:false});
  const browser=await require(config.playwright).chromium.launch({executablePath:config.chromium,args:['--no-sandbox'],env:{...process.env,FONTCONFIG_FILE:config.fontconfig}});
  const observations=[];
  try {
    for(const c of config.cases) for(const width of config.widths||[390,768,1440]) {
      const context=await browser.newContext({viewport:{width,height:844},javaScriptEnabled:false});
      const page=await context.newPage();page.setDefaultTimeout(15000);page.setDefaultNavigationTimeout(15000);
      await context.route('**/*',r=>/^(file|data):/.test(r.request().url())?r.continue():r.abort());
      try {
        await page.goto('file://'+path.resolve(c.offline));
        await page.evaluate(id=>{location.hash=id},c.work?'work-'+c.work:'repository-explanation');
        const pub=JSON.parse(fs.readFileSync(c.public)),d=pub.diagrams.find(x=>x.id===c.diagram);
        assert(d,'actual admitted figure');
        const figure=page.locator('.admitted-diagram[data-semantic-fingerprint="'+d.semantic_fingerprint+'"]');
        const modes=await figure.locator('[data-native-scale]').count()?['auto','fit','read','full']:['inline'];
        for(const scale of modes) {
          if(scale!=='inline'){
            const input=figure.locator('[data-native-scale="'+scale+'"]');
            await input.focus();await input.press('Space');
            assert(await input.isChecked(),'native keyboard scale selection');
          }
          const start=figure.locator('[data-diagram-start]');
          const target=(await start.getAttribute('href')).slice(1);
          await start.click({noWaitAfter:true});
          const observed=await page.evaluate(id=>{
            const node=document.getElementById(id),svg=node.closest('svg');
            const painted=n=>{for(let p=n;p&&p!==svg.parentElement;p=p.parentElement){const s=getComputedStyle(p);if(s.display==='none'||s.visibility==='hidden')return false}const r=n.getBoundingClientRect();return n.textContent.trim()&&r.width>0&&r.height>0};
            const text=[...node.querySelectorAll('text')].find(painted);
            const label=text?.getBoundingClientRect();
            const headerBottom=Math.max(0,...[...document.querySelectorAll('.mobile-header,.document-nav')].map(n=>n.getBoundingClientRect()).filter(r=>r.width&&r.height&&r.top>=0).map(r=>r.bottom));
            return {target:id,label:label?.toJSON(),headerBottom,zoom:getComputedStyle(svg).zoom,pageY:scrollY,hash:location.hash};
          },target);
          assert(observed.label,'actual painted target wording');
          assert(observed.label.top>=observed.headerBottom-1,'native target wording clears sticky navigation');
          assert(observed.label.top<844,'located wording reaches the visible page');
          assert.equal(observed.hash,'#'+target,'native exact target fragment');
          observations.push({case:c.key,width,scale,...observed});
          await page.screenshot({path:path.join(config.output,c.key+'-'+width+'-'+scale+'.png')});
        }
      } finally {await context.close()}
    }
    fs.writeFileSync(path.join(config.output,'result.json'),JSON.stringify({status:'passed',observations,scope:'Script-free actual export location and sticky-clearance geometry only; Source omissions, semantics and Human experience are separate.'},null,2),{flag:'wx'});
    process.stdout.write(JSON.stringify({status:'passed',observations:observations.length})+'\n');
  } finally {await browser.close()}
})().catch(e=>{console.error(e);process.exitCode=1});
