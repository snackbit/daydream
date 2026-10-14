import { formatDuration } from '../../../../library-format';
import type { StudioVisualFrame } from '../../observations/studio-visual-observations';
import { MetadataRow, StudioSidebarSection } from './studio-sidebar-section';

export interface StudioSidebarVisualFrameSectionProps {
  frame: StudioVisualFrame;
}

export function StudioSidebarVisualFrameSection({
  frame,
}: StudioSidebarVisualFrameSectionProps) {
  const observations = frame.vlmObservations.filter(
    (item) => item.observationKind !== 'corrected-ocr',
  );

  return (
    <StudioSidebarSection
      title={`Visual frame ${formatDuration(
        frame.frameTimestampMs,
      )} - ${formatDuration(frame.endTimestampMs)}`}
    >
      <MetadataRow
        label="Sources"
        value={`${frame.sourceFrameRefs.length} frame ${
          frame.sourceFrameRefs.length === 1 ? 'ref' : 'refs'
        }`}
      />
      <MetadataRow label="VLM" value={String(observations.length)} />
      <MetadataRow label="OCR" value={String(frame.ocrRegions.length)} />

      <div className="space-y-2 border-t pt-2">
        <div className="text-muted-foreground">Screen observations</div>
        {observations.length > 0 ? (
          observations.map((observation) => (
            <div
              className="bg-muted/60 rounded-sm border p-2 text-xs leading-5 whitespace-pre-wrap"
              key={observation.id}
            >
              {observation.text}
            </div>
          ))
        ) : (
          <div className="text-muted-foreground rounded-sm border border-dashed p-2">
            No VLM observation for this frame.
          </div>
        )}
      </div>
    </StudioSidebarSection>
  );
}
