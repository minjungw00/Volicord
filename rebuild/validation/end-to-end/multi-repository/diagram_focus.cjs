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
