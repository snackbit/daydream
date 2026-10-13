import { useMemo } from 'react';
import type {
  LibraryRecordingSegment,
  LibraryRecordingTrack,
  RecordingTranscriptionChunkState,
} from '@daydream/shared';
import { type DerivedAiLane } from './derived-ai-lane-model';
import type { StudioInspectorSelection } from './studio-inspector-selection';
import {
  groupTimelineEntries,
  TimelineEntryGroupButton,
} from './timeline/timeline-entry-groups';
import {
  SegmentTranscriptionItem,
  type SegmentTranscriptionDisplayItem,
} from './timeline/transcripts/segment-transcription-item';
import { TranscriptGroupPreview } from './timeline/transcripts/transcript-group-preview';
import { TRACK_TRANSCRIPTION_LANE_HEIGHT_PX } from './track-timeline-math';

export { buildDerivedLane } from './derived-ai-lane-model';
export type { DerivedAiLane } from './derived-ai-lane-model';

export function SegmentTranscriptionLayer({
  color,
  lane,
  segment,
  track,
  onInspectTranscript,
  hiddenRanges = [],
  publishedRanges = [],
  revealOffsetMs = null,
  visibleRange,
  timelineWidthPx,
  onZoomRange,
  overviewRanges = [],
}: {
  color: string;
  lane: DerivedAiLane;
  segment: LibraryRecordingSegment;
  track: LibraryRecordingTrack;
  onInspectTranscript: (selection: StudioInspectorSelection) => void;
  hiddenRanges?: readonly { startOffsetMs: number; endOffsetMs: number }[];
  publishedRanges?: readonly { startOffsetMs: number; endOffsetMs: number }[];
  revealOffsetMs?: number | null;
  visibleRange?: { startTimelineMs: number; endTimelineMs: number } | undefined;
  timelineWidthPx?: number | undefined;
  onZoomRange?: ((start: number, end: number) => void) | undefined;
  overviewRanges?: readonly { startOffsetMs: number; endOffsetMs: number }[];
}) {
  const segmentStartMs = segment.playbackOffsetMs;
  const segmentDurationMs = Math.max(1, segment.durationMs ?? 0);
  const segmentEndMs = segmentStartMs + segmentDurationMs;
  const paintStartMs = Math.max(
    segmentStartMs,
    visibleRange?.startTimelineMs ?? segmentStartMs,
  );
  const paintEndMs = Math.min(
    segmentEndMs,
    visibleRange?.endTimelineMs ?? segmentEndMs,
  );
  const paintDurationMs = Math.max(1, paintEndMs - paintStartMs);
  const pixelsPerMs = (timelineWidthPx ?? 1) / segmentDurationMs;
  const displayItems = useMemo(() => {
    const chunks = lane.chunks.filter(
      (chunk) =>
        (chunk.startOffsetMs ?? 0) < segmentEndMs &&
        (chunk.endOffsetMs ?? chunk.startOffsetMs ?? 0) > segmentStartMs,
    );
    // Current scopes precede retained history. Paint historical status behind
    // current transcript actions when their processing windows overlap.
    const seen = new Set<string>();
    return chunks
      .toReversed()
      .flatMap((chunk) =>
        transcriptionDisplayItems(chunk, segmentStartMs, segmentEndMs),
      )
      .filter((item) => {
        const id =
          item.kind === 'transcript-segment'
            ? item.transcriptSegment.id
            : item.id;
        if (
          seen.has(id) ||
          overviewRanges.some(
            (range) =>
              (item.startOffsetMs ?? 0) < range.endOffsetMs &&
              (item.endOffsetMs ?? 0) > range.startOffsetMs,
          ) ||
          (hiddenRanges.some(
            (range) =>
              (item.startOffsetMs ?? 0) < range.endOffsetMs &&
              (item.endOffsetMs ?? 0) > range.startOffsetMs,
          ) &&
            !publishedRanges.some(
              (range) =>
                item.kind === 'transcript-segment' &&
                (item.startOffsetMs ?? 0) < range.endOffsetMs &&
                (item.endOffsetMs ?? 0) > range.startOffsetMs,
            ))
        )
          return false;
        seen.add(id);
        return true;
      });
  }, [
    lane.chunks,
    segmentStartMs,
    segmentEndMs,
    hiddenRanges,
    publishedRanges,
    overviewRanges,
  ]);
  const visibleItems = displayItems.filter(
    (item) =>
      !visibleRange ||
      ((item.startOffsetMs ?? 0) < visibleRange.endTimelineMs &&
        (item.endOffsetMs ?? 0) > visibleRange.startTimelineMs),
  );
  const groups = groupTimelineEntries(
    visibleItems.map((item) => ({
      ...item,
      startMs: item.startOffsetMs ?? 0,
      endMs: item.endOffsetMs ?? 0,
    })),
    (timelineWidthPx ?? Number.MAX_SAFE_INTEGER) / segmentDurationMs,
  );

  if (displayItems.length === 0) {
    return null;
  }

  return (
    <div
      aria-label={lane.statusLabel}
      className="bg-background/80 absolute bottom-0 overflow-hidden border-t"
      style={{
        height: TRACK_TRANSCRIPTION_LANE_HEIGHT_PX,
        left: visibleRange ? (paintStartMs - segmentStartMs) * pixelsPerMs : 0,
        width: visibleRange ? paintDurationMs * pixelsPerMs : '100%',
      }}
    >
      {groups.map((group) =>
        group.id.startsWith('group:') && onZoomRange ? (
          <TimelineEntryGroupButton
            key={group.id}
            group={{
              ...group,
              startMs: group.startMs,
              endMs: group.endMs,
            }}
            label="transcript entries"
            pixelsPerMs={(timelineWidthPx ?? 1) / segmentDurationMs}
            offsetPx={paintStartMs * pixelsPerMs}
            onZoom={onZoomRange}
            tooltipContent={
              <TranscriptGroupPreview
                trackLabel={track.label}
                count={group.items.length}
                startMs={group.startMs}
                endMs={group.endMs}
                entries={group.items
                  .filter((item) => item.kind === 'transcript-segment')
                  .toSorted((a, b) => a.startMs - b.startMs)
                  .slice(0, 3)
                  .map((item) => ({
                    id: item.id,
                    segmentId: item.chunk.segmentId ?? '',
                    startOffsetMs: item.startMs,
                    endOffsetMs: item.endMs,
                    text: Array.from(item.text ?? '')
                      .slice(0, 240)
                      .join(''),
                    truncated: Array.from(item.text ?? '').length > 240,
                  }))}
              />
            }
          />
        ) : (
          group.items.map((item) => (
            <SegmentTranscriptionItem
              color={color}
              item={item}
              key={item.id}
              segmentDurationMs={paintDurationMs}
              segmentStartMs={paintStartMs}
              track={track}
              onInspectTranscript={onInspectTranscript}
              covered={
                revealOffsetMs !== null &&
                (item.endOffsetMs ?? 0) > revealOffsetMs
              }
            />
          ))
        ),
      )}
    </div>
  );
}

function transcriptionDisplayItems(
  chunk: RecordingTranscriptionChunkState,
  segmentStartMs: number,
  segmentEndMs: number,
): SegmentTranscriptionDisplayItem[] {
  const transcriptSegments = chunk.transcriptSegments
    .filter(
      (transcriptSegment) =>
        transcriptSegment.startOffsetMs < segmentEndMs &&
        transcriptSegment.endOffsetMs > segmentStartMs,
    )
    .map((transcriptSegment) => ({
      chunk,
      endOffsetMs: transcriptSegment.endOffsetMs,
      id: `${chunk.id}:${transcriptSegment.id}`,
      kind: 'transcript-segment' as const,
      startOffsetMs: transcriptSegment.startOffsetMs,
      text: transcriptSegment.text,
      transcriptSegment,
    }));

  if (transcriptSegments.length > 0) {
    return transcriptSegments;
  }

  return [
    {
      chunk,
      endOffsetMs: chunk.endOffsetMs,
      id: chunk.id,
      kind: 'chunk',
      startOffsetMs: chunk.startOffsetMs,
      text: chunk.text,
    },
  ];
}
