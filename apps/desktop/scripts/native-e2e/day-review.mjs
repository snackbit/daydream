import assert from 'node:assert/strict';
import { writeFile } from 'node:fs/promises';
import { createServer } from 'node:http';
import { setTimeout as delay } from 'node:timers/promises';

// Only the synthetic HTTP provider is replaced. Native queue admission, workers,
// evidence eligibility, validation, publication and UI events remain real.
export async function checkDayReview({
  native,
  recordingId,
  artifacts,
  click,
  button,
  navigate,
  viewer,
}) {
  let release, received, retried;
  const gate = new Promise((resolve) => {
    release = resolve;
  });
  const first = new Promise((resolve) => {
    received = resolve;
  });
  const second = new Promise((resolve) => {
    retried = resolve;
  });
  let requests = 0;
  const summary = 'Synthetic screen activity was recorded.';
  const server = createServer(async (request, response) => {
    try {
      let body = '';
      for await (const bytes of request) body += bytes;
      const inputs = JSON.parse(JSON.parse(body).messages.at(-1).content);
      const evidence = inputs.find((input) => input.text.trim().length > 0);
      assert.ok(evidence, 'generation receives eligible native evidence');
      requests += 1;
      if (requests === 1) {
        received();
        await gate;
        response.writeHead(401, { 'Content-Type': 'application/json' });
        response.end(
          JSON.stringify({
            error: { message: 'Synthetic private provider body' },
          }),
        );
        return;
      }
      retried();
      const output = {
        summary: [
          {
            id: 'synthetic-summary',
            text: summary,
            citations: [
              {
                evidenceId: evidence.id,
                startMs: evidence.startMs,
                endMs: evidence.endMs,
                quote: evidence.text.slice(0, 2000),
              },
            ],
          },
        ],
        sessions: [],
        suggestions: [],
      };
      response.writeHead(200, { 'Content-Type': 'application/json' });
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
    } catch {
      response.writeHead(500);
      response.end('Synthetic fixture contract failed');
    }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const original = (await native.invoke('getAppSettings')).settings;
  try {
    const settings = structuredClone(original);
    settings.ai.modelRegistry = {
      version: 1,
      connections: [
        {
          id: 'review-fixture',
          name: 'Local synthetic summary provider',
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
          id: 'review-summary',
          connectionId: 'review-fixture',
          name: 'Synthetic summary',
          model: 'synthetic',
          capabilities: ['text'],
          embeddingDimensions: null,
          embeddingConvention: 'plain',
        },
      ],
      bindings: { summary: 'review-summary', embedding: null, vision: null },
    };
    settings.ai.recall.generation.enabled = true;
    settings.ai.recall.autoGenerate = false;
    await native.invoke('saveAppSettings', {
      input: { settings, applyStorageCleanup: false },
    });
    const recording = await native.invoke('getLibraryRecording', {
      recordingId,
    });
    const { dayId, timezone } = JSON.parse(
      await native.evaluate(
        `JSON.stringify((() => {const date=new Date(${recording.startedAtMs});return {dayId:[date.getFullYear(),String(date.getMonth()+1).padStart(2,'0'),String(date.getDate()).padStart(2,'0')].join('-'),timezone:Intl.DateTimeFormat().resolvedOptions().timeZone};})())`,
      ),
    );
    const input = { dayId, timezone };
    const day = await native.invoke('getRecallDay', { input });
    assert.ok(day.day.evidenceCount > 0);
    await navigate('Library', '/library');
    const dayLink = `a[href="/day/${dayId}"]`;
    await native.wait(
      `document.querySelector(${JSON.stringify(dayLink)}) !== null`,
      'day source link',
    );
    await click(dayLink);
    await native.wait(`location.pathname === '/day/${dayId}'`, 'day Studio');
    await native.wait(
      `Array.from(document.querySelectorAll('button')).some(button => button.textContent.trim() === 'Day review')`,
      'day Studio review control loaded',
    );
    await button('Day review');
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"]')?.textContent.includes('Generate summary')`,
      'generation action',
    );
    await button('Generate summary');
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"] [role="status"]')?.textContent.includes('queued')`,
      'visible queued summary',
    );
    await Promise.race([
      first,
      delay(30_000).then(() => {
        throw new Error('Native generation worker did not reach the fixture');
      }),
    ]);
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"] [role="status"]')?.textContent.includes('Generating')`,
      'visible running summary',
    );
    await viewer.screenshot({ path: `${artifacts}/day-review-running.png` });
    release();
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"] [role="alert"]')?.textContent.includes('authenticate')`,
      'persistent safe summary failure',
    );
    assert.equal(
      await native.evaluate(
        `document.body.textContent.includes('Synthetic private provider body')`,
      ),
      false,
    );
    await button('Generate summary');
    await Promise.race([
      second,
      delay(30_000).then(() => {
        throw new Error('Native generation retry did not reach the fixture');
      }),
    ]);
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"]')?.textContent.includes(${JSON.stringify(summary)})`,
      'published summary',
    );
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"] [role="status"]')?.textContent.includes('completed')`,
      'visible summary completion',
    );
    const saved = await native.invoke('getRecallDay', { input });
    assert.equal(saved.day.summary, summary);
    assert.equal(
      await native.evaluate(
        `document.querySelector('[data-slot="sheet-content"]').textContent.includes('Synthetic literal text')`,
      ),
      false,
    );
    await viewer.screenshot({ path: `${artifacts}/day-review-completed.png` });
    await click('[data-slot="sheet-content"] button:has(.lucide-x)');
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"]') === null`,
      'review closed',
    );
    await button('Day review');
    await native.wait(
      `document.querySelector('[data-slot="sheet-content"] [role="status"]')?.textContent.includes('completed')`,
      'persisted generation state after reopening',
    );
    await writeFile(
      `${artifacts}/day-review.json`,
      JSON.stringify(
        {
          requests,
          summaryPublished: true,
          persistedAfterReopen: true,
          rawOcrVisible: false,
        },
        null,
        2,
      ),
    );
  } finally {
    release();
    await native.invoke('saveAppSettings', {
      input: { settings: original, applyStorageCleanup: false },
    });
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
}
