import type { CSSProperties, MouseEvent } from 'react';
import type { VisualFrameProcessingState } from '@daydream/shared';
import { cn } from '@/lib/utils';
import { formatDuration } from '../../library-format';
import type { StudioVisualFrame } from './studio-visual-observations';

type VisualFrameSource = 'mixed' | 'ocr' | 'vlm';

export interface VisualFrameSegmentsProps {
  onSelectVisualFrame: (frame: StudioVisualFrame, additive: boolean) => void;
  selectedVisualFrameIds: Set<string>;
  timelineWidthPx: number;
  totalDurationMs: number;
  visualFrameStates: VisualFrameProcessingState[];
  visualFrames: StudioVisualFrame[];
}

export function VisualFrameSegments({
  onSelectVisualFrame,
  selectedVisualFrameIds,
  timelineWidthPx,
  totalDurationMs,
  visualFrameStates,
  visualFrames,
}: VisualFrameSegmentsProps) {
  const processingMarkers = visualProcessingMarkers({
    visualFrameStates,
    visualFrames,
  });
  if (visualFrames.length === 0 && processingMarkers.length === 0) {
    return null;
  }

  return (
    <>
      {processingMarkers.map((marker) => (
        <VisualProcessingMarker
          key={marker.id}
          marker={marker}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
        />
      ))}
      {visualFrames.map((frame) => (
        <VisualFrameMarker
          frame={frame}
          key={frame.id}
          selected={selectedVisualFrameIds.has(frame.id)}
          timelineWidthPx={timelineWidthPx}
          totalDurationMs={totalDurationMs}
          onSelectVisualFrame={onSelectVisualFrame}
        />
      ))}
    </>
  );
}

function VisualFrameMarker({
  frame,
  onSelectVisualFrame,
  selected,
  timelineWidthPx,
  totalDurationMs,
}: {
  frame: StudioVisualFrame;
  onSelectVisualFrame: (frame: StudioVisualFrame, additive: boolean) => void;
  selected: boolean;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const startMs = Math.max(0, frame.frameTimestampMs);
  const endMs = Math.max(startMs + 1, frame.endTimestampMs);
  const left =
    (Math.min(startMs, totalDurationMs) / totalDurationMs) * timelineWidthPx;
  const width = Math.max(
    4,
    ((Math.min(endMs, totalDurationMs) - Math.min(startMs, totalDurationMs)) /
      totalDurationMs) *
      timelineWidthPx,
  );
  const source = visualFrameSource(frame);
  const title = visualFrameMarkerTitle(frame);

  function handleSelect(event: MouseEvent<HTMLButtonElement>) {
    event.stopPropagation();
    onSelectVisualFrame(
      frame,
      event.shiftKey || event.metaKey || event.ctrlKey,
    );
  }

  return (
    <button
      aria-label={title}
      aria-pressed={selected}
      className={cn(
        'absolute bottom-1 z-20 h-5 overflow-hidden rounded-sm border px-1 text-[0.625rem] font-medium',
        'focus-visible:ring-ring hover:brightness-95 focus-visible:ring-2 focus-visible:outline-none',
        visualFrameMarkerClassName(source),
        selected && 'ring-primary ring-2',
      )}
      data-library-action
      data-visual-frame-source={source}
      style={{
        left,
        width,
        ...visualFrameMarkerStyle(source),
      }}
      title={title}
      type="button"
      onClick={handleSelect}
    >
      <span className="block truncate">
        {formatVisualFrameMarkerTimestamp(frame.frameTimestampMs)}
      </span>
    </button>
  );
}

interface VisualProcessingMarkerModel {
  endTimestampMs: number;
  id: string;
  label: string;
  sourceKind: 'ocr' | 'vlm';
  status: VisualFrameProcessingState['status'];
  timestampMs: number;
}

function visualProcessingMarkers({
  visualFrameStates,
  visualFrames,
}: {
  visualFrameStates: VisualFrameProcessingState[];
  visualFrames: StudioVisualFrame[];
}): VisualProcessingMarkerModel[] {
  const outputKeys = new Set<string>();
  for (const frame of visualFrames) {
    if (frame.ocrRegions.length > 0) {
      for (const sourceFrameRef of frame.sourceFrameRefs) {
        outputKeys.add(`${sourceFrameRef}:ocr`);
      }
    }
    if (frame.vlmObservations.length > 0) {
      for (const sourceFrameRef of frame.sourceFrameRefs) {
        outputKeys.add(`${sourceFrameRef}:vlm`);
      }
    }
  }

  return visualFrameStates
    .filter(
      (state) =>
        state.status !== 'succeeded' ||
        state.outputCount === 0 ||
        !outputKeys.has(`${state.sourceFrameRef}:${state.sourceKind}`),
    )
    .map((state) => ({
      endTimestampMs: state.frameTimestampMs + 1,
      id: state.id,
      label: visualProcessingMarkerLabel(state),
      sourceKind: state.sourceKind,
      status: state.status,
      timestampMs: state.frameTimestampMs,
    }))
    .sort(
      (left, right) =>
        left.timestampMs - right.timestampMs ||
        left.sourceKind.localeCompare(right.sourceKind) ||
        left.id.localeCompare(right.id),
    );
}

function VisualProcessingMarker({
  marker,
  timelineWidthPx,
  totalDurationMs,
}: {
  marker: VisualProcessingMarkerModel;
  timelineWidthPx: number;
  totalDurationMs: number;
}) {
  const left =
    (Math.min(marker.timestampMs, totalDurationMs) / totalDurationMs) *
    timelineWidthPx;

  return (
    <div
      aria-label={marker.label}
      className={cn(
        'border-border bg-muted-foreground/30 absolute bottom-8 z-10 h-2 rounded-full border',
        marker.status === 'queued' && 'opacity-45',
        marker.status === 'running' && 'bg-primary/60 opacity-90',
        marker.status === 'failed' &&
          'border-destructive bg-destructive/60 opacity-90',
        marker.status === 'cancelled' &&
          'border-muted-foreground bg-muted-foreground/40 opacity-60',
        marker.status === 'succeeded' && 'opacity-70',
      )}
      data-library-action
      style={{
        left,
        width: Math.max(4, (1 / totalDurationMs) * timelineWidthPx),
      }}
      title={marker.label}
    />
  );
}

function visualProcessingMarkerLabel(
  state: VisualFrameProcessingState,
): string {
  const kind = state.sourceKind === 'ocr' ? 'OCR' : 'VLM';
  const timestamp = formatVisualFrameMarkerTimestamp(state.frameTimestampMs);
  if (state.status === 'failed' && state.errorCode) {
    return `${kind} frame ${timestamp} failed: ${state.errorCode}`;
  }
  if (state.status === 'succeeded') {
    return `${kind} frame ${timestamp} processed, ${state.outputCount} outputs`;
  }

  return `${kind} frame ${timestamp} ${state.status}`;
}

function visualFrameSource(frame: StudioVisualFrame): VisualFrameSource {
  const hasOcr = frame.ocrRegions.length > 0 || frame.ocrStatus;
  const hasVlm = frame.vlmObservations.length > 0;
  if (hasOcr && hasVlm) {
    return 'mixed';
  }

  return hasVlm ? 'vlm' : 'ocr';
}

function visualFrameMarkerClassName(source: VisualFrameSource): string {
  switch (source) {
    case 'mixed':
      return 'border-primary/70 bg-primary/10 text-foreground';
    case 'vlm':
      return 'border-dashed border-muted-foreground/55 bg-muted text-foreground';
    case 'ocr':
      return 'border-border bg-card text-foreground';
  }
}

function visualFrameMarkerStyle(
  source: VisualFrameSource,
): CSSProperties | undefined {
  if (source !== 'mixed') {
    return undefined;
  }

  return {
    backgroundImage:
      'repeating-linear-gradient(135deg, color-mix(in oklch, var(--color-primary) 12%, var(--color-card)) 0, color-mix(in oklch, var(--color-primary) 12%, var(--color-card)) 6px, color-mix(in oklch, var(--color-muted-foreground) 14%, var(--color-card)) 6px, color-mix(in oklch, var(--color-muted-foreground) 14%, var(--color-card)) 12px)',
  };
}

function formatVisualFrameMarkerTimestamp(timestampMs: number): string {
  if (timestampMs < 60_000 && timestampMs % 1_000 !== 0) {
    return `${(timestampMs / 1_000).toFixed(1)}s`;
  }

  return formatDuration(timestampMs);
}

function visualFrameMarkerTitle(frame: StudioVisualFrame): string {
  const startSeconds = (frame.frameTimestampMs / 1_000).toFixed(1);
  const endSeconds = (frame.endTimestampMs / 1_000).toFixed(1);
  const vlmText =
    frame.vlmObservations.length > 0
      ? `${frame.vlmObservations.length} VLM`
      : 'no VLM';
  const ocrCount = frame.ocrOutputCount ?? frame.ocrRegions.length;
  const ocrText =
    frame.ocrStatus === 'failed'
      ? 'OCR failed'
      : frame.ocrStatus === 'gap'
        ? 'OCR unavailable'
        : frame.ocrStatus === 'cancelled'
          ? 'OCR cancelled'
          : frame.ocrStatus === 'succeeded'
            ? frame.ocrTextEmpty === true
              ? 'OCR complete, no text detected'
              : 'OCR observation'
            : `${ocrCount} OCR ${ocrCount === 1 ? 'candidate' : 'candidates'}`;

  return frame.ocrDocumentId
    ? `Visual frame ${startSeconds}s observation, ${vlmText}, ${ocrText}`
    : `Visual frame ${startSeconds}s to ${endSeconds}s, ${vlmText}, ${ocrText}`;
}
