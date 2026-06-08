import type { LibraryRecordingDetail } from '@daydream/shared';
import { AlertTriangle, Lock } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Separator } from '@/components/ui/separator';
import { formatDuration, formatTimestamp } from '../../library-format';

export function StudioSidebar({ detail }: { detail: LibraryRecordingDetail }) {
  return (
    <aside className="bg-sidebar text-sidebar-foreground flex h-full flex-col">
      <div className="border-b p-3">
        <div className="text-sm font-medium">Details</div>
        <div className="text-muted-foreground mt-1 text-xs">
          Annotation and metadata workspace
        </div>
      </div>
      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-3 p-3 text-xs">
          {detail.availability !== 'available' ? (
            <Alert
              variant={
                detail.availability === 'locked' ? 'default' : 'destructive'
              }
            >
              {detail.availability === 'locked' ? <Lock /> : <AlertTriangle />}
              <AlertTitle>
                {detail.availability === 'locked'
                  ? 'Recording in progress'
                  : 'Missing video files'}
              </AlertTitle>
              <AlertDescription>
                {detail.missingReasons.join(' ') ||
                  'Recording files are unavailable.'}
              </AlertDescription>
            </Alert>
          ) : null}
          <MetadataRow label="Recording" value={detail.id} />
          <MetadataRow
            label="Started"
            value={formatTimestamp(detail.startedAtMs)}
          />
          <MetadataRow
            label="Duration"
            value={formatDuration(detail.durationMs)}
          />
          <Separator />
          <div className="text-muted-foreground">
            Annotations, extracted metadata, and source-linked notes will appear
            here.
          </div>
        </div>
      </ScrollArea>
    </aside>
  );
}

function MetadataRow({ label, value }: { label: string; value: string }) {
  return (
    <div className="grid grid-cols-[5rem_1fr] gap-2">
      <div className="text-muted-foreground">{label}</div>
      <div className="truncate">{value}</div>
    </div>
  );
}
