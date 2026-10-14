import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';

// Only the speech provider is substituted. No speech boundaries or transcripts are seeded.
export async function checkLanguageContinuity({
  native,
  artifacts,
  navigate,
  click,
  viewer,
}) {
  const fixture = JSON.parse(
    await readFile(`${artifacts}/language-fixture.json`, 'utf8'),
  );
  const calls = [];
  let detections = 0;
  const windows = new Map();
  let rejectRepair = true;
  const server = createServer(async (request, response) => {
    if (request.method === 'GET' && request.url === '/v1/models') {
      response.setHeader('Content-Type', 'application/json');
      response.end(JSON.stringify({ data: [{ id: 'synthetic-speech' }] }));
      return;
    }
    assert.equal(request.url, '/v1/audio/transcriptions');
    const parts = [];
    for await (const bytes of request) parts.push(bytes);
    const body = Buffer.concat(parts);
    const language =
      body.toString('latin1').match(/name="language"\r\n\r\n([^\r]+)/)?.[1] ??
      'auto';
    let duration = 1;
    const riff = body.indexOf('RIFF');
    assert.ok(riff >= 0, 'the app submits extracted WAV audio');
    let pcm;
    let offset = riff + 12,
      bytesPerSecond = 32_000;
    while (offset + 8 < body.length) {
      const kind = body.toString('ascii', offset, offset + 4),
        size = body.readUInt32LE(offset + 4);
      if (kind === 'fmt ') bytesPerSecond = body.readUInt32LE(offset + 16);
      if (kind === 'data') {
        duration = size / bytesPerSecond;
        pcm = body.subarray(offset + 8, offset + 8 + size);
        break;
      }
      offset += 8 + size + (size % 2);
    }
    assert.ok(
      duration > 0 && duration < 60,
      'submitted speech windows are bounded',
    );
    const digest = createHash('sha256').update(pcm).digest('hex');
    if (language === 'auto' && !windows.has(digest))
      windows.set(digest, windows.size);
    const source = windows.get(digest);
    assert.ok(
      source !== undefined,
      'forced retries use an independently detected source',
    );
    if (language === 'auto') detections++;
    const detected = [
      'en',
      'fr',
      'de',
      'de',
      'de',
      'en',
      'de',
      'en',
      'en',
      'en',
      'de',
    ][source];
    assert.ok(detected, 'fixture receives only the expected speech windows');
    const weak = language === 'auto' && source === 0;
    const text = weak
      ? 'Mm-hmm.'
      : language === 'auto'
        ? `Synthetic automatic ${detected} source ${source}.`
        : `Synthetic corrected ${language} source ${source}.`;
    const rejected = rejectRepair && language === 'de' && source === 1;
    calls.push({ language, duration, source, rejected });
    if (rejected) {
      response.writeHead(400, { 'Content-Type': 'application/json' });
      response.end(
        JSON.stringify({ error: { message: 'Synthetic provider failure' } }),
      );
      return;
    }
    response.setHeader('Content-Type', 'application/json');
    response.end(
      JSON.stringify({
        language: language === 'auto' ? detected : language,
        languageConfidence: 0.9,
        duration,
        text,
        task: 'transcribe',
        segments: [{ id: 0, start: 0, end: duration, text }],
        words: [{ word: text, start: 0, end: duration, probability: 0.9 }],
      }),
    );
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const original = (await native.invoke('getAppSettings')).settings;
  let secret;
  try {
    secret = await native.invoke('saveAiProviderSecret', {
      input: {
        role: 'transcription',
        plaintext: 'synthetic-local-fixture-only',
      },
    });
    const settings = structuredClone(original);
    Object.assign(settings.ai.transcription, {
      provider: 'openai-compatible',
      remoteBaseUrl: `http://127.0.0.1:${server.address().port}/v1`,
      remoteModel: 'synthetic-speech',
      remoteApiKeyRef: secret.secretRef,
      language: 'auto',
      silenceGateEnabled: false,
      wordTimestampsEnabled: true,
      mergeGapMs: 500,
      minSpeechMs: 250,
    });
    settings.ai.vision.provider = 'disabled';
    settings.ai.vision.observationsEnabled = false;
    settings.ai.ocr.enabled = false;
    settings.ai.processing.autoStartAfterRecording = false;
    for (const source of Object.values(settings.ai.activity))
      source.enabled = false;
    await native.invoke('saveAppSettings', {
      input: { settings, applyStorageCleanup: false },
    });
    const job = await native.invoke('startRecordingAiProcessing', {
      input: { recordingId: fixture.id },
    });
    async function finish() {
      let completed;
      for (let attempt = 0; attempt < 120; attempt++) {
        const result = await native.invoke('listProcessingJobs', {
          input: { recordingId: fixture.id },
        });
        completed = result.jobs.find((value) => value.id === job.id);
        if (
          completed &&
          ['succeeded', 'completed-with-errors', 'failed'].includes(
            completed.status,
          )
        )
          break;
        await delay(500);
      }
      return completed;
    }
    let completed = await finish();
    assert.equal(
      completed?.status,
      'completed-with-errors',
      'repair failure remains visible',
    );
    const interrupted = await native.invoke('getRecordingTranscript', {
      input: { recordingId: fixture.id },
    });
    assert.ok(
      interrupted.tracks
        .flatMap((track) => track.segments)
        .some((segment) => segment.text === 'Synthetic automatic fr source 1.'),
      'failed backward repair preserves published text',
    );
    assert.equal(
      calls.filter((call) => call.language === 'de' && call.source === 0)
        .length,
      1,
      'first repair succeeded before the failure',
    );
    rejectRepair = false;
    await native.invoke('retryProcessingJob', { input: { jobId: job.id } });
    completed = await finish();
    assert.equal(
      completed?.status,
      'succeeded',
      'actual VAD and source-time speech processing complete',
    );
    const transcript = await native.invoke('getRecordingTranscript', {
      input: { recordingId: fixture.id },
    });
    const segments = transcript.tracks.flatMap((track) => track.segments);
    assert.ok(
      detections === 11,
      'each window continues auto detection after the acknowledgement',
    );
    assert.ok(
      calls.some((call) => call.language === 'de'),
      'later German consensus triggers corrective inference',
    );
    assert.ok(
      segments.some(
        (segment) => segment.text === 'Synthetic corrected de source 1.',
      ),
      'retroactively corrected speech is published through public transcript reads',
    );
    assert.ok(
      !segments.some((segment) => segment.text === 'Mm-hmm.'),
      'the tentative misdetected acknowledgement is replaced',
    );
    assert.equal(
      calls.filter((call) => call.language === 'de' && call.source === 0)
        .length,
      1,
      'successful repairs are not retranscribed after recovery',
    );
    assert.ok(
      segments.some(
        (segment) => segment.text === 'Synthetic corrected de source 5.',
      ),
      'confident isolated English is repaired',
    );
    for (const source of [7, 8])
      assert.ok(
        segments.some(
          (segment) =>
            segment.text === `Synthetic corrected en source ${source}.`,
        ),
        'three independent English detections restore tentative outputs',
      );
    for (const segment of segments) {
      assert.ok(segment.words.length > 0, 'replacement publishes word timing');
      for (const word of segment.words)
        assert.ok(
          word.startOffsetMs >= segment.startOffsetMs &&
            word.endOffsetMs <= segment.endOffsetMs,
          'replacement words align to their own source timeline',
        );
    }
    await navigate('Library', '/library');
    await native.wait(
      `(() => {
      const day = [...document.querySelectorAll('a[aria-label^="Open day"]')].find(link => link.href.includes(${JSON.stringify(fixture.date)}));
      if (!day) return false;
      day.setAttribute('data-language-day', 'true'); return true;
    })()`,
      'synthetic language day is available',
    );
    await click('[data-language-day]');
    await native.wait(
      'document.querySelector("video")?.readyState >= 2',
      'synthetic playback decodes',
    );
    const repaired = segments.find(
      (segment) => segment.text === 'Synthetic corrected de source 0.',
    );
    assert.ok(repaired);
    const selector = '[data-language-replacement]';
    await native.wait(
      `(() => {
        const entry = [...document.querySelectorAll('button[data-inspector-entry]')].find(button => button.textContent.includes('Synthetic corrected de source 0.'));
        if (!entry) return false;
        entry.setAttribute('data-language-replacement', 'true'); return true;
      })()`,
      'Studio displays the replacement',
    );
    await native.evaluate(
      `document.querySelector(${JSON.stringify(selector)}).scrollIntoView({block:'nearest',inline:'nearest'}); true`,
    );
    await click(selector);
    // Native playback restarts a stream at the requested source offset; the
    // decoder's currentTime is relative to that stream, not the recording.
    const sourceTime = `(() => {
      const video = document.querySelector('video[aria-label="Recording video"]');
      if (!video || video.readyState < 2) return null;
      return Number(new URL(video.currentSrc || video.src).searchParams.get('startMs') ?? 0) + video.currentTime * 1000;
    })()`;
    await native.wait(
      `${sourceTime} !== null && Math.abs(${sourceTime} - ${repaired.startOffsetMs}) < 500`,
      'replacement seeks to its original audio source',
    );
    await click('button[aria-label="Play"]');
    await native.wait(
      `!document.querySelector('video[aria-label="Recording video"]').paused && ${sourceTime} > ${repaired.startOffsetMs + 500}`,
      'synthetic audio playback advances at replacement',
    );
    await click('button[aria-label="Pause"]');
    await native.wait(
      `document.querySelector('video[aria-label="Recording video"]').paused && document.querySelector('button[aria-label="Play"]') !== null`,
      'synthetic playback pauses after verifying alignment',
    );
    await viewer.screenshot({
      path: `${artifacts}/language-replacement-playback.png`,
    });
    await writeFile(
      `${artifacts}/language-processing.json`,
      JSON.stringify(
        {
          fixture,
          calls,
          segments: segments.length,
          status: completed.status,
          playback: {
            transcriptStartMs: repaired.startOffsetMs,
            sourceTimeMs: await native.evaluate(sourceTime),
          },
          provider: 'isolated local HTTP protocol fixture',
          liveModelQuality: false,
        },
        null,
        2,
      ),
    );
  } finally {
    await native.invoke('saveAppSettings', {
      input: { settings: original, applyStorageCleanup: false },
    });
    if (
      secret &&
      secret.secretRef !== original.ai.transcription.remoteApiKeyRef
    )
      await native.invoke('deleteAiProviderSecret', {
        input: { secretRef: secret.secretRef },
      });
    await new Promise((resolve) => server.close(resolve));
  }
}
