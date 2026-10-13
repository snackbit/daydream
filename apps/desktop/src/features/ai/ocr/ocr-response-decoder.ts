import type {
  OcrBlock,
  OcrDocument,
  OcrDocumentSelection,
  OcrHistoryEntry,
  OcrHistoryPage,
  OcrWord,
  VisualRegion,
  ProcessingJob,
  OcrTimelineFramePage,
} from '@daydream/shared';

function invalid(): never {
  throw new Error('The native OCR response is invalid.');
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) return invalid();
  return value;
}
function string(value: unknown): string {
  return typeof value === 'string' ? value : invalid();
}
function number(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? value
    : invalid();
}
function offset(value: unknown): number {
  const result = number(value);
  return Number.isSafeInteger(result) && result >= 0 ? result : invalid();
}
function boolean(value: unknown): boolean {
  return typeof value === 'boolean' ? value : invalid();
}
function nullable<T>(value: unknown, decode: (value: unknown) => T): T | null {
  return value === null ? null : decode(value);
}
function array<T>(value: unknown, decode: (value: unknown) => T): T[] {
  return Array.isArray(value) ? value.map(decode) : invalid();
}
function confidence(value: unknown): number {
  const result = number(value);
  return result >= 0 && result <= 1 ? result : invalid();
}
function region(value: unknown): VisualRegion {
  const row = record(value);
  return {
    x: number(row.x),
    y: number(row.y),
    width: number(row.width),
    height: number(row.height),
  };
}
function word(value: unknown): OcrWord {
  const row = record(value);
  return {
    confidence: nullable(row.confidence, confidence),
    id: string(row.id),
    hierarchy: nullable(row.hierarchy, (value) => {
      const hierarchy = record(value);
      return {
        blockNum: offset(hierarchy.blockNum),
        lineNum: offset(hierarchy.lineNum),
        pageNum: offset(hierarchy.pageNum),
        paragraphNum: offset(hierarchy.paragraphNum),
        wordNum: offset(hierarchy.wordNum),
      };
    }),
    rawRegionId: nullable(row.rawRegionId, string),
    region: nullable(row.region, region),
    text: string(row.text),
  };
}
function block(value: unknown): OcrBlock {
  const row = record(value);
  return {
    confidence: nullable(row.confidence, confidence),
    id: string(row.id),
    order: offset(row.order),
    rawRegionIds: array(row.rawRegionIds, string),
    region: nullable(row.region, region),
    text: string(row.text),
    words: array(row.words, word),
  };
}
function historyEntry(value: unknown): OcrHistoryEntry {
  const row = record(value);
  const engine = row.engine;
  if (engine !== 'legacy' && engine !== 'tesseract' && engine !== 'vision')
    return invalid();
  const provenance = row.provenance;
  if (provenance !== 'native' && provenance !== 'reconstructed')
    return invalid();
  const status = row.status;
  if (status !== 'failed' && status !== 'gap' && status !== 'succeeded')
    return invalid();
  const observedStartMs = offset(row.observedStartMs);
  const observedEndMs = offset(row.observedEndMs);
  if (observedEndMs < observedStartMs) return invalid();
  return {
    cacheHit: boolean(row.cacheHit),
    configFingerprint: string(row.configFingerprint),
    engine,
    errorCode: nullable(row.errorCode, string),
    frameTimestampMs: offset(row.frameTimestampMs),
    height: nullable(row.height, offset),
    id: string(row.id),
    model: nullable(row.model, string),
    observedEndMs,
    observedStartMs,
    occurrenceCount: offset(row.occurrenceCount),
    provenance,
    provider: string(row.provider),
    recordingId: string(row.recordingId),
    segmentId: string(row.segmentId),
    sourceFrameRef: string(row.sourceFrameRef),
    sourceTrackId: string(row.sourceTrackId),
    status,
    supportTimestampsMs: array(row.supportTimestampsMs, offset),
    text: string(row.text),
    width: nullable(row.width, offset),
  };
}
function document(value: unknown): OcrDocument {
  const row = record(value);
  return {
    ...historyEntry(row),
    blocks: array(row.blocks, block),
    contentId: nullable(row.contentId, string),
  };
}
export function processingJob(value: unknown): ProcessingJob {
  const row = record(value);
  const kind = row.kind;
  if (
    kind !== 'activity' &&
    kind !== 'ai-processing' &&
    kind !== 'chunking' &&
    kind !== 'digest' &&
    kind !== 'embedding' &&
    kind !== 'frame-extraction' &&
    kind !== 'ocr' &&
    kind !== 'transcription' &&
    kind !== 'vlm' &&
    kind !== 'waveform'
  )
    return invalid();
  const status = row.status;
  if (
    status !== 'cancelled' &&
    status !== 'completed-with-errors' &&
    status !== 'failed' &&
    status !== 'paused' &&
    status !== 'queued' &&
    status !== 'running' &&
    status !== 'succeeded'
  )
    return invalid();
  return {
    createdAtMs: offset(row.createdAtMs),
    errorCode: nullable(row.errorCode, string),
    id: string(row.id),
    kind,
    progress: confidence(row.progress),
    recordingId: string(row.recordingId),
    status,
    updatedAtMs: offset(row.updatedAtMs),
  };
}
export function decodeOcrDocumentSelection(
  value: unknown,
): OcrDocumentSelection {
  const row = record(value);
  return {
    document: nullable(row.document, document),
    job: nullable(row.job, processingJob),
    recordingId: string(row.recordingId),
    sourceTrackId: string(row.sourceTrackId),
  };
}
export function decodeOcrHistoryPage(value: unknown): OcrHistoryPage {
  const row = record(value);
  return {
    items: array(row.items, historyEntry),
    nextCursor: nullable(row.nextCursor, (value) => {
      const cursor = record(value);
      return { id: string(cursor.id), timestampMs: offset(cursor.timestampMs) };
    }),
  };
}

export function decodeOcrTimelineFramePage(
  value: unknown,
): OcrTimelineFramePage {
  const row = record(value);
  return {
    recordingId: string(row.recordingId),
    frames: array(row.frames, (value) => {
      const frame = record(value);
      const status = frame.status;
      if (status !== 'failed' && status !== 'gap' && status !== 'succeeded')
        return invalid();
      const frameTimestampMs = offset(frame.frameTimestampMs);
      const sampleEndMs = offset(frame.sampleEndMs);
      if (sampleEndMs <= frameTimestampMs) return invalid();
      return {
        documentId: string(frame.documentId),
        errorCode: nullable(frame.errorCode, string),
        frameTimestampMs,
        sampleEndMs,
        id: string(frame.id),
        ocrOutputCount: offset(frame.ocrOutputCount),
        recordingId: string(frame.recordingId),
        segmentId: string(frame.segmentId),
        sourceFrameRef: string(frame.sourceFrameRef),
        sourceTrackId: string(frame.sourceTrackId),
        status,
        textEmpty: nullable(frame.textEmpty, boolean),
        textPreview:
          frame.textPreview === undefined
            ? null
            : nullable(frame.textPreview, string),
      };
    }),
    nextCursor: nullable(row.nextCursor, (value) => {
      const cursor = record(value);
      return { id: string(cursor.id), timestampMs: offset(cursor.timestampMs) };
    }),
  };
}
