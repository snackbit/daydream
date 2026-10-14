import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';

export async function checkLibraryDays({
  native,
  navigate,
  viewer,
  artifacts,
}) {
  const started = performance.now();
  await navigate('Library', '/library');
  await native.wait(
    'document.querySelector(\'a[aria-label^="Open day "]\') !== null',
    'Library day metadata',
  );
  const navigationToDaysMs = performance.now() - started;
  const metadataStarted = performance.now();
  const days = await native.invoke('listLibraryDays', {
    input: { beforeDayId: null, limit: 30, timezone: 'UTC' },
  });
  const metadataReadMs = performance.now() - metadataStarted;
  assert.equal(days.days.length, 1);
  assert.equal(days.days[0].recordingCount, 1);
  for (const field of ['evidence', 'summary', 'sessions', 'tasks']) {
    assert.equal(Object.hasOwn(days.days[0], field), false);
  }
  const timelineStarted = performance.now();
  const timeline = await native.invoke('getLibraryDayTimeline', {
    input: { dayId: days.days[0].dayId, timezone: 'UTC' },
  });
  const timelineReadMs = performance.now() - timelineStarted;
  assert.equal(timeline.recordings.length, 1);
  assert.ok(timeline.spans.length > 0);
  assert.equal(timeline.recordings[0].sizeBytes, null);
  await writeFile(
    `${artifacts}/initial-library-timings.json`,
    JSON.stringify(
      { navigationToDaysMs, metadataReadMs, timelineReadMs },
      null,
      2,
    ),
  );
  await viewer.screenshot({ path: `${artifacts}/initial-library-days.png` });
}
