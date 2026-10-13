import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';

// Only the speech provider is substituted. No speech boundaries or transcripts are seeded.
export async function checkLanguageContinuity({ native, artifacts }) {
  const fixture = JSON.parse(
    await readFile(`${artifacts}/language-fixture.json`, 'utf8'),
  );
  const calls = [];
  let detections = 0;
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
    let offset = riff + 12,
      bytesPerSecond = 32_000;
    while (offset + 8 < body.length) {
      const kind = body.toString('ascii', offset, offset + 4),
        size = body.readUInt32LE(offset + 4);
      if (kind === 'fmt ') bytesPerSecond = body.readUInt32LE(offset + 16);
      if (kind === 'data') {
        duration = size / bytesPerSecond;
        break;
      }
      offset += 8 + size + (size % 2);
    }
    assert.ok(
      duration > 0 && duration < 60,
      'submitted speech windows are bounded',
    );
    if (language === 'auto') detections++;
    const weak = language === 'auto' && detections === 1;
    const text = weak
      ? 'Mm-hmm.'
      : language === 'auto'
        ? 'Dies ist ein synthetischer deutscher Beispielsatz.'
        : 'Synthetischer korrigierter Text.';
    calls.push({ language, duration });
    response.setHeader('Content-Type', 'application/json');
    response.end(
      JSON.stringify({
        language: weak ? 'en' : 'de',
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
      detections >= 4,
      'each window continues auto detection after the acknowledgement',
    );
    assert.ok(
      calls.some((call) => call.language === 'de'),
      'later German consensus triggers corrective inference',
    );
    assert.ok(
      segments.some(
        (segment) => segment.text === 'Synthetischer korrigierter Text.',
      ),
      'retroactively corrected speech is published through public transcript reads',
    );
    assert.ok(
      !segments.some((segment) => segment.text === 'Mm-hmm.'),
      'the tentative misdetected acknowledgement is replaced',
    );
    await writeFile(
      `${artifacts}/language-processing.json`,
      JSON.stringify(
        {
          fixture,
          calls,
          segments: segments.length,
          status: completed.status,
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
