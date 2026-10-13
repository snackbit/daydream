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
  const server = createServer(async (request, response) => {
    let body = '';
    for await (const bytes of request) body += bytes;
    const input = JSON.parse(body),
      content = input.messages.at(-1).content;
    const fields = content[0].text;
    const images = content.filter((part) => part.type === 'image_url').length;
    const wantsOcr = fields.includes('ocr'),
      wantsObservation = fields.includes('observation');
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
  } finally {
    releaseProvider();
    await native.invoke('saveAppSettings', {
      input: { settings: original, applyStorageCleanup: false },
    });
    await new Promise((resolve) => server.close(resolve));
  }
}
