import { useCallback } from 'react';
import type { StudioVisualFrame } from '../studio-visual-observations';
import {
  OcrDocumentInspector,
  type OcrInspectorContext,
} from './ocr/ocr-document-inspector';
import { VisualSourceInspector } from './visual-source-inspector';

export interface StudioSourceLocation {
  recordingId: string;
  sourceTrackId: string;
  timestampMs: number;
  toTimeline: (sourceMs: number) => number | null;
}
export interface SourceLocationInspectorProps {
  context: OcrInspectorContext;
  time: number;
  source?: StudioSourceLocation | undefined;
  frame?: StudioVisualFrame | undefined;
  label?: string | undefined;
  onSeek: (time: number) => void;
  onClear: () => void;
  mappingRequired?: boolean;
  documentId?: string | null | undefined;
}
export function SourceLocationInspector({
  context,
  time,
  source,
  frame,
  label,
  onSeek,
  onClear,
  mappingRequired = false,
  documentId,
}: SourceLocationInspectorProps) {
  const seek = useCallback(
    (time: number) => {
      const target = source ? source.toTimeline(time) : time;
      if (target !== null) onSeek(target);
    },
    [source, onSeek],
  );
  const actual = source
    ? {
        ...context,
        recordingId: source.recordingId,
        sourceTrackId: source.sourceTrackId,
      }
    : context;
  if (mappingRequired && !source)
    return (
      <p className="text-muted-foreground p-5 text-sm">
        No recorded source at this time.
      </p>
    );
  return frame ? (
    <VisualSourceInspector
      context={actual}
      frame={
        source ? { ...frame, frameTimestampMs: source.timestampMs } : frame
      }
      onClear={onClear}
      onSeek={seek}
      {...(label ? { sourceLabel: label } : {})}
    />
  ) : (
    <OcrDocumentInspector
      context={actual}
      timestampMs={source?.timestampMs ?? time}
      onSeek={seek}
      {...(documentId === undefined ? {} : { documentId })}
    />
  );
}
