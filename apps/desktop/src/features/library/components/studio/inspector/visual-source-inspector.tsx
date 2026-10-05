import { useCallback } from 'react';
import { Button } from '@/components/ui/button';
import type { StudioVisualFrame } from '../studio-visual-observations';
import { formatTimelineOffset } from './inspector-format';
import { InspectorField, InspectorSection } from './inspector-section';

export interface VisualSourceInspectorProps {
  frame: StudioVisualFrame;
  onClear: () => void;
  onSeek: (timelineOffsetMs: number) => void;
}

export function VisualSourceInspector({
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
          <InspectorField
            label="Frame references"
            value={frame.sourceFrameRefs.length.toString()}
          />
          <InspectorField
            label="OCR regions"
            value={frame.ocrRegions.length.toString()}
          />
          <InspectorField
            label="Screen observations"
            value={frame.vlmObservations.length.toString()}
          />
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

      <InspectorSection title="OCR text">
        {frame.ocrRegions.length > 0 ? (
          <div className="divide-y">
            {frame.ocrRegions.map((region) => (
              <div className="grid gap-1 py-1.5 first:pt-0" key={region.id}>
                <p className="text-[11px] leading-4 whitespace-pre-wrap">
                  {region.text}
                </p>
                <dl className="grid gap-1">
                  <InspectorField label="Provider" value={region.provider} />
                  <InspectorField
                    label="Model"
                    value={region.model ?? 'Unavailable'}
                  />
                  <InspectorField
                    label="Confidence"
                    value={formatConfidence(region.confidence)}
                  />
                  <InspectorField
                    label="Chunk ID"
                    value={<code>{region.chunkId}</code>}
                  />
                  <InspectorField
                    label="Frame reference"
                    value={<code>{region.sourceFrameRef}</code>}
                  />
                </dl>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-muted-foreground text-[10px]">
            No OCR regions for this frame.
          </p>
        )}
      </InspectorSection>

      <InspectorSection title="Screen observations">
        {frame.vlmObservations.length > 0 ? (
          <div className="divide-y">
            {frame.vlmObservations.map((observation) => (
              <div
                className="grid gap-1.5 py-1.5 first:pt-0"
                key={observation.id}
              >
                <p className="text-[11px] leading-4 whitespace-pre-wrap">
                  {observation.text}
                </p>
                <dl className="grid gap-1">
                  <InspectorField
                    label="Observation"
                    value={observation.observationKind}
                  />
                  <InspectorField
                    label="Provider"
                    value={observation.provider}
                  />
                  <InspectorField
                    label="Model"
                    value={observation.model ?? 'Unavailable'}
                  />
                  <InspectorField
                    label="Confidence"
                    value={formatConfidence(observation.confidence)}
                  />
                  <InspectorField
                    label="Chunk ID"
                    value={<code>{observation.chunkId}</code>}
                  />
                  <InspectorField
                    label="Frame reference"
                    value={<code>{observation.sourceFrameRef}</code>}
                  />
                </dl>
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

function formatConfidence(confidence: number | null): string {
  return confidence === null
    ? 'Not provided'
    : confidence.toLocaleString(undefined, { maximumFractionDigits: 3 });
}
