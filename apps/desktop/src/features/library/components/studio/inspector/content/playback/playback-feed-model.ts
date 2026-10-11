import type { RecordingTranscriptWord } from '@daydream/shared';
import { formatDuration } from '../../../../../library-format';
import {
  inspectorPlaybackWindow,
  type InspectorContentEntry,
  type InspectorContentIndex,
} from '../inspector-content';
import {
  transcriptTextSpans,
  type TranscriptTextSpan,
} from '../transcript-text-spans';

export type PlaybackWordSpan = TranscriptTextSpan;
export interface PlaybackFeedIndex extends InspectorContentIndex {
  spans: Map<string, PlaybackWordSpan[]>;
}
export interface PlaybackFeedFrame {
  entries: InspectorContentEntry[];
  focus: InspectorContentEntry | null;
  currentId: string | null;
  activeWords: Map<string, string>;
  rulerWordId: string | undefined;
  activeEntries: Set<string>;
  upcomingEntries: Set<string>;
  time: number;
  signature: string;
}

export function playbackTimestamp(time: number): string {
  return formatDuration(Math.floor(Math.max(0, time) / 1000) * 1000);
}

export function playbackSpeechStartsGroup(
  entry: InspectorContentEntry,
  previous: InspectorContentEntry | undefined,
): boolean {
  return (
    entry.kind === 'speech' &&
    (previous?.kind !== 'speech' ||
      previous.trackId !== entry.trackId ||
      previous.source !== entry.source)
  );
}

/** Align timings to canonical text once; never rebuild the transcript from tokens. */
export function playbackWordSpans(
  entry: InspectorContentEntry,
): PlaybackWordSpan[] {
  return transcriptTextSpans(entry.text, entry.words);
}

export function buildPlaybackFeedIndex(
  index: InspectorContentIndex,
): PlaybackFeedIndex {
  const previousObservation = new Map<string, string>();
  const entries = index.entries.filter((entry) => {
    if (entry.kind === 'screen-text') return false;
    if (entry.kind !== 'observation') return true;
    if (entry.observationKind !== 'screen-observation') return false;
    const text = entry.text.trim().replace(/\s+/gu, ' ');
    if (!text || previousObservation.get(entry.trackId) === text) return false;
    previousObservation.set(entry.trackId, text);
    return true;
  });
  return {
    ...index,
    entries,
    spans: new Map(
      entries
        .filter((entry) => entry.kind === 'speech')
        .map((entry) => [entry.id, playbackWordSpans(entry)]),
    ),
  };
}

export function playbackFeedFrame(
  index: PlaybackFeedIndex,
  time: number,
): PlaybackFeedFrame {
  const window = inspectorPlaybackWindow(index, time);
  const activeWords = new Map<string, string>();
  const activeEntries = new Set<string>();
  const upcomingEntries = new Set<string>();
  const speaking = new Map<string, InspectorContentEntry>();
  for (const entry of window.entries) {
    if (entry.startMs > time) upcomingEntries.add(entry.id);
    if (entry.startMs <= time && time < entry.endMs) {
      activeEntries.add(entry.id);
      if (entry.kind === 'speech') speaking.set(entry.trackId, entry);
    }
  }
  for (const entry of speaking.values()) {
    let active: RecordingTranscriptWord | null = null;
    for (const span of index.spans.get(entry.id) ?? []) {
      const word = span.word;
      if (
        word &&
        word.startOffsetMs <= time &&
        time < word.endOffsetMs &&
        (!active || word.startOffsetMs >= active.startOffsetMs)
      )
        active = word;
    }
    if (active) activeWords.set(entry.id, active.id);
  }
  const anchor =
    [...speaking.values()].find((entry) => entry.source === 'Microphone') ??
    speaking.values().next().value;
  const currentId = anchor?.id ?? window.currentId;
  // The marker holds a text position through silence; highlights remain exact
  // intervals. Derive it from this entry/time so seeks and privacy changes reset it.
  let firstWord: RecordingTranscriptWord | null = null;
  let previousWord: RecordingTranscriptWord | null = null;
  for (const span of currentId === null
    ? []
    : (index.spans.get(currentId) ?? [])) {
    const word = span.word;
    if (!word) continue;
    if (!firstWord || word.startOffsetMs < firstWord.startOffsetMs)
      firstWord = word;
    if (
      word.startOffsetMs <= time &&
      (!previousWord || word.startOffsetMs >= previousWord.startOffsetMs)
    )
      previousWord = word;
  }
  const rulerWordId =
    (currentId === null ? undefined : activeWords.get(currentId)) ??
    previousWord?.id ??
    firstWord?.id;
  return {
    ...window,
    currentId,
    activeWords,
    rulerWordId,
    activeEntries,
    upcomingEntries,
    time,
    signature: JSON.stringify([
      window.entries.map((entry) => entry.id),
      currentId,
      rulerWordId,
      window.focus?.id,
      [...activeWords],
      [...activeEntries],
      [...upcomingEntries],
      Math.floor(time / 1000),
    ]),
  };
}
