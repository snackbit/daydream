import { useEffect, useRef, useState } from 'react';
import type {
  MaintenanceAction,
  MaintenancePreview,
  MaintenanceStatus,
} from '@daydream/shared';
import { resetAppearancePreference } from '@/features/appearance/appearance-store';
import {
  acknowledgeMaintenanceThemeReset,
  executeMaintenance,
  getMaintenanceStatus,
  previewMaintenance,
} from './maintenance-client';
import {
  invalidateMaintenanceContent,
  maintenanceErrorMessage,
  refreshAfterMaintenance,
} from './maintenance-completion';

export const maintenanceLabels: Record<MaintenanceAction, string> = {
  'reset-preferences': 'Reset preferences',
  'clear-library': 'Clear library',
  'factory-reset': 'Factory reset',
};
export const maintenanceConfirmations: Record<MaintenanceAction, string> = {
  'reset-preferences': 'RESET PREFERENCES',
  'clear-library': 'CLEAR LIBRARY',
  'factory-reset': 'FACTORY RESET',
};
type DialogState = {
  action: MaintenanceAction;
  step: 'review' | 'confirm';
  preview: MaintenancePreview | null;
} | null;
export function useMaintenanceController() {
  const [dialog, setDialog] = useState<DialogState>(null);
  const [pending, setPending] = useState(false);
  const [confirmation, setConfirmation] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [recovery, setRecovery] = useState<MaintenanceStatus | null>(null);
  const request = useRef(0);
  const inFlight = useRef(false);
  useEffect(() => {
    let current = true;
    void getMaintenanceStatus()
      .then((status) => {
        if (current) setRecovery(status);
      })
      .catch((cause: unknown) => {
        if (current) setError(maintenanceErrorMessage(cause));
      });
    return () => {
      current = false;
      request.current++;
    };
  }, []);
  async function open(action: MaintenanceAction) {
    if (inFlight.current) return;
    const id = ++request.current;
    setDialog({ action, step: 'review', preview: null });
    setConfirmation('');
    setError(null);
    setMessage(null);
    setPending(true);
    try {
      const preview = await previewMaintenance({ action });
      if (id === request.current)
        setDialog({ action, step: 'review', preview });
    } catch (cause: unknown) {
      if (id === request.current) setError(maintenanceErrorMessage(cause));
    } finally {
      if (id === request.current) setPending(false);
    }
  }
  function close(open: boolean) {
    if (open || inFlight.current) return;
    request.current++;
    setDialog(null);
    setPending(false);
    setConfirmation('');
    setError(null);
  }
  function continueToConfirmation() {
    if (dialog?.preview && !pending) setDialog({ ...dialog, step: 'confirm' });
  }
  async function acknowledge(status: MaintenanceStatus) {
    resetAppearancePreference();
    const completed = await acknowledgeMaintenanceThemeReset({
      operationId: status.operationId,
    });
    setRecovery(null);
    return completed;
  }
  async function execute() {
    if (!dialog?.preview || inFlight.current) return;
    const action = dialog.action;
    if (
      action !== 'reset-preferences' &&
      confirmation !== maintenanceConfirmations[action]
    )
      return;
    inFlight.current = true;
    setPending(true);
    setError(null);
    try {
      let status = await executeMaintenance({
        action,
        previewToken: dialog.preview.token,
        confirmation: maintenanceConfirmations[action],
      });
      setRecovery(status.phase === 'completed' ? null : status);
      invalidateMaintenanceContent(action);
      if (status.phase === 'awaiting-theme-reset')
        status = await acknowledge(status);
      if (action !== 'factory-reset') {
        await refreshAfterMaintenance(action);
        setMessage(`${maintenanceLabels[action]} completed.`);
      }
      setDialog(null);
    } catch (cause: unknown) {
      setError(maintenanceErrorMessage(cause));
      try {
        setRecovery(await getMaintenanceStatus());
      } catch {
        /* Primary error remains visible. */
      }
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }
  async function retryRecovery() {
    if (!recovery || inFlight.current) return;
    if (recovery.phase !== 'awaiting-theme-reset') {
      await open(recovery.action);
      return;
    }
    inFlight.current = true;
    setPending(true);
    setError(null);
    try {
      invalidateMaintenanceContent(recovery.action);
      await acknowledge(recovery);
      await refreshAfterMaintenance(recovery.action);
      setMessage(`${maintenanceLabels[recovery.action]} completed.`);
    } catch (cause: unknown) {
      setError(maintenanceErrorMessage(cause));
    } finally {
      inFlight.current = false;
      setPending(false);
    }
  }
  return {
    dialog,
    pending,
    confirmation,
    setConfirmation,
    error,
    message,
    recovery,
    open,
    close,
    continueToConfirmation,
    execute,
    retryRecovery,
  };
}
