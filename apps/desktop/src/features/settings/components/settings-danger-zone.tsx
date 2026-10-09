import type { ChangeEvent } from 'react';
import {
  AlertDialog as Dialog,
  AlertDialogContent as DialogContent,
  AlertDialogDescription as DialogDescription,
  AlertDialogFooter as DialogFooter,
  AlertDialogHeader as DialogHeader,
  AlertDialogTitle as DialogTitle,
} from '@/components/ui/alert-dialog';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { formatStorageSize } from '@/features/settings/settings-format';
import {
  maintenanceConfirmations,
  maintenanceLabels,
  useMaintenanceController,
} from '../use-maintenance-controller';

export interface SettingsDangerZoneProps {
  disabled?: boolean;
}
function maintenancePhaseLabel(phase: string): string {
  switch (phase) {
    case 'files':
      return 'Removing files';
    case 'database':
      return 'Clearing records';
    case 'compact-database':
      return 'Compacting the local database';
    case 'awaiting-theme-reset':
      return 'Appearance reset pending';
    case 'completed':
      return 'Finished';
    default:
      return 'Needs attention';
  }
}
export function SettingsDangerZone({
  disabled = false,
}: SettingsDangerZoneProps) {
  const controller = useMaintenanceController();
  function resetPreferences() {
    void controller.open('reset-preferences');
  }
  function clearLibrary() {
    void controller.open('clear-library');
  }
  function factoryReset() {
    void controller.open('factory-reset');
  }
  function changeConfirmation(event: ChangeEvent<HTMLInputElement>) {
    controller.setConfirmation(event.target.value);
  }
  function execute() {
    void controller.execute();
  }
  function retry() {
    void controller.retryRecovery();
  }
  function cancel() {
    controller.close(false);
  }
  const action = controller.dialog?.action;
  const blocked = disabled || controller.pending;
  const typed = action !== undefined && action !== 'reset-preferences';
  const confirmationMatches =
    action !== undefined &&
    (!typed || controller.confirmation === maintenanceConfirmations[action]);
  return (
    <section
      aria-labelledby="settings-danger-zone"
      className="border-destructive/30 mt-6 border-t pt-4"
    >
      <h2 id="settings-danger-zone" className="text-sm font-medium">
        Danger zone
      </h2>
      <p className="text-muted-foreground mt-1 text-xs">
        Stop recording before continuing. These actions stop background work.
        Operating system permission grants remain unchanged.
      </p>
      <div className="mt-3 grid gap-3">
        <div>
          <Button
            disabled={
              blocked ||
              (controller.recovery !== null &&
                controller.recovery.action !== 'reset-preferences')
            }
            variant="outline"
            onClick={resetPreferences}
          >
            Reset preferences
          </Button>
          <p className="text-muted-foreground mt-1 text-xs">
            Restore settings, capture scenes and appearance to defaults, and
            remove API keys. Keep recordings, user edits and downloaded models.
            Automatic storage cleanup stays paused until you explicitly save
            settings.
          </p>
        </div>
        <div>
          <Button
            disabled={
              blocked ||
              (controller.recovery !== null &&
                controller.recovery.action !== 'clear-library')
            }
            variant="destructive"
            onClick={clearLibrary}
          >
            Clear library
          </Button>
          <p className="text-muted-foreground mt-1 text-xs">
            Permanently remove all recordings, derived data, search vectors,
            days, jobs, tasks, notes, feedback and corrections. Keep settings,
            API keys and downloaded models.
          </p>
        </div>
        <div>
          <Button
            disabled={
              blocked ||
              (controller.recovery !== null &&
                controller.recovery.action !== 'factory-reset')
            }
            variant="destructive"
            onClick={factoryReset}
          >
            Factory reset
          </Button>
          <p className="text-muted-foreground mt-1 text-xs">
            Remove all recordings, saved configuration, model files and partial
            downloads, API keys, capture scenes, restore tokens and logs.
            Restore defaults and restart after successful cleanup.
          </p>
        </div>
      </div>
      {controller.recovery ? (
        <div className="mt-3" role="alert">
          <p>
            Cleanup is incomplete (
            {maintenancePhaseLabel(controller.recovery.phase)}). New work is
            blocked until cleanup finishes.
          </p>
          {controller.recovery.error ? (
            <p className="text-destructive mt-1 text-xs">
              {controller.recovery.error}
            </p>
          ) : null}
          <Button
            disabled={controller.pending}
            variant="outline"
            onClick={retry}
          >
            Retry {maintenanceLabels[controller.recovery.action].toLowerCase()}
          </Button>
        </div>
      ) : null}
      {controller.message ? (
        <p role="status" className="mt-3 text-xs">
          {controller.message}
        </p>
      ) : null}
      {controller.error && !controller.dialog ? (
        <p role="alert" className="text-destructive mt-3 text-xs">
          {controller.error}
        </p>
      ) : null}
      <Dialog open={controller.dialog !== null} onOpenChange={controller.close}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {action ? maintenanceLabels[action] : 'Maintenance'}
            </DialogTitle>
            <DialogDescription>
              {controller.dialog?.step === 'confirm'
                ? 'Confirm the reviewed action. Deleted files cannot be restored by Daydream.'
                : 'Review the impact before continuing. No data has been changed.'}
            </DialogDescription>
          </DialogHeader>
          {controller.dialog?.preview ? (
            <dl className="grid grid-cols-2 gap-2 text-sm">
              <dt>Recordings</dt>
              <dd>{controller.dialog.preview.recordingCount}</dd>
              <dt>Library rows</dt>
              <dd>{controller.dialog.preview.libraryRowCount}</dd>
              <dt>Files to remove</dt>
              <dd>{controller.dialog.preview.fileCount}</dd>
              <dt>Storage to remove</dt>
              <dd>{formatStorageSize(controller.dialog.preview.sizeBytes)}</dd>
              {action === 'factory-reset' ? (
                <>
                  <dt>Cached models</dt>
                  <dd>{controller.dialog.preview.modelCount}</dd>
                </>
              ) : null}
            </dl>
          ) : (
            <p role="status">Checking cleanup impact…</p>
          )}
          {action === 'reset-preferences' ? (
            <p className="text-xs">
              The library counts above are preserved. Preference reset does not
              remove recordings or models.
            </p>
          ) : null}
          {controller.dialog?.step === 'confirm' && typed && action ? (
            <div className="grid gap-2">
              <Label htmlFor="maintenance-confirmation">
                Type {maintenanceConfirmations[action]}
              </Label>
              <Input
                id="maintenance-confirmation"
                value={controller.confirmation}
                onChange={changeConfirmation}
                disabled={controller.pending}
                autoComplete="off"
              />
            </div>
          ) : null}
          {controller.error ? (
            <p role="alert" className="text-destructive text-sm">
              {controller.error}
            </p>
          ) : null}
          <DialogFooter>
            <Button
              variant="outline"
              disabled={controller.pending}
              onClick={cancel}
            >
              Cancel
            </Button>
            {controller.dialog?.step === 'review' ? (
              <Button
                disabled={
                  controller.pending || controller.dialog.preview === null
                }
                onClick={controller.continueToConfirmation}
              >
                Continue
              </Button>
            ) : (
              <Button
                variant="destructive"
                disabled={controller.pending || !confirmationMatches}
                onClick={execute}
              >
                {controller.pending ? 'Working…' : 'Confirm'}
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </section>
  );
}
