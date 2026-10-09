import type {
  AiModelAsset,
  AiModelAssetList,
  Notification,
  NotificationList,
} from '@daydream/shared';
import { create } from 'zustand';
import { listAiModels } from '@/features/ai/ai-client';
import {
  runAsyncStoreAction,
  unknownErrorMessage,
} from '@/lib/async-store-action';
import { executeModelNotificationAction } from './model-notification-action';
import {
  dismissableNotifications,
  isNotificationOngoing,
} from './notification-lifecycle';
import {
  acknowledgeNotifications,
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
  aiModels: AiModelAsset[];
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
  invalidateLibrary: () => void;
  notifications: Notification[];
  refreshAiModels: () => Promise<void>;
  status: AsyncStatus;
  updateAiModels: (models: AiModelAssetList) => void;
}

export function createNotificationsStore() {
  let generation = 0;
  return create<NotificationsState>((set, get) => {
    async function requestNotifications(
      run: () => Promise<NotificationList>,
    ): Promise<void> {
      const ownedGeneration = generation;
      await runAsyncStoreAction({
        before: () => set({ error: null, status: 'loading' }),
        failure: (error) => {
          if (ownedGeneration !== generation) return false;
          set({ error: notificationErrorMessage(error), status: 'error' });
          return undefined;
        },
        run,
        success: (result) => {
          if (ownedGeneration === generation)
            set({
              error: null,
              notifications: result.notifications,
              status: 'ready',
            });
        },
      });
    }
    return {
      async acknowledge(notificationIds) {
        const state = get();
        const dismissableIds = new Set(
          dismissableNotifications(state.notifications, state.aiModels).map(
            (notification) => notification.id,
          ),
        );
        const eligibleIds = notificationIds.filter((id) =>
          dismissableIds.has(id),
        );

        if (eligibleIds.length === 0) {
          return;
        }

        await requestNotifications(() =>
          acknowledgeNotifications({ notificationIds: eligibleIds }),
        );
      },
      aiModels: [],
      async deleteAll() {
        const state = get();
        const deletableIds = dismissableNotifications(
          state.notifications,
          state.aiModels,
        ).map((notification) => notification.id);

        if (deletableIds.length === 0) {
          return;
        }

        await requestNotifications(() =>
          deleteNotificationsClient({ notificationIds: deletableIds }),
        );
      },
      async deleteNotifications(notificationIds) {
        const state = get();
        const eligibleIds = notificationIds.filter((id) => {
          const notification = state.notifications.find(
            (candidate) => candidate.id === id,
          );

          return notification
            ? !isNotificationOngoing(notification, state.aiModels)
            : false;
        });

        if (eligibleIds.length === 0) {
          return;
        }

        await requestNotifications(() =>
          deleteNotificationsClient({ notificationIds: eligibleIds }),
        );
      },
      error: null,
      async dismiss(notificationId) {
        const state = get();
        const notification = state.notifications.find(
          (candidate) => candidate.id === notificationId,
        );

        if (
          notification &&
          isNotificationOngoing(notification, state.aiModels)
        ) {
          return;
        }

        await requestNotifications(() =>
          dismissNotification({ notificationId }),
        );
      },
      async executeAction(notificationId, actionId, options) {
        const ownedGeneration = generation;
        await requestNotifications(async () => {
          const action = get()
            .notifications.find(
              (notification) => notification.id === notificationId,
            )
            ?.actions.find((candidate) => candidate.id === actionId);
          const result = (await executeModelNotificationAction(
            action,
            notificationId,
          ))
            ? await getNotifications()
            : await executeNotificationAction({
                actionId,
                notificationId,
              });

          if (ownedGeneration !== generation)
            throw new Error('Notification request was invalidated.');
          if (!options?.acknowledgeResult) {
            return result;
          }

          const unseenIds = result.notifications
            .filter(
              (notification) =>
                notification.dismissedAtMs === null &&
                !isNotificationOngoing(notification, get().aiModels),
            )
            .map((notification) => notification.id);

          if (unseenIds.length === 0) {
            return result;
          }

          return acknowledgeNotifications({ notificationIds: unseenIds });
        });
      },
      async load() {
        await requestNotifications(getNotifications);
      },
      notifications: [],
      invalidateLibrary() {
        generation++;
        set({ notifications: [], error: null, status: 'idle' });
      },
      async refreshAiModels() {
        try {
          const result = await listAiModels();
          set({ aiModels: result.models });
        } catch {
          // Notification progress is best-effort UI state.
        }
      },
      status: 'idle',
      updateAiModels(models) {
        set({ aiModels: models.models });
      },
    };
  });
}
export const useNotificationsStore = createNotificationsStore();

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
