import type {
  AiModelAsset,
  AiModelAssetList,
  Notification,
} from '@daydream/shared';
import { create } from 'zustand';
import {
  cancelAiModelDownload,
  downloadAiModel,
  listAiModels,
} from '@/features/ai/ai-client';
import {
  runAsyncStoreAction,
  unknownErrorMessage,
} from '@/lib/async-store-action';
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
  notifications: Notification[];
  refreshAiModels: () => Promise<void>;
  status: AsyncStatus;
  updateAiModels: (models: AiModelAssetList) => void;
}

export const useNotificationsStore = create<NotificationsState>((set) => ({
  async acknowledge(notificationIds) {
    const state = useNotificationsStore.getState();
    const dismissableIds = new Set(
      dismissableNotifications(state.notifications, state.aiModels).map(
        (notification) => notification.id,
      ),
    );
    const eligibleIds = notificationIds.filter((id) => dismissableIds.has(id));

    if (eligibleIds.length === 0) {
      return;
    }

    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: () => acknowledgeNotifications({ notificationIds: eligibleIds }),
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  aiModels: [],
  async deleteAll() {
    const state = useNotificationsStore.getState();
    const deletableIds = dismissableNotifications(
      state.notifications,
      state.aiModels,
    ).map((notification) => notification.id);

    if (deletableIds.length === 0) {
      return;
    }

    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: () => deleteNotificationsClient({ notificationIds: deletableIds }),
      success: (result) =>
        set({
          error: null,
          notifications: result.notifications,
          status: 'ready',
        }),
    });
  },
  async deleteNotifications(notificationIds) {
    const state = useNotificationsStore.getState();
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

    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: notificationErrorMessage(error),
          status: 'error',
        }),
      run: () => deleteNotificationsClient({ notificationIds: eligibleIds }),
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
    const state = useNotificationsStore.getState();
    const notification = state.notifications.find(
      (candidate) => candidate.id === notificationId,
    );

    if (notification && isNotificationOngoing(notification, state.aiModels)) {
      return;
    }

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
        const action = useNotificationsStore
          .getState()
          .notifications.find(
            (notification) => notification.id === notificationId,
          )
          ?.actions.find((candidate) => candidate.id === actionId);
        const modelId = action?.payload.modelId;
        const result =
          action?.handler === 'model-download-retry' && modelId
            ? await downloadAiModel({ modelId, notificationId }).then(
                getNotifications,
              )
            : action?.handler === 'model-download-cancel' && modelId
              ? await cancelAiModelDownload({
                  modelId,
                  notificationId,
                }).then(getNotifications)
              : await executeNotificationAction({
                  actionId,
                  notificationId,
                });

        if (!options?.acknowledgeResult) {
          return result;
        }

        const unseenIds = result.notifications
          .filter(
            (notification) =>
              notification.dismissedAtMs === null &&
              !isNotificationOngoing(
                notification,
                useNotificationsStore.getState().aiModels,
              ),
          )
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
