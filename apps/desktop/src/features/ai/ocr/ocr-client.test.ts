import { beforeEach, expect, test, vi } from 'vitest';
import {
  getOcrDocument,
  getOcrHistory,
  reprocessRecordingOcr,
} from './ocr-client';

const ipc = vi.hoisted(() => ({ invoke: vi.fn(), diagnostic: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (...args: unknown[]) =>
    args[0] === 'log_frontend_failure'
      ? ipc.diagnostic(...args)
      : ipc.invoke(...args),
}));
beforeEach(() => vi.resetAllMocks());
test('OCR queries use exact source/timestamp native commands and decode empty results', async () => {
  ipc.invoke
    .mockResolvedValueOnce({
      document: null,
      job: null,
      recordingId: 'recording-1',
      sourceTrackId: 'video-0',
    })
    .mockResolvedValueOnce({ items: [], nextCursor: null })
    .mockResolvedValueOnce(undefined);
  await expect(
    getOcrDocument({
      recordingId: 'recording-1',
      sourceTrackId: 'video-0',
      timestampMs: 500,
    }),
  ).resolves.toEqual({
    document: null,
    job: null,
    recordingId: 'recording-1',
    sourceTrackId: 'video-0',
  });
  await getOcrHistory({
    recordingId: 'recording-1',
    sourceTrackId: 'video-0',
    limit: 20,
    cursor: null,
  });
  await reprocessRecordingOcr('recording-1');
  expect(ipc.invoke).toHaveBeenNthCalledWith(1, 'getOcrDocument', {
    input: {
      recordingId: 'recording-1',
      sourceTrackId: 'video-0',
      timestampMs: 500,
    },
  });
  expect(ipc.invoke).toHaveBeenNthCalledWith(2, 'getOcrHistory', {
    input: {
      recordingId: 'recording-1',
      sourceTrackId: 'video-0',
      limit: 20,
      cursor: null,
    },
  });
  expect(ipc.invoke).toHaveBeenNthCalledWith(3, 'reprocessRecordingOcr', {
    input: { recordingId: 'recording-1' },
  });
});
test('malformed native OCR pages and another source are rejected instead of being displayed', async () => {
  ipc.invoke
    .mockResolvedValueOnce({ items: 'not a page', nextCursor: null })
    .mockResolvedValueOnce({
      document: null,
      job: null,
      recordingId: 'other-recording',
      sourceTrackId: 'video-0',
    });
  await expect(getOcrHistory({ recordingId: 'recording-1' })).rejects.toThrow(
    /invalid/,
  );
  await expect(
    getOcrDocument({
      recordingId: 'recording-1',
      sourceTrackId: 'video-0',
      timestampMs: 500,
    }),
  ).rejects.toThrow(/another source/);
});
test('native processing failures preserve the explicit setup error without retrying another engine', async () => {
  ipc.invoke.mockRejectedValue({
    code: 'ai_ocr_vision_unconfigured',
    message: 'Configure the Vision OCR provider.',
  });
  await expect(reprocessRecordingOcr('recording-1')).rejects.toThrow(
    'Configure the Vision OCR provider.',
  );
  expect(ipc.invoke).toHaveBeenCalledTimes(1);
  expect(ipc.diagnostic).toHaveBeenCalledWith('log_frontend_failure', {
    input: expect.objectContaining({
      operation: 'reprocessRecordingOcr',
      stage: 'invoke',
      errorCode: 'ai_ocr_vision_unconfigured',
    }),
  });
});
