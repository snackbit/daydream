import type { RecordingTranscriptionProgress } from '@daydream/shared';
import {
  timelineProcessingRanges,
  type TimelineProcessingInput,
  type TimelineProcessingStatus,
} from './timeline-processing-ranges';

export interface CurtainInterval {
  startOffsetMs: number;
  endOffsetMs: number;
}
export interface TranscriptionCurtain {
  id: string;
  status: TimelineProcessingStatus;
  ranges: CurtainInterval[];
  pendingRanges: CurtainInterval[];
  publishedRanges: CurtainInterval[];
  startOffsetMs: number;
  endOffsetMs: number;
  revealOffsetMs: number;
  progress: number;
}

export function mergeCurtainIntervals(
  input: readonly CurtainInterval[],
): CurtainInterval[] {
  const merged: CurtainInterval[] = [];
  for (const range of input.toSorted(
    (a, b) => a.startOffsetMs - b.startOffsetMs,
  )) {
    if (range.endOffsetMs <= range.startOffsetMs) continue;
    const last = merged.at(-1);
    if (last && range.startOffsetMs <= last.endOffsetMs)
      last.endOffsetMs = Math.max(last.endOffsetMs, range.endOffsetMs);
    else merged.push({ ...range });
  }
  return merged;
}

export function transcriptionCurtain(
  input: TimelineProcessingInput,
): TranscriptionCurtain | null {
  if (
    input.track.kind === 'video' ||
    (input.state && input.state.recordingId !== input.detail.id)
  )
    return null;
  const native = input.state?.transcriptionProgress?.find(
    (source) =>
      source.jobId === input.state?.job?.id &&
      (source.trackId === input.track.id ||
        source.sourceRole === input.track.kind),
  );
  // Planned admission replaces the previous job immediately, before IPC catches up.
  const planned =
    input.plannedScopes?.filter(
      (scope) =>
        scope.trackId === input.track.id && scope.operation === 'transcription',
    ) ?? [];
  let requested: CurtainInterval[],
    published: CurtainInterval[],
    status: TimelineProcessingStatus;
  if (planned.length) {
    requested = planned;
    published = [];
    status = planned[0]?.status ?? 'planned';
  } else if (native) {
    requested = native.ranges;
    published = native.publishedRanges;
    status = curtainStatus(native.status);
  } else {
    const pending = timelineProcessingRanges(input).filter(
      (range) => range.operation === 'transcription',
    );
    const complete = (input.state?.chunks ?? []).flatMap((chunk) =>
      chunk.sourceKind === 'audio' &&
      (chunk.trackId === input.track.id ||
        chunk.trackId === input.track.kind) &&
      chunk.status === 'succeeded' &&
      chunk.startOffsetMs !== null &&
      chunk.endOffsetMs !== null
        ? [
            {
              startOffsetMs: chunk.startOffsetMs,
              endOffsetMs: chunk.endOffsetMs,
            },
          ]
        : [],
    );
    if (!pending.length && !complete.length) return null;
    requested = [...pending, ...complete];
    published = complete;
    status = combinedStatus(pending.map((range) => range.status));
  }
  const eligible = input.detail.segments
    .filter(
      (segment) =>
        segment.state === 'finalized' &&
        segment.fileExists &&
        (input.track.kind === 'microphone'
          ? segment.microphoneAudioStreamIndex !== null
          : segment.desktopAudioStreamIndex !== null),
    )
    .map((segment) => ({
      startOffsetMs: segment.playbackOffsetMs,
      endOffsetMs: segment.playbackOffsetMs + (segment.durationMs ?? 0),
    }));
  const redactions = input.detail.redactions.filter(
    (range) =>
      !range.trackIds.length || range.trackIds.includes(input.track.id),
  );
  const ranges = mergeCurtainIntervals(
    requested.flatMap((range) =>
      eligible.flatMap((source) =>
        subtract(
          [
            {
              startOffsetMs: Math.max(
                range.startOffsetMs,
                source.startOffsetMs,
              ),
              endOffsetMs: Math.min(range.endOffsetMs, source.endOffsetMs),
            },
          ],
          redactions,
        ),
      ),
    ),
  );
  const unresolved = subtract(ranges, mergeCurtainIntervals(published));
  const startOffsetMs = ranges[0]?.startOffsetMs,
    endOffsetMs = ranges.at(-1)?.endOffsetMs;
  if (startOffsetMs === undefined || endOffsetMs === undefined) return null;
  const revealOffsetMs = unresolved[0]?.startOffsetMs ?? endOffsetMs;
  // Out-of-order publication never exposes text beyond the first unresolved unit.
  const pendingRanges = ranges.flatMap((range) =>
    range.endOffsetMs > revealOffsetMs
      ? [
          {
            startOffsetMs: Math.max(range.startOffsetMs, revealOffsetMs),
            endOffsetMs: range.endOffsetMs,
          },
        ]
      : [],
  );
  const total = ranges.reduce(
    (sum, range) => sum + range.endOffsetMs - range.startOffsetMs,
    0,
  );
  const done = ranges.reduce(
    (sum, range) =>
      sum +
      Math.max(
        0,
        Math.min(range.endOffsetMs, revealOffsetMs) - range.startOffsetMs,
      ),
    0,
  );
  return {
    id: planned.length
      ? `admission:${input.detail.id}:${input.track.id}`
      : `${native?.jobId ?? input.state?.job?.id}:${input.track.id}`,
    status,
    ranges,
    pendingRanges,
    // The native publication view removes historical segments overlapping these
    // ranges. Fresh segments may retain a whole boundary word beyond the edge.
    publishedRanges:
      native && !planned.length ? mergeCurtainIntervals(published) : [],
    startOffsetMs,
    endOffsetMs,
    revealOffsetMs,
    progress: total ? done / total : 0,
  };
}

function subtract(
  ranges: readonly CurtainInterval[],
  cuts: readonly CurtainInterval[],
): CurtainInterval[] {
  let result = ranges
    .filter((range) => range.endOffsetMs > range.startOffsetMs)
    .map((range) => ({ ...range }));
  for (const cut of cuts)
    result = result.flatMap((range) => {
      if (
        cut.endOffsetMs <= range.startOffsetMs ||
        cut.startOffsetMs >= range.endOffsetMs
      )
        return [range];
      return [
        ...(cut.startOffsetMs > range.startOffsetMs
          ? [{ ...range, endOffsetMs: cut.startOffsetMs }]
          : []),
        ...(cut.endOffsetMs < range.endOffsetMs
          ? [{ ...range, startOffsetMs: cut.endOffsetMs }]
          : []),
      ];
    });
  return result;
}
function curtainStatus(
  status: RecordingTranscriptionProgress['status'],
): TimelineProcessingStatus {
  return status === 'completed-with-errors'
    ? 'failed'
    : status === 'succeeded'
      ? 'running'
      : status;
}
function combinedStatus(
  statuses: TimelineProcessingStatus[],
): TimelineProcessingStatus {
  const order: TimelineProcessingStatus[] = [
    'failed',
    'cancelled',
    'paused',
    'running',
    'queued',
    'planned',
  ];
  return order.find((status) => statuses.includes(status)) ?? 'running';
}
