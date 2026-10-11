import assert from 'node:assert/strict';
import { createServer } from 'node:http';

export async function checkModelSettings({ native, button, checkbox, click }) {
  let requests = 0;
  const server = createServer(async (request, response) => {
    let body = '';
    for await (const chunk of request) body += chunk;
    requests++;
    response.setHeader('Content-Type', 'application/json');
    if (requests <= 2) {
      response.end('synthetic-private-malformed-provider-body');
      return;
    }
    const input = JSON.parse(body).input;
    const vision = input.some((message) =>
      message.content.some((part) => part.type === 'input_image'),
    );
    response.end(
      JSON.stringify({
        status: 'completed',
        output: [
          {
            type: 'message',
            content: [
              {
                type: 'output_text',
                text: vision ? '{"color":"red"}' : '{"ok":true}',
              },
            ],
          },
        ],
      }),
    );
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const baseUrl = `http://127.0.0.1:${server.address().port}/v1`;
  try {
    const fixture = (await native.invoke('getAppSettings')).settings;
    fixture.ai.modelRegistry = {
      version: 1,
      connections: [
        {
          id: 'synthetic-api',
          name: 'Synthetic API',
          provider: 'openai-compatible',
          authMode: 'none',
          baseUrl,
          apiKeyRef: null,
          subscriptionId: null,
          protocol: 'responses',
          tokenLimit: 'modern',
        },
      ],
      models: [
        {
          id: 'synthetic-text',
          connectionId: 'synthetic-api',
          name: 'Synthetic text',
          model: 'synthetic-text',
          capabilities: ['text'],
          embeddingDimensions: null,
          embeddingConvention: 'plain',
        },
      ],
      bindings: { summary: null, embedding: null, vision: null },
    };
    await native.invoke('saveAppSettings', {
      input: { settings: fixture, applyStorageCleanup: false },
    });
    await native.evaluate(
      'window.__nativeModelSettingsReload = true; location.reload(); true',
    );
    await native.wait(
      `!window.__nativeModelSettingsReload && document.querySelector('[aria-label="Enable expert mode"]') !== null`,
      'settings reload',
    );
    await button('Models & accounts');
    await native.wait(
      `document.querySelector('[aria-label="Manage Synthetic API"]') !== null`,
      'reusable model settings',
    );
    await click('[aria-label="Manage Synthetic API"]');
    await click('[aria-label="Edit Synthetic text"]');
    await click(await checkbox('Vision'));
    await button('Test model');
    await native.wait(
      `document.querySelector('[role="dialog"]')?.textContent.includes('Check Diagnostics') && !Array.from(document.querySelectorAll('button')).find(button => button.textContent === 'Test model')?.disabled`,
      'failed synthetic model test',
    );
    const bundle = await native.invoke('get_debug_log_bundle');
    const events = bundle.logTailLines.map((line) => JSON.parse(line));
    assert(
      events.some(
        (event) =>
          event.fields.message === 'model_json_invalid' &&
          event.fields.stage === 'http_json',
      ),
    );
    assert(
      events.some(
        (event) =>
          event.fields.message === 'frontend_operation_failed' &&
          event.fields.operation === 'test_configured_model',
      ),
    );
    assert(
      !bundle.logTailLines
        .join('\n')
        .includes('synthetic-private-malformed-provider-body'),
    );
    await button('Test model');
    await native.wait(
      `document.querySelector('[role="dialog"]')?.textContent.includes('vision: Synthetic test passed.')`,
      'successful model test retry',
    );
    await button('Done with model');
    await button('Done');
    await native.wait(
      'document.querySelector("[role=dialog]") === null',
      'model settings saved',
    );
    const settings = (await native.invoke('getAppSettings')).settings;
    assert.equal(settings.ai.modelRegistry.models.length, 1);
    assert.deepEqual(settings.ai.modelRegistry.models[0].capabilities, [
      'text',
      'vision',
    ]);
    assert.equal(settings.ai.recall.generation.enabled, false);
    assert.equal(settings.ai.recall.embedding.enabled, false);
    assert.equal(settings.ai.vision.provider, 'disabled');
    await button('General');
  } finally {
    server.closeAllConnections();
    await new Promise((resolve) => server.close(resolve));
  }
}
