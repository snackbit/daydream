import type { ProcessingJob } from './processing.js';
import type { VisualRegion } from './visual.js';

export type OcrDocumentEngine = 'legacy' | 'tesseract' | 'vision';
export type OcrDocumentProvenance = 'native' | 'reconstructed';
export type OcrOccurrenceStatus = 'failed' | 'gap' | 'succeeded';

export interface OcrWordHierarchy {
  blockNum: number;
  lineNum: number;
  pageNum: number;
  paragraphNum: number;
  wordNum: number;
}

export interface OcrWord {
  confidence: number | null;
  id: string;
  hierarchy: OcrWordHierarchy | null;
  rawRegionId: string | null;
  region: VisualRegion | null;
  text: string;
}

export interface OcrBlock {
  confidence: number | null;
  id: string;
  order: number;
  rawRegionIds: string[];
  region: VisualRegion | null;
  text: string;
  words: OcrWord[];
}

export interface OcrHistoryEntry {
  cacheHit: boolean;
  configFingerprint: string;
  engine: OcrDocumentEngine;
  errorCode: string | null;
  frameTimestampMs: number;
  height: number | null;
  id: string;
  model: string | null;
  observedEndMs: number;
  observedStartMs: number;
  occurrenceCount: number;
  provenance: OcrDocumentProvenance;
  provider: string;
  recordingId: string;
  segmentId: string;
  sourceFrameRef: string;
  sourceTrackId: string;
  status: OcrOccurrenceStatus;
  supportTimestampsMs: number[];
  text: string;
  width: number | null;
}

export interface OcrDocument extends OcrHistoryEntry {
  blocks: OcrBlock[];
  contentId: string | null;
}

export interface GetOcrDocumentInput {
  documentId?: string | null;
  recordingId: string;
  sourceTrackId: string;
  timestampMs: number;
}

export interface OcrDocumentSelection {
  document: OcrDocument | null;
  job: ProcessingJob | null;
  recordingId: string;
  sourceTrackId: string;
}

export interface OcrHistoryCursor {
  id: string;
  timestampMs: number;
}

export interface GetOcrHistoryInput {
  cursor?: OcrHistoryCursor | null;
  limit?: number | null;
  recordingId: string;
  sourceTrackId?: string | null;
}

export interface OcrHistoryPage {
  items: OcrHistoryEntry[];
  nextCursor: OcrHistoryCursor | null;
}

export interface ReprocessRecordingOcrInput {
  recordingId: string;
}

export interface OcrTimelineFrame {
  documentId: string;
  errorCode: string | null;
  frameTimestampMs: number;
  id: string;
  ocrOutputCount: number;
  recordingId: string;
  segmentId: string;
  sourceFrameRef: string;
  sourceTrackId: string;
  status: OcrOccurrenceStatus;
  sampleEndMs: number;
  textEmpty: boolean | null;
}

export interface GetOcrTimelineFramesInput {
  cursor?: OcrHistoryCursor | null;
  endTimestampMs?: number | null;
  limit?: number | null;
  recordingId: string;
  sourceTrackId?: string | null;
  startTimestampMs?: number | null;
}

export interface OcrTimelineFramePage {
  frames: OcrTimelineFrame[];
  nextCursor: OcrHistoryCursor | null;
  recordingId: string;
}
