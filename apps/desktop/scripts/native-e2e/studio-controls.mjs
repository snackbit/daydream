import { writeFile } from 'node:fs/promises';

export async function checkStudioControls({
  native,
  button,
  click,
  artifacts,
}) {
  await button('Details');
  await native.wait(
    'document.querySelector("[role=tabpanel][data-state=active]").textContent.includes("Recording details")',
    'Inspector summary',
  );
  await button('Review');
  await native.wait(
    'document.querySelector("[role=tab][aria-selected=true]").textContent === "Review"',
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
  // WebKit must change media progress, not just accept the playbackRate property.
  await click('button[aria-label="Play"]');
  await native.wait(
    '!document.querySelector("video").paused && document.querySelector("video").currentTime > 0.1',
    'native rate playback started',
  );
  const fast = await native.evaluate(
    '({time:document.querySelector("video").currentTime,wall:performance.now(),rate:document.querySelector("video").playbackRate,defaultRate:document.querySelector("video").defaultPlaybackRate,duration:document.querySelector("video").duration,ready:document.querySelector("video").readyState,buffered:[...Array(document.querySelector("video").buffered.length)].map((_,i)=>[document.querySelector("video").buffered.start(i),document.querySelector("video").buffered.end(i)])})',
  );
  await native.wait(
    `document.querySelector("video").currentTime > ${fast.time + 1.2}`,
    '2x media progress',
  );
  const after = await native.evaluate(
    '({time:document.querySelector("video").currentTime,wall:performance.now(),rate:document.querySelector("video").playbackRate,duration:document.querySelector("video").duration,ready:document.querySelector("video").readyState})',
  );
  const measured = (after.time - fast.time) / ((after.wall - fast.wall) / 1000);
  await writeFile(
    `${artifacts}/initial-speed.json`,
    JSON.stringify({ fast, after, measured }, null, 2),
  );
  if (measured < 1.6 || measured > 2.4)
    throw new Error(`Selected 2x playback advanced at ${measured.toFixed(2)}x`);
  await click('button[aria-label="Pause"]');
  await click('[aria-label="Playback speed"]');
  await native.evaluate(
    `(() => { const option=[...document.querySelectorAll('[role=option]')].find(element=>element.textContent.trim()==='1x'); if(!option)throw new Error('Missing normal speed'); option.setAttribute('data-native-e2e-normal-speed','active'); return true; })()`,
  );
  await click('[data-native-e2e-normal-speed="active"]');
}
