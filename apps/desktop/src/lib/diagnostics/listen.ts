import {
  listen as nativeListen,
  type EventCallback,
  type EventName,
  type Options,
  type UnlistenFn,
} from '@tauri-apps/api/event';
import { reportFrontendFailure } from './failure-logging';

export async function listen<T>(
  event: EventName,
  handler: EventCallback<T>,
  ...options: [options?: Options]
): Promise<UnlistenFn> {
  try {
    return await nativeListen<T>(
      event,
      (received) => {
        try {
          handler(received);
        } catch (error) {
          reportFrontendFailure(event, 'callback', error);
          throw error;
        }
      },
      ...options,
    );
  } catch (error) {
    reportFrontendFailure(event, 'subscribe', error, { deduplicate: false });
    throw error;
  }
}
