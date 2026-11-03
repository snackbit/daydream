import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { setTimeout as delay } from 'node:timers/promises';

/** Prepared-path timing includes navigation and actual WebKit frame decoding. */
export async function checkPreparedStudioTiming({
  native,
  navigate,
  click,
  artifacts,
  recordingId,
  daySelector = 'a[href^="/day/"]',
  reportName = 'studio-prepared-opening-timings',
  preparationBudgetMs = 60_000,
  seekFractions = [],
}) {
  const preparationStarted = performance.now();
  process.stdout.write(
    `Studio readiness: waiting for saved media (${reportName})\n`,
  );
  let ready = false;
  while (
    !ready &&
    performance.now() - preparationStarted < preparationBudgetMs
  ) {
    const assets = await native.invoke('createLibraryPlaybackSession', {
      input: { recordingId, lazyThumbnails: true, includeRuns: false },
    });
    ready =
      assets.segments.length > 0 &&
      assets.segments.every((segment) => segment.canSeekInStream === true);
    if (assets.sessionId)
      await native.invoke('closeLibraryPlaybackSession', {
        input: { sessionId: assets.sessionId },
      });
    if (!ready) await delay(200);
  }
  assert.ok(ready, 'saved media must finish bounded background preparation');
  const preparationMs = performance.now() - preparationStarted;
  process.stdout.write(
    `Studio readiness: media prepared in ${Math.round(preparationMs)} ms; measuring decoded content\n`,
  );
  const samples = [];
  const stages = [];
  const diagnostic = process.env.DAYDREAM_E2E_DIAGNOSTIC === '1';
  for (let index = 0; index < (diagnostic ? 3 : 20); index++) {
    await navigate('Library', '/library');
    await native.wait(
      `document.querySelector(${JSON.stringify(daySelector)}) !== null`,
      'day available',
    );
    if (diagnostic && index === 0) {
      await native.command('Page.enable');
      await native.command('Timeline.enable');
      await native.command('Timeline.start', { maxCallStackDepth: 10 });
    }
    await native.evaluate(`(() => {
      const sample=window.__studioReadyMeasurement={trusted:false,ms:null,frameMs:null,thumbnailsMs:null};
      const link=document.querySelector(${JSON.stringify(daySelector)}),expected=new URL(link.href).pathname;
      link.addEventListener('click',event=>{
        if(!event.isTrusted)return;
        sample.trusted=true;const start=performance.now();
        function check(){
          const elapsed=performance.now()-start;if(elapsed>10000)return;
          const video=document.querySelector('video');
          const frame=location.pathname===expected&&video?.readyState>=2&&video.getBoundingClientRect().height>0&&video.currentSrc.includes('/file/');
          if(frame&&sample.frameMs===null)sample.frameMs=elapsed;
          const images=[...document.querySelectorAll('img[aria-label="Video timeline thumbnail"]')];
          const thumbnails=images.length>0&&images.every(image=>image.complete&&image.naturalWidth>0);
          if(thumbnails&&sample.thumbnailsMs===null)sample.thumbnailsMs=elapsed;
          const text=document.body.textContent;
          const content=!text.includes('Loading waveform')&&!text.includes('Preparing waveforms')&&!text.includes('Loading details');
          const populated=${seekFractions.length > 0}?document.querySelector('button[aria-label^="Zoom into "][aria-label$=" transcript entries"]')!==null:true;
          if(frame&&thumbnails&&content&&populated)requestAnimationFrame(()=>{sample.ms=performance.now()-start;});
          else requestAnimationFrame(check);
        }
        requestAnimationFrame(check);
      },{once:true,capture:true});return true;
    })()`);
    const started = performance.now();
    await click(daySelector);
    await native.wait(
      'window.__studioReadyMeasurement.ms !== null',
      'prepared frame and visible timeline content painted',
    );
    const driverElapsedMs = performance.now() - started;
    const measured = JSON.parse(
      await native.evaluate('JSON.stringify(window.__studioReadyMeasurement)'),
    );
    assert.ok(
      measured.trusted && Number.isFinite(measured.ms),
      'timing starts at trusted input',
    );
    samples.push(measured.ms);
    stages.push({ ...measured, driverElapsedMs });
    if (diagnostic && index === 0) {
      await native.command('Timeline.stop');
      await writeFile(
        `${artifacts}/${reportName}-profile.json`,
        JSON.stringify(
          native.events.filter((event) => event.method.startsWith('Timeline.')),
        ),
      );
    }
    process.stdout.write(
      `Studio readiness: opening ${index + 1}: ${Math.round(samples.at(-1))} ms\n`,
    );
  }
  const report = {
    decoderWarmed: await native.evaluate(
      'performance.getEntriesByName("studio.decoder.warm").length>0',
    ),
    metric:
      'trusted navigation click to painted prepared frame and visible timeline content; driver wall time recorded separately',
    preparationMs,
    acceptance: !diagnostic,
    stages,
    samples,
    maxMs: Math.max(...samples),
  };
  await writeFile(
    `${artifacts}/${reportName}.json`,
    JSON.stringify(report, null, 2),
  );
  if (diagnostic) return;
  const seeks = [];
  for (const fractionX of seekFractions) {
    const previous = await native.evaluate(
      'document.querySelector("video").currentSrc',
    );
    const started = performance.now();
    await click('[aria-label="Playback position"]', {
      fractionX,
      fractionY: 0.5,
    });
    await native.wait(
      `(() => { const video=document.querySelector('video'); return video?.readyState>=2 && !video.seeking && video.currentSrc.includes('/file/') && video.currentSrc!==${JSON.stringify(previous)}; })()`,
      'requested prepared segment decoded',
    );
    await native.wait(
      '!document.body.textContent.includes("Loading waveform") && !document.body.textContent.includes("Preparing waveforms") && !document.body.textContent.includes("Loading details") && [...document.querySelectorAll(\'img[aria-label="Video timeline thumbnail"]\')].every(image=>image.complete&&image.naturalWidth>0)',
      'seeked visible timeline content ready',
    );
    seeks.push({ fractionX, decodedMs: performance.now() - started });
  }
  if (seeks.length) {
    await writeFile(
      `${artifacts}/${reportName}-seeks.json`,
      JSON.stringify(seeks, null, 2),
    );
  }
  // Collect both measurements before enforcing their unchanged budgets.
  assert.ok(
    samples.every((ms) => ms < 1000),
    `every prepared opening must finish below 1 second: ${report.maxMs} ms`,
  );
  assert.ok(
    seeks.every((sample) => sample.decodedMs < 1000),
    'prepared seeks decode below one second',
  );
}
