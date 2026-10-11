export async function checkStudioControls({ native, button, click }) {
  await button('Inspector');
  await native.wait(
    'document.querySelector("[role=tabpanel][data-state=active]").textContent.includes("Recording details")',
    'Inspector summary',
  );
  await button('Teleprompter');
  await native.wait(
    'document.querySelector("[role=tab][aria-selected=true]").textContent === "Teleprompter"',
    'Teleprompter selected',
  );
  await click('[aria-label="Playback speed"]');
  await native.wait(
    'document.querySelectorAll("[role=option]").length === 6',
    'six speed choices',
  );
  await native.evaluate(
    `(() => { const option = [...document.querySelectorAll('[role=option]')].find(element => element.textContent.trim() === '2x'); if (!option) throw new Error('Missing speed option'); option.setAttribute('data-native-e2e-speed','active'); return true; })()`,
  );
  await click('[data-native-e2e-speed="active"]');
  await native.wait(
    'document.querySelector("video").playbackRate === 2 && !document.querySelector("[role=listbox]")',
    'speed applies without replacing media',
  );
}
