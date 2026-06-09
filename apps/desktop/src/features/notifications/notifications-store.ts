import type { Notification } from '@daydream/shared';
import { create } from 'zustand';
import {
  runAsyncStoreAction,
  unknownErrorMessage,
} from '@/lib/async-store-action';
import {
  acknowledgeNotifications,
  deleteAllNotifications,
  deleteNotifications as deleteNotificationsClient,
  dismissNotification,
  executeNotificationAction,
  getNotifications,
} from './notifications-client';

type AsyncStatus = 'error' | 'idle' | 'loading' | 'ready';

interface ExecuteNotificationActionOptions {
  acknowledgeResult?: boolean;
}

interface NotificationsState {
  acknowledge: (notificationIds: string[]) => Promise<void>;
  deleteAll: () => Promise<void>;
  deleteNotifications: (notificationIds: string[]) => Promise<void>;
  error: string | null;
  executeAction: (
    notificationId: string,
    actionId: string,
    options?: ExecuteNotificationActionOptions,
  ) => Promise<void>;
  dismiss: (notificationId: string) => Promise<void>;
  load: () => Promise<void>;
  notifications: Notification[];
  status: AsyncStatus;
}

export const useNotificationsStore = create<NotificationsState>((set) => ({
  async acknowledge(notificationIds) {
    if (notificationIds.length === 0) {
      return;
    }

    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: () => acknowledgeNotifications({ notificationIds }),
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  async deleteAll() {
    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: deleteAllNotifications,
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  async deleteNotifications(notificationIds) {
    if (notificationIds.length === 0) {
      return;
    }

    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: () => deleteNotificationsClient({ notificationIds }),
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  error: null,
  async dismiss(notificationId) {
    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: () => dismissNotification({ notificationId }),
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  async executeAction(notificationId, actionId, options) {
    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: async () => {
        const result = await executeNotificationAction({
          actionId,
          notificationId,
        });

        if (!options?.acknowledgeResult) {
          return result;
        }

        const unseenIds = result.notifications
          .filter((notification) => notification.dismissedAtMs === null)
          .map((notification) => notification.id);

        if (unseenIds.length === 0) {
          return result;
        }

        return acknowledgeNotifications({ notificationIds: unseenIds });
      },
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  async load() {
    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: getNotifications,
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  notifications: [],
  status: 'idle',
}));

function notificationErrorMessage(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'message' in error &&
    typeof error.message === 'string'
  ) {
    return error.message;
  }

  return unknownErrorMessage(error);
}
