import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

interface SettingsLeaveDialogProps {
  busy: boolean;
  canSave: boolean;
  onDiscard: () => void;
  onOpenChange: (open: boolean) => void;
  onSave: () => void;
  onStay: () => void;
  open: boolean;
  validationError: string | null;
}

export function SettingsLeaveDialog({
  busy,
  canSave,
  onDiscard,
  onOpenChange,
  onSave,
  onStay,
  open,
  validationError,
}: SettingsLeaveDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Do you want to save?</DialogTitle>
          <DialogDescription>
            Settings has unsaved changes. Save them before leaving, discard
            them, or stay on this page.
          </DialogDescription>
        </DialogHeader>
        {validationError ? (
          <p className="text-destructive text-xs" role="alert">
            {validationError}
          </p>
        ) : null}
        <DialogFooter>
          <Button disabled={busy} onClick={onStay} variant="outline">
            Stay
          </Button>
          <Button disabled={busy} onClick={onDiscard} variant="destructive">
            Discard changes
          </Button>
          <Button disabled={!canSave} onClick={onSave}>
            Save
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
