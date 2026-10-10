import { useCallback, useState } from 'react';
import type {
  RecallEvidence,
  CreateLibraryRedactionRangeInput,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { formatDuration, formatTimestamp } from '../../../library-format';
import { InspectorField, InspectorSection } from './inspector-section';

export interface RecallEvidenceInspectorProps {
  evidence: RecallEvidence;
  onSeek: (offsetMs: number) => void;
  onClear: () => void;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  debugEnabled?: boolean;
}

export function RecallEvidenceInspector({
  evidence,
  onSeek,
  onClear,
  onCreateRedactions,
  debugEnabled = false,
}: RecallEvidenceInspectorProps) {
  const [masking, setMasking] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mask = useCallback(async () => {
    setMasking(true);
    setError(null);
    try {
      await onCreateRedactions([
        {
          startOffsetMs: evidence.startOffsetMs,
          endOffsetMs: evidence.endOffsetMs,
          trackIds: [evidence.sourceTrackId],
        },
      ]);
    } catch {
      setError('This source could not be hidden. Try again.');
    } finally {
      setMasking(false);
    }
  }, [
    evidence.startOffsetMs,
    evidence.endOffsetMs,
    evidence.sourceTrackId,
    onCreateRedactions,
  ]);
  const jump = useCallback(
    () => onSeek(evidence.startOffsetMs),
    [onSeek, evidence.startOffsetMs],
  );
  return (
    <InspectorSection title="Recall source">
      <p className="leading-6 break-words whitespace-pre-wrap">
        {evidence.text}
      </p>
      <dl className="space-y-3 text-xs">
        <InspectorField
          label="Source"
          value={
            evidence.sourceKind === 'ocr'
              ? 'Screen text'
              : evidence.sourceKind === 'desktop-audio'
                ? 'Desktop audio'
                : 'Microphone'
          }
        />
        <InspectorField
          label="Time"
          value={formatTimestamp(evidence.absoluteStartMs)}
        />
        <InspectorField
          label="Range"
          value={`${formatDuration(evidence.startOffsetMs)} – ${formatDuration(evidence.endOffsetMs)}`}
        />
        {debugEnabled ? (
          <>
            <InspectorField label="Segment" value={evidence.segmentId} />
            <InspectorField label="Track" value={evidence.sourceTrackId} />
            <InspectorField label="Evidence" value={evidence.sourceId} />
          </>
        ) : null}
      </dl>
      <div className="flex flex-wrap gap-2">
        <Button variant="outline" onClick={jump}>
          Jump to source
        </Button>
        <Button variant="ghost" onClick={onClear}>
          Clear recall source
        </Button>
        <Button variant="outline" disabled={masking} onClick={mask}>
          {masking ? 'Hiding source…' : 'Hide source interval'}
        </Button>
      </div>
      <p className="text-muted-foreground text-xs leading-5">
        Hiding is reversible. It withholds this interval from processing, search
        and recall while retaining the original media.
      </p>
      {error ? (
        <p role="alert" className="text-destructive text-sm">
          {error}
        </p>
      ) : null}
    </InspectorSection>
  );
}
