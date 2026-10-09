import { useState } from 'react';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { errorMessage } from './recording-studio-route-utils';

export function RecordingDeleteDialog({
  onConfirm,
  onOpenChange,
  open,
}: {
  onConfirm: () => Promise<void>;
  onOpenChange: (open: boolean) => void;
  open: boolean;
}) {
  const [deleting, setDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<string | null>(null);
  function changeOpen(nextOpen: boolean) {
    if (deleting) return;
    setDeleteError(null);
    onOpenChange(nextOpen);
  }
  function cancel() {
    changeOpen(false);
  }
  async function confirm() {
    if (deleting) return;
    setDeleting(true);
    setDeleteError(null);
    try {
      await onConfirm();
    } catch (error) {
      setDeleteError(errorMessage(error));
    } finally {
      setDeleting(false);
    }
  }
  function submit() {
    void confirm();
  }
  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete recording</DialogTitle>
          <DialogDescription>
            This removes the broken Library entry and any remaining files in
            Daydream storage.
          </DialogDescription>
        </DialogHeader>
        {deleteError ? (
          <Alert variant="destructive">
            <AlertDescription>{deleteError}</AlertDescription>
          </Alert>
        ) : null}
        <DialogFooter>
          <Button disabled={deleting} variant="outline" onClick={cancel}>
            Cancel
          </Button>
          <Button disabled={deleting} variant="destructive" onClick={submit}>
            {deleting ? 'Deleting…' : 'Delete'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
