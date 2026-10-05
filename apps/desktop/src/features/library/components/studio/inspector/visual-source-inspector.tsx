import { useCallback } from 'react';
import { Button } from '@/components/ui/button';
import type { StudioVisualFrame } from '../studio-visual-observations';
import { formatTimelineOffset } from './inspector-format';
import { InspectorField, InspectorSection } from './inspector-section';
import {
  OcrDocumentInspector,
  type OcrInspectorContext,
} from './ocr/ocr-document-inspector';

export interface VisualSourceInspectorProps {
  context: OcrInspectorContext;
  frame: StudioVisualFrame;
  onClear: () => void;
  onSeek: (timelineOffsetMs: number) => void;
}

export function VisualSourceInspector({
  context,
  frame,
  onClear,
  onSeek,
}: VisualSourceInspectorProps) {
  const handleSeek = useCallback(() => {
    onSeek(frame.frameTimestampMs);
  }, [frame.frameTimestampMs, onSeek]);
  return (
    <>
      <InspectorSection title="Selected frame">
        <dl className="space-y-1.5">
          <InspectorField
            label="Time"
            value={formatTimelineOffset(frame.frameTimestampMs)}
          />
          <InspectorField label="Source" value={context.sourceTrackId} />
        </dl>
        <div className="flex gap-1 border-t pt-2">
          <Button
            className="min-w-0 flex-1"
            onClick={handleSeek}
            size="sm"
            type="button"
            variant="outline"
          >
            Jump to frame
          </Button>
          <Button
            aria-label="Clear selected frame"
            onClick={onClear}
            size="icon-sm"
            type="button"
            variant="ghost"
          >
            <span aria-hidden="true">×</span>
          </Button>
        </div>
      </InspectorSection>
      <OcrDocumentInspector
        context={context}
        key={JSON.stringify([
          context.recordingId,
          context.sourceTrackId,
          context.privacyRevision,
          frame.frameTimestampMs,
        ])}
        onSeek={onSeek}
        timestampMs={frame.frameTimestampMs}
        documentId={frame.ocrDocumentId ?? null}
      />
      <InspectorSection title="Screen observations">
        {frame.vlmObservations.length > 0 ? (
          <div className="divide-y">
            {frame.vlmObservations.map((observation) => (
              <div className="space-y-1 py-2" key={observation.id}>
                <p className="text-[11px] leading-5 break-words whitespace-pre-wrap">
                  {observation.text}
                </p>
                <p className="text-muted-foreground text-[10px]">
                  {observation.observationKind === 'corrected-ocr'
                    ? 'Visual-model OCR correction observation'
                    : 'Visual-model screen observation'}{' '}
                  · {observation.model ?? observation.provider}
                </p>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-muted-foreground text-[10px]">
            No screen observation for this frame.
          </p>
        )}
      </InspectorSection>
    </>
  );
}
