import type { AppSettings } from '@daydream/shared';
import { expect, test } from 'vitest';
import { emptyRegistry } from '../models/model-registry';
import { defaultSettingsState } from '../testing/settings-browser-fixtures';
import { applySettingsEdit } from './settings-edit';

test('rebasing newer preferences preserves normalized credential references and concurrent connections', () => {
  const base: AppSettings = structuredClone(defaultSettingsState.settings);
  base.ai.modelRegistry = {
    ...emptyRegistry(),
    connections: [
      {
        id: 'first',
        name: 'First',
        provider: 'openai',
        authMode: 'api-key',
        baseUrl: 'https://api.openai.com/v1',
        protocol: 'responses',
        tokenLimit: 'modern',
        subscriptionId: null,
        apiKeyRef: null,
      },
    ],
  };
  const edited = {
    ...base,
    expert: { enabled: true },
    ai: {
      ...base.ai,
      transcription: { ...base.ai.transcription, language: 'de' },
    },
  };
  const normalized: AppSettings = {
    ...base,
    debug: { enabled: true },
    ai: {
      ...base.ai,
      transcription: {
        ...base.ai.transcription,
        remoteApiKeyRef: 'synthetic-encrypted-ref',
      },
      modelRegistry: {
        ...base.ai.modelRegistry,
        connections: [
          ...base.ai.modelRegistry.connections.map((connection) => ({
            ...connection,
            apiKeyRef: 'synthetic-encrypted-model-ref',
          })),
          {
            id: 'second',
            name: 'Second',
            provider: 'openai-compatible',
            authMode: 'none',
            baseUrl: 'https://example.test/v1',
            protocol: 'responses',
            tokenLimit: 'modern',
            subscriptionId: null,
            apiKeyRef: null,
          },
        ],
      },
    },
  };
  const next = applySettingsEdit(base, edited, normalized);
  expect(next).toMatchObject({
    expert: { enabled: true },
    debug: { enabled: true },
    ai: {
      transcription: {
        language: 'de',
        remoteApiKeyRef: 'synthetic-encrypted-ref',
      },
    },
  });
  expect(
    next.ai.modelRegistry?.connections.map((connection) => [
      connection.id,
      connection.apiKeyRef,
    ]),
  ).toEqual([
    ['first', 'synthetic-encrypted-model-ref'],
    ['second', null],
  ]);
});
