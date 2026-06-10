import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type { AppStartupStatus } from '@daydream/shared';

export const appStartupStatusChangedEvent = 'app-startup-status-changed';

export async function getAppStartupStatus(): Promise<AppStartupStatus> {
  return invoke<AppStartupStatus>('getAppStartupStatus');
}

export async function retryAppStartup(): Promise<AppStartupStatus> {
  return invoke<AppStartupStatus>('retryAppStartup');
}

export async function listenToAppStartupStatusChanged(
  handler: (status: AppStartupStatus) => void,
): Promise<() => void> {
  return listen<AppStartupStatus>(appStartupStatusChangedEvent, (event) => {
    handler(event.payload);
  });
}
