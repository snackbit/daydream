import { useEffect, useState } from 'react';
import type { LibraryRecordingDebugData } from '@daydream/shared';
import { AlertTriangle, Clipboard } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { getLibraryRecordingDebug } from '../library-client';
import { errorMessage } from './recording-studio-route-utils';

export function RecordingDebugDialog({
  onOpenChange,
  open,
  recordingId,
}: {
  onOpenChange: (open: boolean) => void;
  open: boolean;
  recordingId: string;
}) {
  const [debugData, setDebugData] = useState<LibraryRecordingDebugData | null>(
    null,
  );
  const [debugError, setDebugError] = useState<string | null>(null);
  const [debugLoading, setDebugLoading] = useState(true);
  const [debugCopyStatus, setDebugCopyStatus] = useState<
    'copied' | 'failed' | 'idle'
  >('idle');

  useEffect(() => {
    if (!open) {
      return;
    }
    getLibraryRecordingDebug({ recordingId })
      .then(setDebugData)
      .catch((debugLoadError: unknown) => {
        setDebugError(errorMessage(debugLoadError));
      })
      .finally(() => setDebugLoading(false));
  }, [open, recordingId]);

  async function copyDebugData() {
    if (!debugData) {
      return;
    }
    try {
      await navigator.clipboard.writeText(JSON.stringify(debugData, null, 2));
      setDebugCopyStatus('copied');
    } catch (_error) {
      setDebugCopyStatus('failed');
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>Recording debug data</DialogTitle>
          <DialogDescription>
            Raw local state for this recording. Transcript text may be included.
          </DialogDescription>
        </DialogHeader>
        {debugLoading ? (
          <div className="text-muted-foreground border p-4 text-sm">
            Loading debug data.
          </div>
        ) : null}
        {debugError ? (
          <Alert variant="destructive">
            <AlertTriangle />
            <AlertTitle>Debug data failed</AlertTitle>
            <AlertDescription>{debugError}</AlertDescription>
          </Alert>
        ) : null}
        <pre className="bg-muted max-h-[60vh] overflow-auto p-3 text-xs leading-5">
          {JSON.stringify(debugData ?? { status: 'loading' }, null, 2)}
        </pre>
        <DialogFooter>
          <p
            aria-live="polite"
            className="text-muted-foreground min-h-5 flex-1 text-xs"
          >
            {debugCopyStatus === 'copied' ? 'Debug JSON copied.' : null}
            {debugCopyStatus === 'failed' ? 'Unable to copy debug JSON.' : null}
          </p>
          <Button
            className="gap-2"
            disabled={!debugData}
            variant="outline"
            onClick={() => void copyDebugData()}
          >
            <Clipboard className="size-3.5" />
            Copy JSON
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
