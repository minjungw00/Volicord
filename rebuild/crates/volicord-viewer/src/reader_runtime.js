// One enhancement lifetime and one modal authority over server-owned native HTML.
// Factories are local to the CSP-pinned bundle; they expose no browser globals.
function createReaderLifetime() {
  const controller = new AbortController(), cleanups = new Set(), frames = new Set();
  let disposed = false;
  return {
    get disposed() { return disposed; },
    on(target, type, listener, options = {}) {
      target.addEventListener(type, listener, {...options, signal:controller.signal});
    },
    cleanup(fn) { cleanups.add(fn); return () => cleanups.delete(fn); },
    observe(target, callback) {
      if (disposed) return () => {};
      const observer = new ResizeObserver(() => { if (!disposed && target.isConnected) callback(); });
      observer.observe(target);
      const disconnect = () => { observer.disconnect(); cleanups.delete(disconnect); };
      cleanups.add(disconnect); return disconnect;
    },
    frame(callback) {
      if (disposed) return;
      const id = requestAnimationFrame(() => { frames.delete(id); if (!disposed) callback(); });
      frames.add(id); return id;
    },
    dispose() {
      if (disposed) return;
      disposed = true; controller.abort();
      for (const id of frames) cancelAnimationFrame(id);
      frames.clear();
      for (const cleanup of [...cleanups]) cleanup();
      cleanups.clear();
    }
  };
}

function createReaderModal(lifetime) {
  const owners = new Map(), siblings = new Map();
  const priority = {drawer:1,figure:2,evidence:3};
  let current = null;
  function sync() {
    for (const [node, inert] of siblings) if (node.isConnected) node.inert = inert;
    siblings.clear();
    for (const {root} of owners.values()) { root.removeAttribute('role'); root.removeAttribute('aria-modal'); }
    current = [...owners].filter(([,entry])=>entry.root.isConnected)
      .sort(([a],[b])=>priority[b]-priority[a])[0]?.[1] || null;
    if (!current) return;
    const root = current.root;
    root.setAttribute('role','dialog'); root.setAttribute('aria-modal','true');
    for (let child=root; child?.parentElement; child=child.parentElement) {
      for (const sibling of child.parentElement.children) {
        if (sibling===child || sibling.matches('script,.evidence-backdrop,.nav-backdrop')) continue;
        siblings.set(sibling,sibling.inert); sibling.inert=true;
      }
      if (child.parentElement===document.body) break;
    }
  }
  const modal = {
    set(owner, root, dismiss) {
      const previous=owners.get(owner);
      if (previous?.root===root && previous?.dismiss===dismiss) return;
      if (previous) { previous.root.removeAttribute('role'); previous.root.removeAttribute('aria-modal'); }
      if (root) owners.set(owner,{root,dismiss}); else owners.delete(owner);
      sync();
    },
    clear() { for (const owner of [...owners.keys()]) modal.set(owner,null); }
  };
  lifetime.on(document,'keydown',event=>{
    if (!current || event.defaultPrevented) return;
    if (event.key==='Escape') { event.preventDefault(); current.dismiss?.(); return; }
    if (event.key!=='Tab') return;
    const root=current.root;
    const controls=[...root.querySelectorAll('a[href],button:not(:disabled),summary,input,select,[tabindex="0"]')]
      .filter(n=>n.getClientRects().length && !n.closest('details:not([open]) :not(summary)'));
    const first=controls[0],last=controls.at(-1),active=document.activeElement;
    if (!first) { event.preventDefault(); root.focus({preventScroll:true}); }
    else if (event.shiftKey && (!root.contains(active) || active===first || active===root)) { event.preventDefault(); last.focus(); }
    else if (!event.shiftKey && (!root.contains(active) || active===last || active===root)) { event.preventDefault(); first.focus(); }
  });
  lifetime.cleanup(modal.clear);
  return modal;
}

const readerLifetime=createReaderLifetime();
const readerModal=createReaderModal(readerLifetime);
