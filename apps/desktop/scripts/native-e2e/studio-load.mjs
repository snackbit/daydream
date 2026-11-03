import assert from 'node:assert/strict';
import { readFile, readdir, stat, writeFile } from 'node:fs/promises';
import { cpus, totalmem } from 'node:os';
import { join } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { checkPreparedStudioTiming } from './studio-ready-timing.mjs';
import { checkStudioTranscriptGroups } from './transcripts/studio-transcript-groups.mjs';

const percentile95 = (values) =>
  [...values].sort((a, b) => a - b)[Math.ceil(values.length * 0.95) - 1];
export async function checkStudioLoad({
  native,
  click,
  button,
  navigate,
  viewer,
  artifacts,
}) {
  const fixture = JSON.parse(
    await readFile(join(artifacts, 'studio-fixture.json'), 'utf8'),
  );
  await native.evaluate(`(() => {
    window.__studioPerf = {events:[],ipc:[],draws:0};
    const draw=CanvasRenderingContext2D.prototype.fillRect;
    CanvasRenderingContext2D.prototype.fillRect=function(...args) { window.__studioPerf.draws++; return draw.apply(this,args); };
    window.__studioPerf.media=[];
    if (${process.env.DAYDREAM_E2E_DIAGNOSTIC === '1'}) {
    const pause=HTMLMediaElement.prototype.pause;
    function trace(media,type){if(window.__studioPerf.media.length>100)return;window.__studioPerf.media.push({type,kind:media.tagName,time:media.currentTime,ready:media.readyState,paused:media.paused,buffered:Array.from({length:media.buffered.length},(_,i)=>[media.buffered.start(i),media.buffered.end(i)]),stack:type==='pause-call'?new Error().stack:undefined});}
    HTMLMediaElement.prototype.pause=function(){trace(this,'pause-call');return pause.call(this);};
    for(const type of ['waiting','canplay','pause'])document.addEventListener(type,event=>{if(event.target instanceof HTMLMediaElement)trace(event.target,type);},true);
    }
    return true;
  })()`);
  await navigate('Library', '/library');
  await native.wait(
    `document.querySelector('a[aria-label^="Open day"]') !== null || [...document.querySelectorAll('button')].some(button=>button.textContent.trim()==='Refresh eligible sources')`,
    'day list or explicit eligibility recovery',
  );
  if (
    await native.evaluate(
      `Array.from(document.querySelectorAll('button')).some(button=>button.textContent.trim()==='Refresh eligible sources')`,
    )
  )
    await button('Refresh eligible sources');
  await native.wait(
    `(() => { const link=[...document.querySelectorAll('a[aria-label^="Open day"]')].find(link=>link.closest('article')?.textContent.includes('8h 0m recorded')); if(!link)return false; link.setAttribute('data-studio-eight-hour-day','true'); return true; })()`,
    'synthetic eight-hour day listed',
  );
  const coldStart = performance.now();
  await click('[data-studio-eight-hour-day]');
  await native.wait(
    'document.querySelector(\'button[aria-label="Zoom in tracks"]\') !== null',
    'Studio controls usable',
  );
  const coldOpenMs = performance.now() - coldStart;
  await click('button[aria-label="Play"]');
  await native.wait(
    '!document.querySelector("video").paused && document.querySelector("video").currentTime>0.1',
    'eight-hour Studio playback advances',
  );
  process.stdout.write(
    'Studio load: playing synthetic media; sampling controls during waveform work\n',
  );
  assert.equal(
    await native.evaluate('document.body.textContent.includes("Segment 1")'),
    false,
    'storage boundaries never appear as clip labels',
  );
  await native.wait(
    `Array.from(document.querySelectorAll('button')).some(button=>/^Zoom into \\d+ transcript entries$/.test(button.getAttribute('aria-label')??''))`,
    'assembled Studio groups dense speech entries',
  );
  await native.wait(
    `Array.from(document.querySelectorAll('[data-lane-content][data-track-id="activity:mouse"] button')).some(button=>/^Zoom into \\d+ (mouse activity intervals|activity events)$/.test(button.getAttribute('aria-label')??''))`,
    'assembled Studio groups dense mouse intervals',
  );

  async function sample(selector, type = 'click') {
    await native.evaluate(`(() => {
      const element=document.querySelector(${JSON.stringify(selector)});
      if(!element)throw new Error('Missing performance control');
      element.addEventListener(${JSON.stringify(type)},event=>{
        const start=performance.now();
        requestAnimationFrame(()=>{const firstMs=performance.now()-start;requestAnimationFrame(()=>window.__studioPerf.events.push({ms:performance.now()-start,firstMs,trusted:event.isTrusted}));});
      },{once:true,capture:true}); return true;
    })()`);
    const before = await native.evaluate('window.__studioPerf.events.length');
    await click(selector);
    await native.wait(
      `window.__studioPerf.events.length>${before}`,
      'control paints after trusted input',
    );
  }
  const profile = process.env.DAYDREAM_E2E_PROFILE === '1';
  async function startProfile() {
    await native.command('Page.enable');
    await native.command('Timeline.enable');
    await native.command('Timeline.start', { maxCallStackDepth: 5 });
  }
  async function stopProfile() {
    await native.command('Timeline.stop');
    await writeFile(
      join(artifacts, 'studio-render-profile.json'),
      JSON.stringify(
        native.events.filter((event) => event.method.startsWith('Timeline.')),
        null,
        2,
      ),
    );
  }
  if (process.env.DAYDREAM_E2E_DIAGNOSTIC === '1') {
    if (profile) await startProfile();
    const layouts = [];
    for (let i = 0; i < 5; i++) {
      await sample(`button[aria-label="Zoom ${i % 2 ? 'in' : 'out'} tracks"]`);
      layouts.push(
        JSON.parse(
          await native.evaluate(
            `JSON.stringify((()=>{const content=document.querySelector('[aria-label="Track timeline"]'),viewport=content.closest('[data-slot="scroll-area-viewport"]');return {nodes:document.querySelectorAll('*').length,viewport:{width:viewport.clientWidth,height:viewport.clientHeight,scrollWidth:viewport.scrollWidth,scrollLeft:viewport.scrollLeft},content:{width:content.clientWidth,height:content.clientHeight},canvases:[...content.querySelectorAll('canvas')].map(c=>({width:c.width,height:c.height})),lanes:[...content.querySelectorAll('[data-lane-content]')].map(l=>({width:l.clientWidth,height:l.clientHeight})),panels:[...document.querySelectorAll('[data-slot="resizable-panel"]')].map(p=>({width:p.clientWidth,height:p.clientHeight}))};})())`,
          ),
        ),
      );
    }
    if (profile) await stopProfile();
    await writeFile(
      join(artifacts, 'studio-layout-diagnostic.json'),
      JSON.stringify(
        {
          acceptance: false,
          media: JSON.parse(
            await native.evaluate('JSON.stringify(window.__studioPerf.media)'),
          ),
          layouts,
          events: JSON.parse(
            await native.evaluate('JSON.stringify(window.__studioPerf.events)'),
          ),
        },
        null,
        2,
      ),
    );
    return 'diagnostic';
  }
  await native.evaluate(
    `document.querySelectorAll('[role=tab]').forEach(tab=>tab.setAttribute('data-perf-tab',tab.textContent.trim())); true`,
  );
  for (let i = 0; i < 50; i++)
    await sample(
      `[role=tab][data-perf-tab="${i % 2 ? 'Review' : 'Details'}"]`,
      'mousedown',
    );
  process.stdout.write(
    'Studio load: 50 control interactions sampled; waiting for durable waveform preparation\n',
  );
  await button('Review');
  await native.invoke('plugin:event|emit', {
    event: 'ai-processing-changed',
    payload: { recordingId: fixture.recordings[0] },
  });
  await native.wait(
    'document.querySelector("[role=tab][aria-selected=true]").textContent === "Review"',
    'processing refresh retains Review',
  );

  // The real local background queue prepares every source. No peaks are seeded.
  const coldWaveStart = performance.now();
  let pending = [...fixture.recordings];
  while (pending.length && performance.now() - coldWaveStart < 1_200_000) {
    const remaining = [];
    for (const recordingId of pending) {
      const result = await native.invoke('getRecordingWaveformTiles', {
        input: {
          recordingId,
          startTimelineMs: 0,
          endTimelineMs: 7_200_000,
          viewportWidthPx: 1200,
        },
      });
      assert.equal(result.error, null, 'native waveform jobs must not fail');
      if (result.pending) remaining.push(recordingId);
    }
    pending = remaining;
    if (pending.length) await delay(2000);
  }
  assert.deepEqual(pending, [], 'all cold waveform work completes durably');
  const coldWaveMs = performance.now() - coldWaveStart;
  async function cacheFiles() {
    const files = [];
    for (const id of fixture.recordings) {
      const base = join(fixture.recordingsRoot, id, 'waveforms');
      for (const file of await readdir(base, { recursive: true })) {
        const path = join(base, file),
          info = await stat(path);
        if (info.isFile())
          files.push({ path, size: info.size, modified: info.mtimeMs });
      }
    }
    return files.sort((a, b) => a.path.localeCompare(b.path));
  }
  const beforeFiles = await cacheFiles();
  const mediaBefore = await native.evaluate(
    'Number(document.querySelector(\'[role="slider"][aria-label="Playback position"]\').getAttribute("aria-valuenow"))',
  );
  const drawsBefore = await native.evaluate('window.__studioPerf.draws');
  await writeFile(
    join(artifacts, 'studio-media-before-warm.json'),
    await native.evaluate(
      `JSON.stringify([...document.querySelectorAll('video,audio')].map(media=>({kind:media.tagName,time:media.currentTime,duration:media.duration,paused:media.paused,ended:media.ended,ready:media.readyState,error:media.error?.code})),null,2)`,
    ),
  );
  await native.wait(
    `Number(document.querySelector('[role="slider"][aria-label="Playback position"]').getAttribute('aria-valuenow'))>${mediaBefore + 1000}`,
    'playback advances after waveform preparation',
  );
  assert.equal(
    await native.evaluate('window.__studioPerf.draws'),
    drawsBefore,
    'media clock ticks cannot repaint source waveforms',
  );
  const controls = await native.evaluate(
    'JSON.stringify(window.__studioPerf.events)',
  );
  await native.evaluate('window.__studioPerf.events=[]; true');
  if (profile) await startProfile();
  for (let i = 0; i < 50; i++) {
    await sample(`button[aria-label="Zoom ${i % 2 ? 'in' : 'out'} tracks"]`);
    if (profile && i === 4) {
      await stopProfile();
    }
  }
  const zoom = JSON.parse(
    await native.evaluate('JSON.stringify(window.__studioPerf.events)'),
  );
  await writeFile(
    join(artifacts, 'studio-zoom-samples.json'),
    JSON.stringify(zoom, null, 2),
  );
  const pan = [];
  for (let i = 0; i < 50; i++) {
    await native.evaluate(`(() => {
      const timeline=document.querySelector('[aria-label="Track timeline"]').closest('[data-slot="scroll-area-viewport"]');
      const start=performance.now(); timeline.scrollTo({left:${i % 2 ? 0 : 1000},behavior:'instant'});
      requestAnimationFrame(()=>requestAnimationFrame(()=>window.__studioPerf.events.push({ms:performance.now()-start,pan:true}))); return true;
    })()`);
    await native.wait(
      `window.__studioPerf.events.filter(item=>item.pan).length>${i}`,
      'warm pan painted',
    );
  }
  pan.push(
    ...JSON.parse(
      await native.evaluate(
        'JSON.stringify(window.__studioPerf.events.filter(item=>item.pan))',
      ),
    ),
  );
  await click('button[aria-label="Pause"]');
  await button('Review');
  await native.invoke('plugin:event|emit', {
    event: 'ai-processing-changed',
    payload: { recordingId: fixture.recordings[0] },
  });
  await native.wait(
    'document.querySelector("[role=tab][aria-selected=true]").textContent === "Review"',
    'warm publication retains chosen tab',
  );
  assert.deepEqual(
    await cacheFiles(),
    beforeFiles,
    'warm zoom, pan and refresh never regenerate waveform files',
  );
  const control = JSON.parse(controls);
  assert.ok(
    control.every((item) => item.trusted) && zoom.every((item) => item.trusted),
    'controls use trusted native input',
  );
  const report = {
    fixture,
    hardware: {
      cpu: cpus()[0].model,
      logicalCpus: cpus().length,
      memoryBytes: totalmem(),
    },
    runtime: 'GTK Broadway / native WebKit, packaged debug build',
    metric:
      'input/scroll to second animation frame; IPC and cold preparation recorded separately',
    coldOpenMs,
    coldWaveMs,
    control: {
      samples: control.length,
      p95Ms: percentile95(control.map((item) => item.ms)),
    },
    zoom: {
      samples: zoom.length,
      p95Ms: percentile95(zoom.map((item) => item.ms)),
    },
    pan: {
      samples: pan.length,
      p95Ms: percentile95(pan.map((item) => item.ms)),
    },
    ipcMeasurement:
      'Unavailable: native invoke is read-only; no IPC count inferred.',
  };
  await writeFile(
    join(artifacts, 'studio-performance.json'),
    JSON.stringify(report, null, 2),
  );
  await checkPreparedStudioTiming({
    native,
    navigate,
    click,
    artifacts,
    recordingId: fixture.recordings[0],
    daySelector: `a[href="/day/${fixture.date}"]`,
    reportName: 'studio-eight-hour-opening-timings',
    preparationBudgetMs: 1_200_000,
    seekFractions: [0.03, 0.12, 0.22],
  });
  await checkStudioTranscriptGroups({ native, click, viewer, artifacts });
  for (const theme of ['Light', 'Dark']) {
    await click('[aria-label="Color appearance"]');
    await native.wait(
      'document.querySelector("[role=option]")!==null',
      'appearance choices',
    );
    await native.evaluate(
      `(() => {const choice=[...document.querySelectorAll('[role=option]')].find(e=>e.textContent.trim()===${JSON.stringify(theme)}); if(!choice)throw new Error('Missing appearance choice'); choice.setAttribute('data-studio-theme','true'); return true;})()`,
    );
    await click('[data-studio-theme]');
    await native.wait(
      `document.documentElement.dataset.theme===${JSON.stringify(theme.toLowerCase())}`,
      'native Studio appearance',
    );
    await native.wait(
      'document.querySelector("[role=option]")===null',
      'appearance popup fully closes',
    );
    await click('button[aria-label="Zoom out tracks"]');
    await native.evaluate(
      `document.querySelector('[aria-label="Track timeline"]').scrollIntoView({block:'end',behavior:'instant'}); true`,
    );
    await viewer.screenshot({
      path: join(artifacts, `studio-${theme.toLowerCase()}.png`),
    });
  }
  for (const name of ['control', 'zoom', 'pan'])
    assert.ok(
      report[name].p95Ms < 100,
      `${name} p95 must remain below 100 ms: ${report[name].p95Ms}`,
    );
}
