import type {
  LibraryRecordingDetail,
  RecordingActivity,
  RecordingAiProcessingState,
  RecordingTranscriptSegment,
  RecordingTranscriptWord,
  VlmObservation,
} from '@daydream/shared';
import {
  ACTIVITY_FOCUS_LANE_ID,
  ACTIVITY_KEYBOARD_LANE_ID,
  ACTIVITY_MOUSE_LANE_ID,
} from '../../studio-activity-lanes';
import type { StudioTimeRangeSelection } from '../../studio-timeline-store';
import type { StudioVisualFrame } from '../../studio-visual-observations';
import { transcriptTextSpans } from './transcript-text-spans';

export interface InspectorFragment {
  text: string;
  emphasis: 'selected' | 'context';
}
export interface InspectorContentEntry {
  id: string;
  kind: 'speech' | 'focus' | 'observation' | 'screen-text' | 'activity';
  source: string;
  trackId: string;
  startMs: number;
  endMs: number;
  text: string;
  words: RecordingTranscriptWord[];
  fragments: InspectorFragment[];
  boundary: boolean;
  wordTimingUnavailable?: boolean;
  observationKind?: VlmObservation['observationKind'];
}
export interface InspectorContentIndex {
  entries: InspectorContentEntry[];
  speechBySource: Map<string, InspectorContentEntry[]>;
  focus: InspectorContentEntry[];
}
const overlaps = (start: number, end: number, from: number, to: number) =>
  start < to && end > from;

export function transcriptSelectionEntries(
  state: RecordingAiProcessingState | null,
  ranges: StudioTimeRangeSelection[],
  segmentId?: string,
): InspectorContentEntry[] {
  return selectInspectorEntries(transcriptEntries(state, segmentId), ranges);
}
function transcriptEntries(
  state: RecordingAiProcessingState | null,
  segmentId?: string,
): InspectorContentEntry[] {
  const entries = new Map<string, InspectorContentEntry>();
  function add(
    segment: RecordingTranscriptSegment,
    trackId: string,
    role: string,
  ) {
    if (segmentId !== undefined && segment.segmentId !== segmentId) return;
    const id = `speech:${role}:${segment.id}`;
    entries.set(id, {
      id,
      kind: 'speech',
      source: role === 'microphone' ? 'Microphone' : 'Desktop audio',
      trackId,
      startMs: segment.startOffsetMs,
      endMs: segment.endOffsetMs,
      text: segment.text,
      words: segment.words,
      fragments: [],
      boundary: false,
    });
  }
  for (const track of state?.transcriptionTracks ?? []) {
    if (track.libraryTrackId === null) continue;
    // The transcript is authoritative even when a scoped pass has no chunk for
    // another source. Chunk segments also support historical read models.
    const canonical =
      state?.transcript.tracks
        .filter((candidate) => candidate.sourceRole === track.sourceRole)
        .flatMap((candidate) => candidate.segments) ?? [];
    for (const segment of canonical)
      add(segment, track.libraryTrackId, track.sourceRole);
    for (const chunk of track.chunks)
      for (const segment of chunk.transcriptSegments)
        if (!entries.has(`speech:${track.sourceRole}:${segment.id}`))
          add(segment, track.libraryTrackId, track.sourceRole);
  }
  return [...entries.values()].sort(compareEntries);
}
export function buildInspectorContent({
  detail,
  state,
  activity,
  visualFrames,
}: {
  detail: LibraryRecordingDetail;
  state: RecordingAiProcessingState | null;
  activity: RecordingActivity | null;
  visualFrames: StudioVisualFrame[];
}): InspectorContentIndex {
  const entries = transcriptEntries(state);
  // Minimal legacy states may lack the transcription lane read model.
  if (!state?.transcriptionTracks?.length)
    for (const track of state?.transcript.tracks ?? []) {
      const source = detail.tracks.find(
        (candidate) => candidate.kind === track.sourceRole,
      );
      if (source)
        for (const segment of track.segments)
          entries.push({
            id: `speech:${track.sourceRole}:${segment.id}`,
            kind: 'speech',
            source: source.label,
            trackId: source.id,
            startMs: segment.startOffsetMs,
            endMs: segment.endOffsetMs,
            text: segment.text,
            words: segment.words,
            fragments: [],
            boundary: false,
          });
    }
  let previousFocus: string | null = null;
  for (const event of [...(activity?.events ?? [])].sort(
    (a, b) => a.offsetMs - b.offsetMs,
  )) {
    const text =
      [event.applicationName, event.title]
        .filter((value) => value !== null && value !== '')
        .join(' · ') || 'Window focus changed';
    if (text === previousFocus) continue;
    previousFocus = text;
    entries.push({
      id: `focus:${event.id}`,
      kind: 'focus',
      source: 'Window focus',
      trackId: ACTIVITY_FOCUS_LANE_ID,
      startMs: event.offsetMs,
      endMs: event.offsetMs + 1,
      text,
      words: [],
      fragments: [],
      boundary: false,
    });
  }
  for (const frame of visualFrames) {
    const trackId =
      frame.sourceTrackId ??
      detail.tracks.find((track) => track.kind === 'video')?.id;
    if (!trackId) continue;
    const source =
      detail.tracks.find((track) => track.id === trackId)?.label ?? 'Screen';
    for (const observation of frame.vlmObservations)
      entries.push({
        id: `observation:${observation.id}`,
        kind: 'observation',
        observationKind: observation.observationKind,
        source,
        trackId,
        startMs: frame.frameTimestampMs,
        endMs: frame.frameTimestampMs + 1,
        text: observation.text,
        words: [],
        fragments: [],
        boundary: false,
      });
    if (frame.ocrTextPreview)
      entries.push({
        id: `screen-text:${frame.id}`,
        kind: 'screen-text',
        source,
        trackId,
        startMs: frame.frameTimestampMs,
        endMs: frame.ocrSampleEndMs ?? frame.frameTimestampMs + 1,
        text: frame.ocrTextPreview,
        words: [],
        fragments: [],
        boundary: false,
      });
  }
  for (const interval of activity?.intervals ?? []) {
    const mouse = interval.sourceKind === 'mouse';
    if (!mouse && interval.sourceKind !== 'keyboard') continue;
    if (interval.state !== 'active') continue;
    entries.push({
      id: `activity:${interval.id}`,
      kind: 'activity',
      source: mouse ? 'Mouse' : 'Keyboard',
      trackId: mouse ? ACTIVITY_MOUSE_LANE_ID : ACTIVITY_KEYBOARD_LANE_ID,
      startMs: interval.startOffsetMs,
      endMs: interval.endOffsetMs,
      text: mouse ? 'Mouse activity' : 'Keyboard activity',
      words: [],
      fragments: [],
      boundary: false,
    });
  }
  const visible = entries
    .filter(
      (entry) =>
        !detail.redactions.some(
          (range) =>
            (range.trackIds.length === 0 ||
              range.trackIds.includes(entry.trackId)) &&
            overlaps(
              entry.startMs,
              entry.endMs,
              range.startOffsetMs,
              range.endOffsetMs,
            ),
        ),
    )
    .sort(compareEntries);
  const speechBySource = new Map<string, InspectorContentEntry[]>();
  for (const entry of visible)
    if (entry.kind === 'speech') {
      const source = speechBySource.get(entry.trackId) ?? [];
      source.push(entry);
      speechBySource.set(entry.trackId, source);
    }
  return {
    entries: visible,
    speechBySource,
    focus: visible.filter((entry) => entry.kind === 'focus'),
  };
}
export function selectInspectorEntries(
  entries: InspectorContentEntry[],
  ranges: StudioTimeRangeSelection[],
): InspectorContentEntry[] {
  return entries.flatMap((entry) => {
    const selected = ranges.filter(
      (range) =>
        range.laneIds.includes(entry.trackId) &&
        overlaps(entry.startMs, entry.endMs, range.startMs, range.endMs),
    );
    if (selected.length === 0) return [];
    const boundary = !selected.some(
      (range) => range.startMs <= entry.startMs && range.endMs >= entry.endMs,
    );
    const spans =
      entry.kind === 'speech'
        ? transcriptTextSpans(entry.text, entry.words)
        : [];
    const wordTimingUnavailable = !spans.some((span) => span.word !== null);
    const fragments = !wordTimingUnavailable
      ? spans.map(({ text, word }) => ({
          text,
          emphasis:
            word &&
            selected.some((range) =>
              overlaps(
                word.startOffsetMs,
                word.endOffsetMs,
                range.startMs,
                range.endMs,
              ),
            )
              ? ('selected' as const)
              : ('context' as const),
        }))
      : [
          {
            text: entry.text,
            emphasis: boundary ? ('context' as const) : ('selected' as const),
          },
        ];
    return [{ ...entry, boundary, fragments, wordTimingUnavailable }];
  });
}
function compareEntries(
  a: InspectorContentEntry,
  b: InspectorContentEntry,
): number {
  return (
    a.startMs - b.startMs ||
    a.source.localeCompare(b.source) ||
    a.id.localeCompare(b.id)
  );
}
function upperBound(entries: InspectorContentEntry[], time: number): number {
  let lo = 0,
    hi = entries.length;
  while (lo < hi) {
    const mid = (lo + hi) >>> 1;
    const entry = entries[mid];
    if (entry && entry.startMs <= time) lo = mid + 1;
    else hi = mid;
  }
  return lo;
}
export function inspectorPlaybackWindow(
  index: InspectorContentIndex,
  time: number,
): {
  entries: InspectorContentEntry[];
  focus: InspectorContentEntry | null;
  currentId: string | null;
} {
  const next = upperBound(index.entries, time);
  const visible = new Map(
    index.entries
      .slice(Math.max(0, next - 6), next + 6)
      .map((entry) => [entry.id, entry]),
  );
  let currentId = index.entries[next - 1]?.id ?? null;
  for (const source of index.speechBySource.values()) {
    const current = source[upperBound(source, time) - 1];
    if (current && current.endMs > time) {
      visible.set(current.id, current);
      currentId = current.id;
    }
  }
  return {
    entries: [...visible.values()].sort(compareEntries),
    focus: index.focus[upperBound(index.focus, time) - 1] ?? null,
    currentId,
  };
}
