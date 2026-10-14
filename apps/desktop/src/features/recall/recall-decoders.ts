import type {
  RecallCitation,
  RecallCoverageInterval,
  RecallDay,
  RecallDayCard,
  RecallDayRecording,
  RecallEvidence,
  RecallFeedback,
  RecallJob,
  RecallProviderTestResult,
  RecallSearchResult,
  RecallSession,
  RecallStateChanged,
  RecallStatus,
  RecallSuggestion,
  RecallTask,
  LibraryDayCard,
  LibraryDaysPage,
} from '@daydream/shared';

function invalid(): never {
  throw new Error('Recall returned an invalid response. Reload to reconnect.');
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  return isRecord(value) ? value : invalid();
}
function string(value: unknown): string {
  return typeof value === 'string' ? value : invalid();
}
function boolean(value: unknown): boolean {
  return typeof value === 'boolean' ? value : invalid();
}
function number(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? value
    : invalid();
}
function count(value: unknown): number {
  const result = number(value);
  return Number.isSafeInteger(result) && result >= 0 ? result : invalid();
}
function nullable<T>(value: unknown, decode: (value: unknown) => T): T | null {
  return value === null ? null : decode(value);
}
function array<T>(value: unknown, decode: (value: unknown) => T): T[] {
  return Array.isArray(value) ? value.map(decode) : invalid();
}
function oneOf<const T extends string>(
  value: unknown,
  choices: readonly T[],
): T {
  for (const choice of choices) if (value === choice) return choice;
  return invalid();
}
function range(start: unknown, end: unknown): [number, number] {
  const a = count(start);
  const b = count(end);
  return b >= a ? [a, b] : invalid();
}
export function decodeRecallCitation(value: unknown): RecallCitation {
  const r = record(value);
  const [startOffsetMs, endOffsetMs] = range(r.startOffsetMs, r.endOffsetMs);
  const [absoluteStartMs, absoluteEndMs] = range(
    r.absoluteStartMs,
    r.absoluteEndMs,
  );
  return {
    evidenceId: string(r.evidenceId),
    recordingId: string(r.recordingId),
    segmentId: string(r.segmentId),
    sourceTrackId: string(r.sourceTrackId),
    sourceId: string(r.sourceId),
    sourceFrameRef: nullable(r.sourceFrameRef, string),
    documentId: nullable(r.documentId, string),
    sourceKind: oneOf(r.sourceKind, ['ocr', 'desktop-audio', 'microphone']),
    startOffsetMs,
    endOffsetMs,
    absoluteStartMs,
    absoluteEndMs,
    contentRevision: string(r.contentRevision),
  };
}
export function decodeRecallEvidence(value: unknown): RecallEvidence {
  const r = record(value);
  return { ...decodeRecallCitation(value), text: string(r.text) };
}
function decodeLibraryDayCard(value: unknown): LibraryDayCard {
  const r = record(value);
  return {
    privacyRevision: count(r.privacyRevision),
    dayId: string(r.dayId),
    recordingCount: count(r.recordingCount),
    evidenceCount: count(r.evidenceCount),
    reviewed: boolean(r.reviewed),
    status: oneOf(r.status, [
      'empty',
      'pending',
      'ready',
      'stale',
      'failed',
      'unconfigured',
      'generating',
      'partial',
      'invalidated',
      'insufficient',
    ]),
    capturedDurationMs: count(r.capturedDurationMs),
    thumbnailPath: nullable(r.thumbnailPath, string),
    thumbnailRecordingId: nullable(r.thumbnailRecordingId, string),
    redactionCount: count(r.redactionCount),
    recordingActive: boolean(r.recordingActive),
  };
}

export function decodeLibraryDaysPage(value: unknown): LibraryDaysPage {
  const r = record(value);
  const page = {
    privacyRevision: count(r.privacyRevision),
    days: array(r.days, decodeLibraryDayCard),
    nextBeforeDayId: nullable(r.nextBeforeDayId, string),
    generationReady: boolean(r.generationReady),
    generationUnavailableReason: nullable(
      r.generationUnavailableReason,
      string,
    ),
  };
  if (
    page.days.some((day) => day.privacyRevision !== page.privacyRevision) ||
    (page.nextBeforeDayId !== null &&
      page.nextBeforeDayId !== page.days.at(-1)?.dayId)
  )
    invalid();
  return page;
}

export function decodeRecallDayCard(value: unknown): RecallDayCard {
  const r = record(value);
  const [startMs, endMs] = range(r.startMs, r.endMs);
  return {
    ...decodeLibraryDayCard(value),
    startMs,
    endMs,
    timezone: string(r.timezone),
    summary: nullable(r.summary, string),
    inputFingerprint: string(r.inputFingerprint),
  };
}
function session(value: unknown): RecallSession {
  const r = record(value);
  const [startMs, endMs] = range(r.startMs, r.endMs);
  return {
    id: string(r.id),
    title: string(r.title),
    category: string(r.category),
    titleCorrection: nullable(r.titleCorrection, string),
    categoryCorrection: nullable(r.categoryCorrection, string),
    summary: string(r.summary),
    startMs,
    endMs,
    citations: array(r.citations, decodeRecallCitation),
    correction: nullable(r.correction, string),
  };
}
function suggestion(value: unknown): RecallSuggestion {
  const r = record(value);
  return {
    id: string(r.id),
    title: string(r.title),
    description: string(r.description),
    citations: array(r.citations, decodeRecallCitation),
    acceptedTaskId: nullable(r.acceptedTaskId, string),
    dismissed: boolean(r.dismissed),
  };
}
function feedback(value: unknown): RecallFeedback {
  const r = record(value);
  return {
    targetId: string(r.targetId),
    rating: nullable(r.rating, (v) => oneOf(v, ['helpful', 'unhelpful'])),
    comment: string(r.comment),
  };
}
function coverage(value: unknown): RecallCoverageInterval {
  const r = record(value);
  const [startMs, endMs] = range(r.startMs, r.endMs);
  return {
    sourceTrackIds: array(r.sourceTrackIds, string),
    recordingId: string(r.recordingId),
    segmentId: nullable(r.segmentId, string),
    startMs,
    endMs,
    kind: oneOf(r.kind, [
      'captured',
      'gap',
      'redacted',
      'missing',
      'uninterpreted',
    ]),
  };
}
export function decodeRecallTask(value: unknown): RecallTask {
  const r = record(value);
  return {
    id: string(r.id),
    dayId: string(r.dayId),
    title: string(r.title),
    notes: string(r.notes),
    status: oneOf(r.status, ['open', 'done', 'dismissed']),
    citations: array(r.citations, decodeRecallCitation),
    sourceWithheld: boolean(r.sourceWithheld),
    updatedAtMs: count(r.updatedAtMs),
  };
}
export function decodeRecallDay(value: unknown): RecallDay {
  const r = record(value);
  return {
    evidence:
      r.evidence === undefined ? [] : array(r.evidence, decodeRecallEvidence),
    privacyRevision: count(r.privacyRevision),
    day: decodeRecallDayCard(r.day),
    recordings:
      r.recordings === undefined ? [] : array(r.recordings, decodeDayRecording),
    sessions: array(r.sessions, session),
    suggestions: array(r.suggestions, suggestion),
    tasks: array(r.tasks, decodeRecallTask),
    feedback: array(r.feedback, feedback),
    coverage: array(r.coverage, coverage),
  };
}
function decodeDayRecording(value: unknown): RecallDayRecording {
  const r = record(value);
  return {
    recordingId: string(r.recordingId),
    title: nullable(r.title, string),
    startMs: count(r.startMs),
    endMs: nullable(r.endMs, count),
    startOffsetMs: nullable(r.startOffsetMs, count),
    durationMs: nullable(r.durationMs, count),
    availability: oneOf(r.availability, [
      'available',
      'partial',
      'unavailable',
      'hidden',
    ]),
    processingStatus: nullable(r.processingStatus, string),
  };
}
export function decodeRecallJob(value: unknown): RecallJob {
  const r = record(value);
  const progress = number(r.progress);
  if (progress < 0 || progress > 1) return invalid();
  return {
    id: string(r.id),
    dayId: nullable(r.dayId, string),
    kind: oneOf(r.kind, ['index', 'generation']),
    status: oneOf(r.status, [
      'pending',
      'running',
      'paused',
      'succeeded',
      'failed',
      'cancelled',
    ]),
    progress,
    errorCode: nullable(r.errorCode, string),
    updatedAtMs: count(r.updatedAtMs),
  };
}
export function decodeRecallStatus(value: unknown): RecallStatus {
  const r = record(value);
  return {
    privacyRevision: count(r.privacyRevision),
    indexedEvidenceCount: count(r.indexedEvidenceCount),
    pendingEvidenceCount: count(r.pendingEvidenceCount),
    embeddingReady: boolean(r.embeddingReady),
    generationReady: boolean(r.generationReady),
    embeddingUnavailableReason: nullable(r.embeddingUnavailableReason, string),
    generationUnavailableReason: nullable(
      r.generationUnavailableReason,
      string,
    ),
    jobs: array(r.jobs, decodeRecallJob),
  };
}
export function decodeRecallSearchResult(value: unknown): RecallSearchResult {
  const r = record(value);
  return {
    privacyRevision: count(r.privacyRevision),
    hits: array(r.hits, (v) => {
      const h = record(v);
      return {
        evidence: decodeRecallEvidence(h.evidence),
        score: number(h.score),
      };
    }),
    complete: boolean(r.complete),
    unavailableReason: nullable(r.unavailableReason, string),
    nextCursor: nullable(r.nextCursor, string),
    semanticUsed: boolean(r.semanticUsed),
    indexedEvidenceCount: count(r.indexedEvidenceCount),
    eligibleEvidenceCount: count(r.eligibleEvidenceCount),
  };
}
export function decodeRecallProviderTestResult(
  value: unknown,
): RecallProviderTestResult {
  const r = record(value);
  return {
    kind: oneOf(r.kind, ['embedding', 'generation']),
    status: oneOf(r.status, ['succeeded', 'failed']),
    errorCode: nullable(r.errorCode, string),
    elapsedMs: count(r.elapsedMs),
  };
}
export function decodeRecallStateChanged(value: unknown): RecallStateChanged {
  const r = record(value);
  return {
    dayId: nullable(r.dayId, string),
    privacyRevision: count(r.privacyRevision),
  };
}
export function decodeRecallDays(value: unknown): RecallDayCard[] {
  return array(value, decodeRecallDayCard);
}
export function decodeResolvedRecallEvidence(
  value: unknown,
): RecallEvidence | null {
  return nullable(value, decodeRecallEvidence);
}
