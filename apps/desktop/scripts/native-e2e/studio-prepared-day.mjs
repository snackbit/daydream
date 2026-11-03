import { readFile, writeFile } from 'node:fs/promises';
import { checkPreparedStudioTiming } from './studio-ready-timing.mjs';

/** Rerun prepared content checks without repeating cold cache generation. */
export async function checkPreparedStudioDay({
  native,
  navigate,
  click,
  artifacts,
}) {
  const fixture = JSON.parse(
    await readFile(`${artifacts}/studio-fixture.json`, 'utf8'),
  );
  if (process.env.DAYDREAM_E2E_DIAGNOSTIC === '1') {
    const warmup = await native.invoke('getPlaybackDecoderWarmupUrl');
    await native.evaluate(`(() => {
      const video=document.createElement('video'),state=window.__decoderProbe={status:'loading'};
      function finish(status){state.status=status;state.error=video.error?.code;state.width=video.videoWidth;video.removeAttribute('src');video.load();}
      video.muted=true;video.preload='auto';video.addEventListener('loadeddata',()=>finish('decoded'),{once:true});video.addEventListener('error',()=>finish('error'),{once:true});
      video.src=${JSON.stringify(warmup)};video.load();return true;
    })()`);
    await native.wait(
      'window.__decoderProbe.status!=="loading"',
      'native synthetic decoder probe completes',
    );
    await writeFile(
      `${artifacts}/studio-decoder-probe.json`,
      await native.evaluate('JSON.stringify(window.__decoderProbe)'),
    );
    const ranges = {
      recordingId: fixture.recordings[0],
      startOffsetMs: 0,
      endOffsetMs: 180_000,
      reviewStartOffsetMs: 0,
      reviewEndOffsetMs: 60_000,
      bucketMs: 15_000,
    };
    const reads = [];
    for (const [command, input] of [
      ['getLibraryDayTimeline', { dayId: fixture.date, timezone: 'UTC' }],
      [
        'createLibraryPlaybackSession',
        {
          recordingId: fixture.recordings[0],
          lazyThumbnails: true,
          includeRuns: false,
        },
      ],
      ['getRecordingAiProcessingState', ranges],
      ['getRecordingActivity', ranges],
      [
        'getRecordingVisualObservations',
        { ...ranges, includeOcrRegions: false },
      ],
      [
        'getRecordingWaveformTiles',
        {
          recordingId: fixture.recordings[0],
          startTimelineMs: 0,
          endTimelineMs: 180_000,
          viewportWidthPx: 1200,
        },
      ],
    ]) {
      const started = performance.now();
      const result = await native.invoke(command, { input });
      reads.push({
        command,
        ms: performance.now() - started,
        bytes: JSON.stringify(result).length,
      });
      if (command === 'createLibraryPlaybackSession')
        await native.invoke('closeLibraryPlaybackSession', {
          input: { sessionId: result.sessionId },
        });
    }
    await writeFile(
      `${artifacts}/studio-ready-read-diagnostic.json`,
      JSON.stringify(reads, null, 2),
    );
  }
  await checkPreparedStudioTiming({
    native,
    navigate,
    click,
    artifacts,
    recordingId: fixture.recordings[0],
    daySelector: `a[href="/day/${fixture.date}"]`,
    reportName: 'studio-eight-hour-opening-timings',
    seekFractions: [0.03, 0.12, 0.22],
  });
  return process.env.DAYDREAM_E2E_DIAGNOSTIC === '1'
    ? 'NATIVE-STUDIO-PREPARED-READINESS-DIAGNOSTIC'
    : 'NATIVE-STUDIO-PREPARED-READINESS';
}
