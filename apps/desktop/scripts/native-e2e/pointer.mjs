import assert from 'node:assert/strict';

// Observe real native events before releasing input. Chromium's event delivery
// to Broadway and GTK's delivery to WebKit are separate asynchronous boundaries.
export function createNativePointer({ native, viewer, origin }) {
  return async function click(selector) {
    const query = JSON.stringify(selector);
    await native.wait(
      `(() => { const element = document.querySelector(${query}); if (!element || element.matches(':disabled') || element.getAttribute('aria-disabled') === 'true') return false; const rect = element.getBoundingClientRect(); const style = getComputedStyle(element); return rect.width > 0 && rect.height > 0 && style.visibility === 'visible' && style.display !== 'none' && style.pointerEvents !== 'none'; })()`,
      `enabled control: ${selector}`,
    );
    await native.evaluate(
      `document.querySelector(${query}).scrollIntoView({block:'center',behavior:'instant'}); true`,
    );
    await native.wait(
      `(() => { const element = document.querySelector(${query}); const rect = element.getBoundingClientRect(); return rect.left >= 0 && rect.top >= 0 && rect.right <= innerWidth && rect.bottom <= innerHeight && element.contains(document.elementFromPoint(rect.x + rect.width/2, rect.y + rect.height/2)); })()`,
      `control receives pointer inside viewport: ${selector}`,
    );
    const point = JSON.parse(
      await native.evaluate(`JSON.stringify((() => {
      const element = document.querySelector(${query});
      const rect = element.getBoundingClientRect();
      window.__nativeE2ePointer = {selector:${query},selectTrigger:element.getAttribute('role') === 'combobox',selectOption:element.getAttribute('role') === 'option',events:[]};
      for (const type of ['pointerdown','pointerup','mousedown','mouseup','click']) document.addEventListener(type, event => {
        const target = event.target.closest('button,a,input');
        window.__nativeE2ePointer.events.push({type:event.type,expectedHit:element.contains(event.target),trusted:event.isTrusted,target:target?.getAttribute('aria-label') ?? target?.textContent,disabled:target?.disabled,pathname:location.pathname,x:event.clientX,y:event.clientY});
      }, {once:true,capture:true});
      return {x:rect.x+rect.width/2,y:rect.y+rect.height/2};
    })())`),
    );
    const surface = await viewer
      .locator('canvas')
      .nth(origin.index)
      .boundingBox();
    assert.ok(surface, 'Native Broadway surface must be visible');
    await viewer.mouse.move(
      surface.x + ((origin.x + point.x) * surface.width) / origin.width,
      surface.y + ((origin.y + point.y) * surface.height) / origin.height,
    );
    await viewer.mouse.down();
    try {
      await native.wait(
        `window.__nativeE2ePointer.events.some(event => ['pointerdown','mousedown'].includes(event.type) && event.expectedHit && event.trusted)`,
        `native pointer down: ${selector}`,
      );
    } finally {
      await viewer.mouse.up();
    }
    await native.wait(
      // Radix Select opens on pointerdown and prevents its compatibility mouse
      // event. The released trusted gesture plus the open native popup proves
      // the control was activated; ordinary buttons still require their click.
      `(() => { const observed = window.__nativeE2ePointer; if (observed.events.some(event => event.type === 'click' && event.expectedHit && event.trusted)) return true; const down = observed.events.some(event => event.type === 'pointerdown' && event.expectedHit && event.trusted); if (!down) return false; if (observed.selectTrigger) return document.querySelector(${query})?.getAttribute('aria-expanded') === 'true' && document.querySelector('[role=listbox]') !== null && observed.events.some(event => event.type === 'pointerup' && event.trusted); return observed.selectOption && document.querySelector('[role=listbox]') === null && observed.events.some(event => event.type === 'pointerup' && event.expectedHit && event.trusted); })()`,
      `native click: ${selector}`,
    );
  };
}
