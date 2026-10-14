import type {
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
} from '@daydream/shared';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { formatDuration } from '../../../../library-format';
import { useRedactionMutation } from '../../redactions/use-redaction-mutation';
import { MetadataRow, StudioSidebarSection } from './studio-sidebar-section';

export function StudioSidebarRedactionSection({
  detail,
  onDeleteRedaction,
  redaction,
}: {
  detail: LibraryRecordingDetail;
  onDeleteRedaction: (redactionId: string) => Promise<void>;
  redaction: LibraryRecordingRedaction;
}) {
  const mutation = useRedactionMutation();
  function unredact() {
    void mutation.run(() => onDeleteRedaction(redaction.id));
  }
  const trackLabels = redaction.trackIds
    .map(
      (trackId) =>
        detail.tracks.find((track) => track.id === trackId)?.label ?? trackId,
    )
    .join(', ');

  return (
    <StudioSidebarSection title="Redaction">
      <MetadataRow
        label="Range"
        value={`${formatDuration(redaction.startOffsetMs)} - ${formatDuration(
          redaction.endOffsetMs,
        )}`}
      />
      <MetadataRow label="Targets" value={trackLabels} />
      <MetadataRow
        label="Duration"
        value={formatDuration(redaction.endOffsetMs - redaction.startOffsetMs)}
      />
      {mutation.error ? (
        <Alert variant="destructive">
          <AlertDescription>{mutation.error}</AlertDescription>
        </Alert>
      ) : null}
      {mutation.pending ? (
        <p role="status" className="text-muted-foreground text-xs">
          Updating redactions…
        </p>
      ) : null}
      <Button
        className="w-full"
        disabled={mutation.pending}
        type="button"
        variant="outline"
        onClick={unredact}
      >
        Unredact
      </Button>
    </StudioSidebarSection>
  );
}
