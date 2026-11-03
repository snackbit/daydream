import { useEffect, useState } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { Skeleton } from '@/components/ui/skeleton';
import { getLibraryRecordingSize } from '../../../library-client';
import { formatFileSize } from '../../../library-format';

type DiskUsageRead =
  | { owner: string; status: 'error' }
  | { owner: string; status: 'ready'; bytes: number };

/** Disk scans belong to the visible details field, never the playback model. */
export function RecordingDiskUsage({
  detail,
  active,
}: {
  detail: LibraryRecordingDetail;
  active: boolean;
}) {
  const [read, setRead] = useState<DiskUsageRead | null>(null);
  const [attempt, setAttempt] = useState(0);
  const owner = JSON.stringify([detail.id, detail.redactions, attempt]);
  useEffect(() => {
    if (!active || detail.sizeBytes !== null || detail.id.startsWith('day:'))
      return;
    let current = true;
    void getLibraryRecordingSize(detail.id)
      .then((bytes) => {
        if (current) setRead({ owner, status: 'ready', bytes });
      })
      .catch(() => {
        if (current) setRead({ owner, status: 'error' });
      });
    return () => {
      current = false;
    };
  }, [active, detail.id, detail.sizeBytes, owner, attempt]);

  if (detail.sizeBytes !== null) return formatFileSize(detail.sizeBytes);
  if (detail.id.startsWith('day:')) return 'Select a recording for disk usage';
  const current = read?.owner === owner ? read : null;
  return (
    <span
      title="Last measured local storage usage"
      className="inline-flex min-h-5 items-center gap-2"
    >
      {current?.status === 'ready' ? (
        formatFileSize(current.bytes)
      ) : current?.status === 'error' ? (
        <>
          <span>Unavailable</span>
          <Button
            variant="link"
            size="sm"
            className="h-5 p-0"
            onClick={() => setAttempt((value) => value + 1)}
          >
            Retry disk usage
          </Button>
        </>
      ) : (
        <>
          <Skeleton aria-hidden="true" className="h-3 w-20" />
          <span className="sr-only">Loading disk usage</span>
        </>
      )}
    </span>
  );
}
