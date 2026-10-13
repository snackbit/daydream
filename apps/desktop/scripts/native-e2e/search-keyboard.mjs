import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';

export async function checkSearchKeyboard({
  native,
  viewer,
  keyboard,
  navigate,
  click,
  artifacts,
  phase,
}) {
  await viewer.evaluate(() => {
    window.__broadwayE2eKeys = [];
    for (const type of ['keydown', 'keypress', 'keyup'])
      document.addEventListener(
        type,
        (event) => {
          const entry = {
            type: event.type,
            key: event.key,
            code: event.code,
            keyCode: event.keyCode,
            charCode: event.charCode,
            ctrl: event.ctrlKey,
            alt: event.altKey,
            special: window.getKeysymSpecial?.(event),
          };
          window.__broadwayE2eKeys.push(entry);
          queueMicrotask(() => (entry.prevented = event.defaultPrevented));
        },
        true,
      );
  });
  await navigate('Search', '/search');
  await native.wait(
    'document.querySelector("main input") !== null',
    'search ready',
  );
  await click('#recall-query');
  await native.wait(
    'document.activeElement?.id === "recall-query"',
    'native search input focused',
  );
  await native.evaluate(`(() => {
      window.__nativeE2eKeys = [];
      for (const type of ['keydown','keypress','keyup','input']) {
        document.addEventListener(type, event => {
          if (event.target.id === 'recall-query') {
            const entry = {type:event.type,key:event.key,value:event.target.value,trusted:event.isTrusted,ctrl:event.ctrlKey,alt:event.altKey,shift:event.shiftKey,meta:event.metaKey,composing:event.isComposing};
            window.__nativeE2eKeys.push(entry);
            queueMicrotask(() => entry.prevented = event.defaultPrevented);
          }
        }, true);
      }
      return true;
    })()`);
  await keyboard.type('synthetic unmatched query');
  await native.wait(
    `document.querySelector('#recall-query').value === 'synthetic unmatched query'`,
    'native WebDriver text committed',
  );
  assert.equal(
    await native.evaluate(
      "window.__nativeE2eKeys.filter(event => event.type === 'keydown').length",
    ),
    'synthetic unmatched query'.length,
  );
  assert.ok(
    await native.evaluate(
      'window.__nativeE2eKeys.every(event => event.trusted)',
    ),
    'native automation emits trusted keyboard events',
  );
  await writeFile(
    `${artifacts}/${phase}-keyboard-events.json`,
    await native.evaluate('JSON.stringify(window.__nativeE2eKeys, null, 2)'),
  );
  await writeFile(
    `${artifacts}/${phase}-viewer-keys.json`,
    JSON.stringify(
      await viewer.evaluate(() => window.__broadwayE2eKeys),
      null,
      2,
    ),
  );
  await keyboard.enter();
  await native.wait(
    'new URLSearchParams(location.search).get("q") === "synthetic unmatched query"',
    'typed query submitted',
  );
  await native.wait(
    `document.querySelector('section[aria-label="Search results"]') !== null && document.body.textContent.includes("No eligible matches")`,
    'real native empty search result',
  );
}
