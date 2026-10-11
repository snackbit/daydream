import type { AppStartupStatus } from '@daydream/shared';
import { invoke } from '@/lib/diagnostics/invoke';
import { listen } from '@/lib/diagnostics/listen';

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
