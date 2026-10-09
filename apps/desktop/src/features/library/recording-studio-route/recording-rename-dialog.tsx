import { useState } from 'react';
import type { LibraryRecordingDetail } from '@daydream/shared';
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
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  errorMessage,
  RECORDING_TITLE_MAX_CHARS,
} from './recording-studio-route-utils';

export function RecordingRenameDialog({
  detail,
  onOpenChange,
  onSave,
  open,
}: {
  detail: LibraryRecordingDetail | null;
  onOpenChange: (open: boolean) => void;
  onSave: (title: string) => Promise<void>;
  open: boolean;
}) {
  const [renameSaving, setRenameSaving] = useState(false);
  const [renameError, setRenameError] = useState<string | null>(null);
  const [renameTitle, setRenameTitle] = useState(detail?.title ?? '');

  function close() {
    onOpenChange(false);
    setRenameTitle('');
  }

  async function save() {
    if (renameSaving) return;
    setRenameError(null);
    setRenameSaving(true);
    try {
      await onSave(renameTitle);
      close();
    } catch (error) {
      setRenameError(errorMessage(error));
    } finally {
      setRenameSaving(false);
    }
  }

  function submit() {
    void save();
  }

  function changeOpen(nextOpen: boolean) {
    if (renameSaving) return;
    onOpenChange(nextOpen);
    if (!nextOpen) setRenameTitle('');
  }

  return (
    <Dialog open={open} onOpenChange={changeOpen}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Rename recording</DialogTitle>
          <DialogDescription>
            Leave the title blank to use the recording timestamp.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-2">
          <Label htmlFor="studio-recording-title">Title</Label>
          <Input
            id="studio-recording-title"
            maxLength={RECORDING_TITLE_MAX_CHARS + 1}
            value={renameTitle}
            onChange={(event) => setRenameTitle(event.target.value)}
          />
          <div className="text-muted-foreground text-xs">
            {renameTitle.trim().length}/{RECORDING_TITLE_MAX_CHARS}
          </div>
        </div>
        {renameError ? (
          <Alert variant="destructive">
            <AlertDescription>{renameError}</AlertDescription>
          </Alert>
        ) : null}
        <DialogFooter>
          <Button disabled={renameSaving} variant="outline" onClick={close}>
            Cancel
          </Button>
          <Button
            disabled={
              renameSaving ||
              renameTitle.trim().length > RECORDING_TITLE_MAX_CHARS
            }
            onClick={submit}
          >
            {renameSaving ? 'Saving…' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
