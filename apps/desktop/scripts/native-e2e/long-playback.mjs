import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';

export const longPlayback = process.env.DAYDREAM_E2E_LONG_PLAYBACK === '1';

export async function checkLongPlayback({ native, click, detail, artifacts }) {
  assert.ok(
    detail.durationMs >= 102_000 && detail.durationMs < 110_000,
    `captured duration must match the long journey: ${detail.durationMs} ms`,
  );
  assert.ok(
    detail.segments.length >= 5,
    'four rollovers plus the resumed capture',
  );
  assert.ok(
    detail.segments.every(
      (segment) => segment.state === 'finalized' && segment.durationMs > 0,
    ),
    'every rollover must be durably finalized after Stop',
  );
  assert.equal(detail.playableSegmentCount, detail.segments.length);
  const duration = await native.evaluate(
    `document.querySelector('[aria-label="Playback position"]').getAttribute('aria-valuemax')`,
  );
  assert.ok(
    Number(duration) >= 102_000,
    'visible player covers the full recording',
  );
  const timings = [];
  for (const targetMs of [31_000, 61_000, 91_000]) {
    const pendingThumbnails = await native.evaluate(
      `Array.from(document.querySelectorAll('img[aria-label="Video timeline thumbnail"]')).filter(image => !image.complete).length`,
    );
    const started = performance.now();
    // Native Broadway clicks commit actual seekbar interaction. No component
    // state, currentTime or input values are assigned by the fixture.
    await click('[aria-label="Playback position"]', {
      fractionX: targetMs / Number(duration),
      fractionY: 0.5,
    });
    await native.wait(
      `Math.abs(Number(document.querySelector('[aria-label="Playback position"]').getAttribute('aria-valuenow')) - ${targetMs}) <= 1000`,
      'seek beyond storage boundary',
    );
    await click('button[aria-label="Play"]');
    await native.wait(
      '!document.querySelector("video").paused && document.querySelector("video").currentTime > 0.1',
      'late playback advances',
    );
    timings.push({
      targetMs,
      pendingThumbnails,
      playingMs: Math.round(performance.now() - started),
    });
    await click('button[aria-label="Pause"]');
    await native.wait(
      'document.querySelector("video").paused',
      'late playback paused',
    );
  }
  await writeFile(
    `${artifacts}/initial-seek-timings.json`,
    JSON.stringify(timings, null, 2),
  );
}
