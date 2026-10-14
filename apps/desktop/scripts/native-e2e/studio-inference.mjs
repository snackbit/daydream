import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';

// Substitute only the configured provider. Extraction, admission, coordination,
// validation, retries, privacy checks and SQLite publication remain native.
export async function checkStudioInference({
  native,
  recordingId,
  artifacts,
  click,
  button,
  navigate,
  viewer,
  afterBaseline,
}) {
  const requests = [];
  let releaseProvider, signalRequest;
  const providerGate = new Promise((resolve) => {
    releaseProvider = resolve;
  });
  const firstRequest = new Promise((resolve) => {
    signalRequest = resolve;
  });
  let malformed = true;
  let holdDescriptions = false,
    releaseDescriptions,
    signalRecoveryRequest;
  const recoveryRequest = new Promise((resolve) => {
    signalRecoveryRequest = resolve;
  });
  const descriptionGate = new Promise((resolve) => {
    releaseDescriptions = resolve;
  });
  const server = createServer(async (request, response) => {
    let body = '';
    for await (const bytes of request) body += bytes;
    const input = JSON.parse(body),
      content = input.messages.at(-1).content;
    const fields = content[0].text;
    const images = content.filter((part) => part.type === 'image_url').length;
    const wantsOcr = fields.includes('ocr'),
      wantsObservation = fields.includes('observation');
    if (holdDescriptions) {
      signalRecoveryRequest();
      await descriptionGate;
    }
    requests.push({ images, ocr: wantsOcr, observation: wantsObservation });
    if (requests.length === 1) {
      signalRequest();
      await providerGate;
    }
    const output = {};
    if (wantsOcr) {
      const bad =
        malformed &&
        content.filter((part) => part.type === 'image_url').at(-1).image_url.url
          .length > 10_000;
      output.ocr = bad
        ? { blocks: [{ text: 'Synthetic', confidence: 1 }] }
        : { blocks: [{ text: 'Synthetic literal text' }] };
      if (bad) malformed = false;
    }
    if (wantsObservation)
      output.observation =
        images === 1
          ? { status: 'initial', text: 'Synthetic visible screen.' }
          : { status: 'unchanged', text: null };
    response.setHeader('Content-Type', 'application/json');
    response.end(
      JSON.stringify({
        choices: [
          {
            finish_reason: 'stop',
            message: { content: JSON.stringify(output) },
          },
        ],
      }),
    );
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const original = (await native.invoke('getAppSettings')).settings;
  process.stdout.write('Studio inference: settings read\n');
  try {
    const settings = structuredClone(original);
    settings.ai.modelRegistry = {
      version: 1,
      connections: [
        {
          id: 'studio-fixture',
          name: 'Local synthetic provider',
          provider: 'openai-compatible',
          authMode: 'none',
          baseUrl: `http://127.0.0.1:${server.address().port}/v1`,
          apiKeyRef: null,
          subscriptionId: null,
          protocol: 'chat-completions',
          tokenLimit: 'modern',
        },
      ],
      models: [
        {
          id: 'studio-vision',
          connectionId: 'studio-fixture',
          name: 'Synthetic vision',
          model: 'synthetic',
          capabilities: ['text', 'vision'],
          embeddingDimensions: null,
          embeddingConvention: 'plain',
        },
      ],
      bindings: { summary: null, embedding: null, vision: 'studio-vision' },
    };
    settings.ai.vision.observationsEnabled = true;
    settings.ai.vision.provider = 'openai-compatible';
    settings.ai.ocr = { enabled: true, engine: 'vision' };
    settings.ai.processing.frameRecognitionSyncWithRecording = false;
    settings.ai.processing.frameRecognitionFps = 0.2;
    settings.ai.processing.visualSamplingMode = 'full';
    settings.ai.processing.autoStartAfterRecording = false;
    for (const source of Object.values(settings.ai.activity))
      source.enabled = false;
    await native.invoke('saveAppSettings', {
      input: { settings, applyStorageCleanup: false },
    });
    process.stdout.write('Studio inference: synthetic provider configured\n');
    const job = await native.invoke('startRecordingAiProcessing', {
      input: { recordingId },
    });
    process.stdout.write('Studio inference: source job admitted\n');
    await Promise.race([
      firstRequest,
      delay(10_000).then(() => {
        throw new Error('Synthetic provider did not receive source work');
      }),
    ]);
    const overview = await native.invoke('listRecordingProcessingGroups', {
      input: { recordingId },
    });
    const screen = overview.groups[0].lanes;
    assert.ok(screen.find((lane) => lane.kind === 'ocr').running > 0);
    assert.ok(
      screen
        .find((lane) => lane.kind === 'ocr')
        .activity.some((work) => work.workerActive === true),
    );
    assert.ok(
      screen
        .find((lane) => lane.kind === 'vlm')
        .activity.some((work) => work.waitingReason === 'ocr'),
    );
    await navigate('Processing', '/processing');
    await native.wait(
      `document.querySelector('button[aria-label="Inspect OCR"]') !== null`,
      'processing pipeline',
    );
    await native.evaluate(
      `(() => { const row=[...document.querySelectorAll('li')].find(row=>row.getAttribute('aria-label')===${JSON.stringify(overview.groups[0].title)}); if(!row) throw new Error('Missing processing recording');row.setAttribute('data-native-processing','');return true; })()`,
    );
    await click('[data-native-processing] button[aria-label="Inspect OCR"]');
    await native.wait(
      `document.querySelector('[data-native-processing] [aria-label="Current and next work"]')?.textContent.includes('Recognizing screen text')`,
      'live OCR phase',
    );
    await viewer.screenshot({ path: `${artifacts}/processing-pipeline.png` });
    await click(`[data-native-processing] a[href^="/library/${recordingId}"]`);
    await native.wait(
      `document.querySelector('[data-timeline-processing]') !== null`,
      'continuous processing timeline',
    );
    assert.ok(
      await native.evaluate(
        `(() => {const overlays=[...document.querySelectorAll('[data-timeline-processing]')];return overlays.length>0&&overlays.every(overlay=>overlay.querySelectorAll('[data-processing-range]').length<=1);})()`,
      ),
      'screen processing is one continuous row per source',
    );
    await button('Processing overview');
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"] [aria-label="Processing pipeline"]') !== null`,
      'Studio processing sheet',
    );
    await native.wait(
      `(() => {const sheet=document.querySelector('[data-slot="sheet-content"]');return sheet&&getComputedStyle(sheet).opacity==='1'&&!sheet.getAnimations().some(animation=>animation.playState==='running');})()`,
      'processing sheet animation settles',
    );
    await viewer.screenshot({
      path: `${artifacts}/processing-studio-overview.png`,
    });
    await button('Close');
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"]') === null`,
      'processing sheet closes',
    );
    releaseProvider();
    let completed;
    for (let attempt = 0; attempt < 120; attempt++) {
      const jobs = await native.invoke('listProcessingJobs', {
        input: { recordingId },
      });
      completed = jobs.jobs.find((current) => current.id === job.id);
      if (
        completed &&
        ['succeeded', 'completed-with-errors', 'failed'].includes(
          completed.status,
        )
      )
        break;
      await delay(500);
    }
    assert.equal(
      completed?.status,
      'succeeded',
      'partial OCR validation recovers without losing valid comparison output',
    );
    await writeFile(
      `${artifacts}/studio-inference-requests.json`,
      JSON.stringify(requests, null, 2),
    );
    assert.equal(
      requests.length,
      13,
      'twelve frame/source pairs share inference, with one OCR-only retry',
    );
    assert.equal(
      requests.filter((request) => request.ocr && request.observation).length,
      7,
      'six changing frames share inference; the static source also reuses its identical-image OCR content',
    );
    assert.equal(
      requests.filter((request) => request.ocr && !request.observation).length,
      1,
    );
    assert.ok(
      requests.some((request) => request.images === 4),
      'current image plus three preceding images',
    );
    assert.ok(
      requests.every((request) => request.images >= 1 && request.images <= 4),
    );
    const visual = await native.invoke('getRecordingVisualObservations', {
      input: { recordingId, includeOcrRegions: false },
    });
    assert.equal(
      visual.vlmObservations.length,
      2,
      'unchanged frames extend existing observations without repeating cards',
    );
    assert.ok(
      visual.vlmObservations.every(
        (observation) => observation.confirmedEndMs === 25_001,
      ),
      'no observation claims the unprocessed final five seconds',
    );
    const ocr = await native.invoke('getOcrTimelineFrames', {
      input: { recordingId, limit: 100 },
    });
    assert.equal(
      ocr.frames.length,
      12,
      'both sources publish independently validated OCR',
    );
    await writeFile(
      `${artifacts}/studio-inference.json`,
      JSON.stringify(
        {
          requests,
          observations: visual.vlmObservations.length,
          ocrFrames: ocr.frames.length,
        },
        null,
        2,
      ),
    );
    // Review consumes the baseline provider's synthetic text. Run that journey
    // before local replacement of these text-free color/test-pattern screens.
    await afterBaseline();
    await navigate('Settings', '/settings');
    await native.wait(
      `document.querySelector('[aria-label="Enable debug tools"]') !== null`,
      'settings cache is loaded before local recovery',
    );
    await navigate('Processing', '/processing');
    holdDescriptions = true;
    // The baseline's exact OCR outputs are already cached. Change the saved
    // cadence so this scenario owns a genuinely new provider request to drain.
    const uncached = (await native.invoke('getAppSettings')).settings;
    uncached.ai.processing.frameRecognitionFps = 0.25;
    await native.invoke('saveAppSettings', {
      input: { settings: uncached, applyStorageCleanup: false },
    });
    const interrupted = await native.invoke('reprocessRecordingOcr', {
      input: { recordingId },
    });
    await Promise.race([
      recoveryRequest,
      delay(10_000).then(() => {
        throw new Error('Recovery fixture did not receive the old OCR request');
      }),
    ]);
    await navigate('Processing', '/processing');
    await native.wait(
      `[...document.querySelectorAll('button')].some(button => button.textContent.trim() === 'Use fast local processing' && !button.disabled)`,
      'local recovery control is available',
    );
    await button('Use fast local processing');
    let local;
    for (let attempt = 0; attempt < 120; attempt++) {
      local = (await native.invoke('getAppSettings')).settings;
      if (local.ai.ocr.engine === 'tesseract') break;
      await delay(500);
    }
    const retired = (
      await native.invoke('listProcessingJobs', { input: { recordingId } })
    ).jobs.find((job) => job.id === interrupted.id);
    assert.equal(
      retired.status,
      'completed-with-errors',
      'The old owner is stopped before local replacement admission',
    );
    assert.equal(local.ai.ocr.engine, 'tesseract');
    assert.equal(local.ai.processing.visualSamplingMode, 'adaptive-v1');
    assert.equal(local.ai.processing.resourceMode, 'fast');
    let localOverview;
    for (let attempt = 0; attempt < 120; attempt++) {
      localOverview = await native.invoke('listRecordingProcessingGroups', {
        input: { recordingId },
      });
      if (
        localOverview.groups[0]?.lanes.find((lane) => lane.kind === 'ocr')
          .status === 'succeeded'
      )
        break;
      await delay(500);
    }
    const localGroup = localOverview.groups[0];
    assert.equal(
      localGroup.lanes.find((lane) => lane.kind === 'ocr').status,
      'succeeded',
      'Local OCR completes while Vision descriptions are held',
    );
    assert.ok(
      localGroup.lanes.find((lane) => lane.kind === 'vlm').running > 0,
      'Descriptions remain a separate active lane',
    );
    await native.wait(
      `document.querySelector('button[aria-label="Inspect OCR"]')?.textContent.includes('Complete') && document.querySelector('button[aria-label="Inspect Screen observations"]')?.textContent.includes('frames processed')`,
      'visible OCR completion and independent description progress',
    );
    await writeFile(
      `${artifacts}/local-visual-recovery.json`,
      JSON.stringify(
        {
          ocrCompleted: true,
          descriptionsRunning: true,
          frameProgress: localGroup.lanes
            .find((lane) => lane.kind === 'vlm')
            .activity.map((work) => ({
              total: work.totalUnits,
              completed: work.completedUnits,
            })),
        },
        null,
        2,
      ),
    );
    await viewer.mouse.move(10, 10);
    await viewer.screenshot({ path: `${artifacts}/local-visual-recovery.png` });
    releaseDescriptions();
    for (let attempt = 0; attempt < 120; attempt++) {
      const page = await native.invoke('listRecordingProcessingGroups', {
        input: { recordingId },
      });
      if (page.groups[0]?.status === 'succeeded') break;
      await delay(500);
      if (attempt === 119)
        throw new Error('Local visual recovery did not complete');
    }
    await navigate('Settings', '/settings');
    await native.wait(
      `document.querySelector('[aria-label="Enable debug tools"]') !== null`,
      'settings reopen after local recovery',
    );
    const debugBefore = (await native.invoke('getAppSettings')).settings.debug
      .enabled;
    await click('[aria-label="Enable debug tools"]');
    let savedPreset;
    for (let attempt = 0; attempt < 100; attempt++) {
      savedPreset = (await native.invoke('getAppSettings')).settings;
      if (savedPreset.debug.enabled !== debugBefore) break;
      await delay(100);
    }
    assert.equal(
      savedPreset.debug.enabled,
      !debugBefore,
      'Unrelated preference is saved',
    );
    assert.equal(savedPreset.ai.ocr.engine, 'tesseract');
    assert.equal(savedPreset.ai.processing.resourceMode, 'fast');
    assert.equal(savedPreset.ai.processing.visualSamplingMode, 'adaptive-v1');
    await writeFile(
      `${artifacts}/local-visual-settings.json`,
      JSON.stringify({ presetRetainedAfterUnrelatedSave: true }),
    );
  } finally {
    releaseProvider();
    releaseDescriptions();
    await native.invoke('saveAppSettings', {
      input: { settings: original, applyStorageCleanup: false },
    });
    await new Promise((resolve) => server.close(resolve));
  }
}
