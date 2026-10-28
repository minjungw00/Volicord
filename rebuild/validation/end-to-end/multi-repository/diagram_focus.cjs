// Run after preserving ordinary entry. Text-only guards and state ownership
// have no node/card border on which to show the map's usual focus cue.
async function inspectTextOriginFocus(figure) {
  const origins = figure.locator('a.diagram-element').filter({
    hasNot: figure.page().locator('.participant-box,.edge-key,.message-label')
  });
  const observations = [];
  for (const origin of await origins.all()) {
    await origin.focus();
    observations.push(await origin.evaluate(a => {
      const visible = [...a.querySelectorAll('text')].filter(t => {
        const r = t.getBoundingClientRect();
        return r.width > 0 && r.height > 0;
      });
      return {
        id: a.id,
        focused: document.activeElement === a,
        labels: visible.map(t => ({
          text: t.textContent,
          underline: getComputedStyle(t).textDecorationLine.includes('underline')
        }))
      };
    }));
  }
  return {
    observations,
    errors: observations.filter(o => !o.focused || !o.labels.length ||
      o.labels.some(t => !t.underline)).map(o => 'text origin has no visible keyboard focus: ' + o.id)
  };
}
module.exports = {inspectTextOriginFocus};

// A relationship's name and stroke must remain one reading target. Inspect
// native focus too: a client-only hover handler cannot repair the export.
async function inspectRelationFocus(figure, diagram) {
  const observations=[];
  for(const relation of diagram.meaning.relations||[]) {
    for(const id of [relation.id,...(relation.condition?[relation.id+':condition']:[])]) {
      const origin=figure.locator(`a.diagram-element[id="${await figure.getAttribute('id')}-${id}"]`);
      await figure.page().mouse.move(0,0);
      const previous=await figure.locator('a.diagram-route').evaluateAll(routes=>Object.fromEntries(routes.map(a=>[a.dataset.element,[...a.querySelectorAll('.relation-line')].map(p=>parseFloat(getComputedStyle(p).strokeWidth))])));
      await origin.focus();
      observations.push(await origin.evaluate((origin,{relationId,previous})=>{
        const route=[...origin.closest('svg').querySelectorAll('a.diagram-route')].find(a=>a.dataset.element===origin.closest('figure').id+'-'+relationId);
        const lines=[...route?.querySelectorAll('.relation-line')||[]].filter(p=>p.getBoundingClientRect().height>0||p.getBoundingClientRect().width>0);
        const unrelated=[...origin.closest('svg').querySelectorAll('a.diagram-route')].filter(a=>a!==route).filter(a=>[...a.querySelectorAll('.relation-line')].some((p,i)=>parseFloat(getComputedStyle(p).strokeWidth)>previous[a.dataset.element][i]));
        return {id:origin.id,focused:document.activeElement===origin,
          exactRoute:route?.dataset.element===origin.closest('figure').id+'-'+relationId,
          sameClaim:idWithoutCondition(origin,relationId,route),
          unrelatedEmphasis:unrelated.map(a=>a.dataset.element),
          widths:lines.map(p=>parseFloat(getComputedStyle(p).strokeWidth))};
        function idWithoutCondition(origin,id,route){
          const primary=document.getElementById(origin.closest('figure').id+'-'+id);
          return primary?.getAttribute('href')===route?.getAttribute('href');
        }
      },{relationId:relation.id,previous}));
    }
  }
  return {observations,errors:observations.filter(o=>!o.focused||!o.exactRoute||!o.sameClaim||o.unrelatedEmphasis.length||!o.widths.length||o.widths.some(w=>w<2)).map(o=>'relationship focus does not identify its exact route: '+o.id)};
}
module.exports.inspectRelationFocus=inspectRelationFocus;
