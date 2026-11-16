// Actual description navigation, separate from SVG activation and Source return.
const assert=require('node:assert/strict');
exports.inspect=async(page,figure)=>{
  const first=await figure.locator('.diagram-claim').evaluateAll(claims=>claims.find(n=>n.querySelector('[data-description-step="next"]'))?.dataset.element);
  if(!first)return {state:'not_applicable',reason:'No pair of related descriptions'};
  const element=page.locator('[id="'+first+'"]');
  await element.focus();await page.keyboard.press('Enter');
  await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
  const results=[];
  for(const direction of ['next','previous']){
    const control=figure.locator('.diagram-claim[data-active] [data-description-step="'+direction+'"]');
    await control.scrollIntoViewIfNeeded();await control.focus();
    await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(r))));
    const observe=()=>figure.evaluate(n=>{
      const v=n.querySelector('.diagram-viewport');
      return {x:scrollX,y:scrollY,left:v.scrollLeft,top:v.scrollTop,figureLeft:n.scrollLeft,figureTop:n.scrollTop,hash:location.hash,sourceOpen:document.body.dataset.sourceOpen};
    });
    const before=await observe(),href=await control.getAttribute('href');
    await page.keyboard.press('Enter');
    await page.waitForFunction(hash=>location.hash===hash,href);
    await page.evaluate(()=>new Promise(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>requestAnimationFrame(r)))));
    const after=await observe();
    assert.notEqual(after.hash,before.hash,'description really changed');
    for(const coordinate of ['x','y','left','top','figureLeft','figureTop'])assert(Math.abs(after[coordinate]-before[coordinate])<=1,'description step preserves '+coordinate+' '+JSON.stringify({direction,before,after}));
    assert.equal(after.sourceOpen,'false','description step opens no Source');
    const focused=await figure.evaluate(n=>{
      const active=document.activeElement;
      return n.querySelector('.diagram-claim[data-active]')?.contains(active)&&active.matches('[data-description-step]');
    });
    assert(focused,'keyboard continuation remains on an available description control');
    results.push({direction,before,after});
  }
  return {state:'passed',observations:results};
};
