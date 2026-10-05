import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { SettingsLeaveDialog } from '@/features/settings/components/settings-leave-dialog';
import { formatStorageSize } from '@/features/settings/settings-format';
import { formatSessionCount } from '@/features/settings/settings-screen-utils';
import type { CleanupConfirmation } from '@/features/settings/use-settings-screen-controller';

export interface SettingsScreenDialogsModel {
  blockerStatus: 'blocked' | 'idle';
  busy: boolean;
  canSave: boolean;
  cleanupConfirmation: CleanupConfirmation | null;
  resetOpen: boolean;
  saving: boolean;
  validationError: string | null;
}

export interface SettingsScreenDialogsActions {
  confirmCleanup: () => Promise<void>;
  discardAndLeaveSettings: () => void;
  dismissCleanupConfirmation: () => void;
  requestReset: () => Promise<void>;
  resetLeaveBlocker: () => void;
  saveAndLeaveSettings: () => Promise<void>;
  setResetOpen: (open: boolean) => void;
}

export interface SettingsScreenDialogsProps {
  actions: SettingsScreenDialogsActions;
  model: SettingsScreenDialogsModel;
}

export function SettingsScreenDialogs({
  actions,
  model,
}: SettingsScreenDialogsProps) {
  function handleRequestReset() {
    void actions.requestReset();
  }

  function handleCleanupOpenChange(open: boolean) {
    if (!open) {
      actions.dismissCleanupConfirmation();
    }
  }

  function handleConfirmCleanup() {
    void actions.confirmCleanup();
  }

  function handleLeaveOpenChange(open: boolean) {
    if (!open && !model.busy) {
      actions.resetLeaveBlocker();
    }
  }

  function handleSaveAndLeave() {
    void actions.saveAndLeaveSettings();
  }

  return (
    <>
      <Dialog open={model.resetOpen} onOpenChange={actions.setResetOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Reset settings?</DialogTitle>
            <DialogDescription>
              This restores capture and storage settings to defaults. It does
              not delete recordings or scenes.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <DialogClose asChild>
              <Button disabled={model.busy} variant="outline">
                Cancel
              </Button>
            </DialogClose>
            <Button
              disabled={model.busy}
              onClick={handleRequestReset}
              variant="destructive"
            >
              Reset settings
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={Boolean(model.cleanupConfirmation)}
        onOpenChange={handleCleanupOpenChange}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete old recordings?</DialogTitle>
            <DialogDescription>
              This would delete{' '}
              {model.cleanupConfirmation
                ? formatSessionCount(
                    model.cleanupConfirmation.preview.recordingCount,
                  )
                : '0 sessions'}{' '}
              (
              {formatStorageSize(
                model.cleanupConfirmation?.preview.sizeBytes ?? 0,
              )}
              ) from the local library.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              disabled={model.saving}
              onClick={actions.dismissCleanupConfirmation}
              variant="outline"
            >
              Cancel
            </Button>
            <Button
              disabled={model.saving}
              onClick={handleConfirmCleanup}
              variant="destructive"
            >
              {model.cleanupConfirmation?.action === 'reset'
                ? 'Delete and reset'
                : 'Delete and save'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <SettingsLeaveDialog
        busy={model.busy}
        canSave={model.canSave}
        onDiscard={actions.discardAndLeaveSettings}
        onOpenChange={handleLeaveOpenChange}
        onSave={handleSaveAndLeave}
        onStay={actions.resetLeaveBlocker}
        open={model.blockerStatus === 'blocked' && !model.cleanupConfirmation}
        validationError={model.validationError}
      />
    </>
  );
}
