// Owns figure controls, responsive geometry and expansion, never admitted meaning.
function createReaderFigures({lifetime,modal}) {
  const ko=document.documentElement.lang==='ko';
  const expandedReturns=new WeakMap(),disconnects=new Map();
  function closeExpansion(figure) {
    const prior = expandedReturns.get(figure);
    figure.classList.remove('diagram-expanded');
    modal.set('figure',null);
    const control = figure.querySelector('[data-diagram-action=expand]');
    control.setAttribute('aria-expanded','false');
    control.textContent = ko ? '확대·이동 보기' : 'Zoom and pan view';
    if (prior) {
      const viewport = figure.querySelector('.diagram-viewport');
      viewport.scrollLeft = prior.left; viewport.scrollTop = prior.top;
      control.focus({preventScroll:true}); window.scrollTo(0,prior.y);
    }
  }
  function sizeFigure(figure) {
    const svg=figure.querySelector('svg.connected-overview');if(!svg)return;
    const available=figure.querySelector('.diagram-viewport').clientWidth;
    const variant=available>=760?'wide':available>=520?'medium':'narrow';
    if(figure.dataset.variant===variant)return;
    figure.dataset.variant=variant;
    for(const name of ['width','height']){
      const size=parseFloat(svg.style.getPropertyValue(`--map-${variant}-${name}`));
      figure.dataset[name==='width'?'baseWidth':'baseHeight']=size;
      svg.style[name]=(size*(Number(figure.dataset.scale)||1))+'px';
    }
  }
  for (const figure of document.querySelectorAll('.admitted-diagram')) {
    if(!figure.querySelector('svg'))continue;
    figure.dataset.scale='1';figure.querySelector('.diagram-controls').hidden=false;
    disconnects.set(figure,lifetime.observe(figure.querySelector('.diagram-viewport'),()=>sizeFigure(figure)));
    sizeFigure(figure);
  }
  lifetime.on(document,'click', event => {
    const diagramControl=event.target.closest('button[data-diagram-action]');
    if (diagramControl) {
      const figure=diagramControl.closest('.admitted-diagram');const action=diagramControl.dataset.diagramAction;
      const svg=figure.querySelector('svg');
      if (action==='expand') {
        if (figure.classList.contains('diagram-expanded')) closeExpansion(figure);
        else {
          const viewport=figure.querySelector('.diagram-viewport');
          expandedReturns.set(figure,{y:scrollY,left:viewport.scrollLeft,top:viewport.scrollTop});
          figure.classList.add('diagram-expanded');
          diagramControl.setAttribute('aria-expanded','true');
          diagramControl.textContent=ko?'개요로 돌아가기 (Esc)':'Return to overview (Esc)';
          modal.set('figure',figure,()=>closeExpansion(figure));
          sizeFigure(figure);
        }
      } else {
        const old=Number(figure.dataset.scale);const next=action==='reset'?1:Math.min(2,Math.max(.75,old+(action==='in'?.25:-.25)));
        figure.dataset.scale=String(next);svg.style.width=(Number(figure.dataset.baseWidth)*next)+'px';svg.style.height=(Number(figure.dataset.baseHeight)*next)+'px';
        svg.querySelector('.diagram-content')?.setAttribute('transform',`scale(${next})`);
        figure.querySelector('.diagram-scale').textContent=Math.round(next*100)+'%';
        figure.querySelector('[data-diagram-action=in]').disabled=next===2;
        figure.querySelector('[data-diagram-action=out]').disabled=next===.75;
      }
    }
  });
  lifetime.cleanup(()=>{
    for(const figure of disconnects.keys()){
      figure.classList.remove('diagram-expanded');
      figure.querySelector('.diagram-controls').hidden=true;
      const svg=figure.querySelector('svg');
      svg.style.removeProperty('width');svg.style.removeProperty('height');
      svg.querySelector('.diagram-content')?.removeAttribute('transform');
    }
    disconnects.clear();
  });
  return {closeExpansion, forgetPublication(publication) {
    for (const [figure,disconnect] of disconnects) {
      if (figure.dataset.readerPublication!==publication) continue;
      disconnect(); disconnects.delete(figure); expandedReturns.delete(figure);
      if(figure.classList.contains('diagram-expanded'))modal.set('figure',null);
    }
  }};
}
