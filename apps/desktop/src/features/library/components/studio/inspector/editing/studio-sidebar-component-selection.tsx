import type {
  CreateLibraryRedactionRangeInput,
  LibraryRecordingDetail,
  RecordingAiProcessingState,
} from '@daydream/shared';
import { ShieldOff, ShieldX } from 'lucide-react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { formatDuration } from '../../../../library-format';
import { redactionsIntersectingRanges } from '../../redactions/studio-redactions';
import { useSelectionRedactionMutation } from '../../redactions/use-redaction-mutation';
import { type StudioTimeRangeSelection } from '../../timeline/state/studio-timeline-store';
import {
  selectInspectorEntries,
  type InspectorContentIndex,
} from '../content/inspector-content';
import { InspectorContentEntryCard } from '../content/inspector-content-entry';
import { MetadataRow, StudioSidebarSection } from './studio-sidebar-section';
import { TranscriptionSelectionDetails } from './studio-sidebar-transcription';

export interface StudioSidebarComponentSelectionProps {
  aiProcessingState: RecordingAiProcessingState | null;
  clearSelection: () => void;
  detail: LibraryRecordingDetail;
  onCreateRedactions: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  onUnredactRanges: (
    ranges: CreateLibraryRedactionRangeInput[],
  ) => Promise<void>;
  selections: StudioTimeRangeSelection[];
  content?: InspectorContentIndex;
}

export function StudioSidebarComponentSelection({
  aiProcessingState,
  clearSelection,
  detail,
  onCreateRedactions,
  onUnredactRanges,
  selections,
  content,
}: StudioSidebarComponentSelectionProps) {
  const mutation = useSelectionRedactionMutation({
    ranges: selections,
    onCreate: onCreateRedactions,
    onUnredact: onUnredactRanges,
    onClear: clearSelection,
  });
  const intersectingRedactions = redactionsIntersectingRanges(
    detail.redactions,
    selections,
  );
  const totalDurationMs = selections.reduce(
    (durationMs, selection) =>
      durationMs + Math.max(0, selection.endMs - selection.startMs),
    0,
  );

  return (
    <StudioSidebarSection title="Selection">
      <MetadataRow label="Duration" value={formatDuration(totalDurationMs)} />
      <MetadataRow
        label="Targets"
        value={`${new Set(selections.flatMap((range) => range.laneIds)).size} ${new Set(selections.flatMap((range) => range.laneIds)).size === 1 ? 'lane' : 'lanes'}`}
      />
      <div className="space-y-1">
        {selections.map((selection, index) => (
          <MetadataRow
            key={selection.id}
            label={`Range ${index + 1}`}
            value={`${formatDuration(selection.startMs)} - ${formatDuration(
              selection.endMs,
            )}`}
          />
        ))}
      </div>
      {content ? (
        <ol className="space-y-2">
          {selectInspectorEntries(content.entries, selections).map((entry) => (
            <InspectorContentEntryCard key={entry.id} entry={entry} />
          ))}
        </ol>
      ) : (
        <TranscriptionSelectionDetails
          aiProcessingState={aiProcessingState}
          selections={selections}
        />
      )}
      <div className="grid gap-2">
        {mutation.pending ? (
          <p role="status" className="text-muted-foreground text-xs">
            Updating redactions…
          </p>
        ) : null}
        {mutation.error ? (
          <Alert variant="destructive">
            <AlertDescription>{mutation.error}</AlertDescription>
          </Alert>
        ) : null}
        <Button
          disabled={mutation.pending}
          type="button"
          variant="outline"
          onClick={mutation.redact}
        >
          <ShieldX data-icon="inline-start" />
          Redact
        </Button>
        {intersectingRedactions.length > 0 ? (
          <Button
            disabled={mutation.pending}
            type="button"
            variant="outline"
            onClick={mutation.unredact}
          >
            <ShieldOff data-icon="inline-start" />
            Unredact
          </Button>
        ) : null}
      </div>
    </StudioSidebarSection>
  );
}
