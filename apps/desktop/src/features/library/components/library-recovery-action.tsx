import { useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import type {
  LibraryRecordingDetail,
  RecordingRecoveryResult,
} from '@daydream/shared';
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { unknownErrorMessage } from '@/lib/async-store-action';
import { recoverLibraryRecording } from '../recovery-client';

export interface LibraryRecoveryActionProps {
  detail: LibraryRecordingDetail;
}
export function LibraryRecoveryAction({ detail }: LibraryRecoveryActionProps) {
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<RecordingRecoveryResult | null>(null);
  const running = useRef(false);
  function requestRecovery() {
    setError(null);
    setResult(null);
    setOpen(true);
  }
  function changeOpen(value: boolean) {
    if (!running.current) setOpen(value);
  }
  function cancel() {
    changeOpen(false);
  }
  async function confirmRecovery() {
    if (running.current) return;
    running.current = true;
    setPending(true);
    setError(null);
    try {
      setResult(await recoverLibraryRecording({ recordingId: detail.id }));
    } catch (cause: unknown) {
      setError(unknownErrorMessage(cause));
    } finally {
      running.current = false;
      setPending(false);
    }
  }
  function recover() {
    void confirmRecovery();
  }
  function viewRecovered() {
    if (result) {
      setOpen(false);
      void navigate({
        to: '/library/$recordingId',
        params: { recordingId: result.recoveredRecordingId },
      });
    }
  }
  if (detail.status === 'recovered-partial')
    return (
      <span className="text-muted-foreground max-w-64 text-xs">
        Recovered partial copy. Review the recovered coverage before choosing
        Process AI.
      </span>
    );
  if (detail.status !== 'failed') return null;
  return (
    <>
      <Button variant="outline" onClick={requestRecovery}>
        Recover recording
      </Button>
      <AlertDialog open={open} onOpenChange={changeOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>Recover recording</AlertDialogTitle>
            <AlertDialogDescription>
              Create a new partial recording from usable media. The failed
              original is kept unchanged. Background readers stop first;
              recovery does not run local or remote analysis.
            </AlertDialogDescription>
          </AlertDialogHeader>
          {result ? (
            <p role="status">
              Recovered {result.recoveredSegmentCount} segments; skipped{' '}
              {result.skippedSegmentCount}. Gaps remain in the source timeline.
              Review the new copy before processing it.
            </p>
          ) : null}
          {error ? (
            <p role="alert" className="text-destructive text-sm">
              {error}
            </p>
          ) : null}
          <AlertDialogFooter>
            <Button variant="outline" disabled={pending} onClick={cancel}>
              {result ? 'Close' : 'Cancel'}
            </Button>
            {result ? (
              <Button onClick={viewRecovered}>View recovered recording</Button>
            ) : (
              <Button disabled={pending} onClick={recover}>
                {pending ? 'Recovering…' : 'Create recovered copy'}
              </Button>
            )}
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
