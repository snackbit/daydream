import type {
  LibraryRecordingDetail,
  LibraryRecordingRedaction,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { formatDuration } from '../../library-format';
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
      <Button
        className="w-full"
        type="button"
        variant="outline"
        onClick={() => void onDeleteRedaction(redaction.id)}
      >
        Unredact
      </Button>
    </StudioSidebarSection>
  );
}
