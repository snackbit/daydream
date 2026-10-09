import { Button } from '@/components/ui/button';
import {
  Dialog,
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
  saving: boolean;
  validationError: string | null;
}

export interface SettingsScreenDialogsActions {
  confirmCleanup: () => Promise<void>;
  discardAndLeaveSettings: () => void;
  dismissCleanupConfirmation: () => void;
  resetLeaveBlocker: () => void;
  saveAndLeaveSettings: () => Promise<void>;
}

export interface SettingsScreenDialogsProps {
  actions: SettingsScreenDialogsActions;
  model: SettingsScreenDialogsModel;
}

export function SettingsScreenDialogs({
  actions,
  model,
}: SettingsScreenDialogsProps) {
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
              Delete and save
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
