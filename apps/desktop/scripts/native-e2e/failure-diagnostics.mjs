import { writeFile } from 'node:fs/promises';

export async function writeNativeFailureDiagnostics({
  native,
  viewer,
  artifacts,
  phase,
}) {
  if (native) {
    const state = await native
      .evaluate(
        `JSON.stringify({
      pathname: location.pathname,
      viewport:{width:innerWidth,height:innerHeight},
      transcriptGroups:[...document.querySelectorAll('button[aria-label^="Zoom into "]')].filter(element=>element.getAttribute('aria-label').endsWith(' transcript entries')).slice(0,12).map(element=>{const rect=element.getBoundingClientRect(),hit=document.elementFromPoint(rect.x+rect.width/2,rect.y+rect.height/2);return {label:element.getAttribute('aria-label'),rect,hit:hit?.tagName,hitClass:hit?.className,contains:element.contains(hit)};}),
      dialogs: [...document.querySelectorAll('[role=dialog]')].map(element => ({ state: element.dataset.state, text: element.textContent })),
      statuses: [...document.querySelectorAll('[role=status]')].map(element => element.textContent),
      checkboxes: [...document.querySelectorAll('[role=checkbox]')].map(element => ({ id:element.id, checked:element.getAttribute('aria-checked') })),
      inputs: [...document.querySelectorAll('main input:not([type=password])')].map(element => ({ id:element.id, value:element.value })),
      keyboard: window.__nativeE2eKeys,
      pointer: window.__nativeE2ePointer,
      headings: [...document.querySelectorAll('main h1')].map(element => element.textContent)
    })`,
      )
      .catch((error) => JSON.stringify({ diagnosticFailure: error.message }));
    await writeFile(`${artifacts}/${phase}-failure-state.json`, state);
  }
  await viewer?.screenshot({ path: `${artifacts}/${phase}-failure.png` });
  if (viewer)
    await writeFile(
      `${artifacts}/${phase}-viewer-keys.json`,
      JSON.stringify(
        await viewer.evaluate(() => window.__broadwayE2eKeys),
        null,
        2,
      ) ?? 'null',
    );
}
