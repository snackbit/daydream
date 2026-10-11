import type {
  GetOcrDocumentInput,
  GetOcrHistoryInput,
  OcrDocumentSelection,
  OcrHistoryPage,
  GetOcrTimelineFramesInput,
  OcrTimelineFramePage,
} from '@daydream/shared';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import {
  decodeOcrDocumentSelection,
  decodeOcrHistoryPage,
  decodeOcrTimelineFramePage,
} from './ocr-response-decoder';

async function invokeOcr(command: string, input: object): Promise<unknown> {
  try {
    return await invoke<unknown>(command, { input });
  } catch (failure) {
    if (failure instanceof Error) throw failure;
    if (
      typeof failure === 'object' &&
      failure !== null &&
      'message' in failure &&
      typeof failure.message === 'string'
    )
      throw new Error(failure.message, { cause: failure });
    throw new Error('The native OCR command failed.', { cause: failure });
  }
}

export async function getOcrDocument(
  input: GetOcrDocumentInput,
): Promise<OcrDocumentSelection> {
  return decodeResponse(
    'getOcrDocument',
    await invokeOcr('getOcrDocument', input),
    (value) => {
      const result = decodeOcrDocumentSelection(value);
      if (
        result.recordingId !== input.recordingId ||
        result.sourceTrackId !== input.sourceTrackId ||
        (result.job && result.job.recordingId !== input.recordingId) ||
        (result.document &&
          (result.document.recordingId !== input.recordingId ||
            result.document.sourceTrackId !== input.sourceTrackId))
      )
        throw new Error('The OCR document belongs to another source.');
      return result;
    },
  );
}
export async function getOcrHistory(
  input: GetOcrHistoryInput,
): Promise<OcrHistoryPage> {
  return decodeResponse(
    'getOcrHistory',
    await invokeOcr('getOcrHistory', input),
    (value) => {
      const result = decodeOcrHistoryPage(value);
      if (
        result.items.some(
          (item) =>
            item.recordingId !== input.recordingId ||
            (input.sourceTrackId && item.sourceTrackId !== input.sourceTrackId),
        )
      )
        throw new Error('The OCR history belongs to another source.');
      return result;
    },
  );
}

export async function reprocessRecordingOcr(
  recordingId: string,
): Promise<void> {
  await invokeOcr('reprocessRecordingOcr', { recordingId });
}

export async function getOcrTimelineFrames(
  input: GetOcrTimelineFramesInput,
): Promise<OcrTimelineFramePage> {
  return decodeResponse(
    'getOcrTimelineFrames',
    await invokeOcr('getOcrTimelineFrames', input),
    (value) => {
      const page = decodeOcrTimelineFramePage(value);
      if (
        page.recordingId !== input.recordingId ||
        page.frames.some(
          (frame) =>
            frame.recordingId !== input.recordingId ||
            (input.sourceTrackId &&
              frame.sourceTrackId !== input.sourceTrackId),
        )
      )
        throw new Error('The OCR observation index belongs to another source.');
      return page;
    },
  );
}
