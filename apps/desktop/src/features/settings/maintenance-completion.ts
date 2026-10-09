import type { MaintenanceAction } from '@daydream/shared';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import { useBackgroundTaskStore } from '@/features/processing/background-task-store';
import { useRecordingProcessingStore } from '@/features/processing/recording-processing-store';
import { useRecorderWorkspaceStore } from '@/features/recorder/recorder-workspace-store';
import { useSettingsStore } from './settings-store';

/** Fence old IPC results as soon as native cleanup has committed. */
export function invalidateMaintenanceContent(action: MaintenanceAction): void {
  if (action === 'clear-library') {
    useBackgroundTaskStore.getState().invalidateLibrary();
    useRecordingProcessingStore.getState().invalidateLibrary();
    useNotificationsStore.getState().invalidateLibrary();
  } else if (action === 'reset-preferences') {
    useSettingsStore.getState().invalidatePreferences();
    useRecorderWorkspaceStore.getState().invalidatePreferences();
  }
}

/** Preference reads wait until the browser theme acknowledgement is complete. */
export async function refreshAfterMaintenance(
  action: MaintenanceAction,
): Promise<void> {
  if (action === 'clear-library') {
    await useNotificationsStore.getState().load();
  } else if (action === 'reset-preferences') {
    await useSettingsStore.getState().load();
    await useRecorderWorkspaceStore
      .getState()
      .reloadPreferences(useSettingsStore.getState().savedScene);
  }
}

export function maintenanceErrorMessage(cause: unknown): string {
  return typeof cause === 'string' && cause.trim().length > 0
    ? cause
    : 'Maintenance could not finish. Please try again.';
}
