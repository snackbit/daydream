import { expect, test } from 'vitest';
import { modelTransferError } from './model-transfer-format';

test('model failures expose actionable summaries without SQL, provider payloads or raw error text', () => {
  const privateError = new Error(
    'CHECK constraint failed: private provider payload',
  );
  expect(modelTransferError(privateError)).toBe(
    'The model action could not be completed. Your saved partial data can be retried.',
  );
  expect(
    modelTransferError({
      code: 'ai_storage_failed',
      message: 'CHECK constraint failed: provider=local-embedding',
    }),
  ).toBe(
    'The download could not be saved. Check available disk space, then retry.',
  );
  expect(
    modelTransferError({
      code: 'ai_model_checksum_failed',
      message: 'private server response',
    }),
  ).toBe(
    'The downloaded data could not be verified. Remove partial data, then retry.',
  );
});
