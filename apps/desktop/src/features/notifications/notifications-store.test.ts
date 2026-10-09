import type {
  AiModelAsset,
  Notification,
  NotificationList,
} from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import {
  createNotificationsStore,
  useNotificationsStore,
} from './notifications-store';

const client = vi.hoisted(() => ({
  acknowledgeNotifications: vi.fn(),
  deleteAllNotifications: vi.fn(),
  deleteNotifications: vi.fn(),
  dismissNotification: vi.fn(),
  executeNotificationAction: vi.fn(),
  getNotifications: vi.fn(),
}));

const aiClient = vi.hoisted(() => ({
  cancelAiModelDownload: vi.fn(),
  downloadAiModel: vi.fn(),
  listAiModels: vi.fn(),
}));

vi.mock('@/features/ai/ai-client', () => aiClient);
vi.mock('./notifications-client', () => client);

const notification: Notification = {
  actions: [
    {
      handler: 'model-download-retry',
      id: 'retry',
      label: 'Retry',
      payload: {
        modelId: 'small',
      },
      style: 'secondary',
    },
  ],
  createdAtMs: 1,
  dismissedAtMs: null,
  id: 'notification-1',
  message: 'Download unavailable.',
  source: 'ai',
  status: 'failed',
  title: 'Download unavailable',
  topic: 'model-download',
  updatedAtMs: 1,
};

test('library invalidation prevents delayed notification reads restoring cleared text', async () => {
  let finish: ((list: NotificationList) => void) | undefined;
  client.getNotifications
    .mockReturnValueOnce(
      new Promise<NotificationList>((resolve) => {
        finish = resolve;
      }),
    )
    .mockResolvedValueOnce({ notifications: [] });
  const store = createNotificationsStore();
  store.setState({ notifications: [notification] });
  const old = store.getState().load();
  store.getState().invalidateLibrary();
  await store.getState().load();
  finish?.({ notifications: [notification] });
  await old;
  expect(store.getState().notifications).toEqual([]);
  expect(store.getState().status).toBe('ready');
});

const activeModel: AiModelAsset = {
  checksum: null,
  checksumAlgorithm: null,
  createdAtMs: 1,
  downloadedBytes: 10,
  downloadStatus: 'downloading',
  etag: null,
  errorCode: null,
  id: 'small',
  label: 'Whisper small',
  localPath: null,
  provider: 'local-whisper-cpp',
  selfTestStatus: 'not-run',
  sizeBytes: 100,
  sourceUrl: null,
  totalBytes: 100,
  updatedAtMs: 1,
  verificationStatus: 'pending',
};

const ongoingNotification: Notification = {
  ...notification,
  actions: [
    {
      handler: 'model-download-cancel',
      id: 'cancel',
      label: 'Cancel',
      payload: {
        modelId: 'small',
      },
      style: 'destructive',
    },
  ],
  status: 'running',
  title: 'Model download started',
};

beforeEach(() => {
  vi.resetAllMocks();
  client.getNotifications.mockResolvedValue({ notifications: [notification] });
  client.acknowledgeNotifications.mockResolvedValue({
    notifications: [
      {
        ...notification,
        dismissedAtMs: 2,
      },
    ],
  });
  client.dismissNotification.mockResolvedValue({ notifications: [] });
  client.deleteNotifications.mockResolvedValue({ notifications: [] });
  client.deleteAllNotifications.mockResolvedValue({ notifications: [] });
  client.executeNotificationAction.mockResolvedValue({
    notifications: [
      {
        ...notification,
        id: 'notification-2',
      },
    ],
  });
  aiClient.downloadAiModel.mockResolvedValue(notification);
  aiClient.cancelAiModelDownload.mockResolvedValue(notification);
  aiClient.listAiModels.mockResolvedValue({ models: [] });
  useNotificationsStore.setState({
    aiModels: [],
    error: null,
    notifications: [],
    status: 'idle',
  });
});

test('notifications store loads and refreshes after actions', async () => {
  await useNotificationsStore.getState().load();

  expect(useNotificationsStore.getState().notifications).toEqual([
    notification,
  ]);

  await useNotificationsStore
    .getState()
    .executeAction('notification-1', 'retry');

  expect(aiClient.downloadAiModel).toHaveBeenCalledWith({
    modelId: 'small',
    notificationId: 'notification-1',
  });
  expect(client.getNotifications).toHaveBeenCalledTimes(2);
  expect(useNotificationsStore.getState().notifications[0]?.id).toBe(
    'notification-1',
  );

  await useNotificationsStore.getState().acknowledge(['notification-1']);

  expect(client.acknowledgeNotifications).toHaveBeenCalledWith({
    notificationIds: ['notification-1'],
  });
  expect(useNotificationsStore.getState().notifications[0]?.dismissedAtMs).toBe(
    2,
  );

  await useNotificationsStore.getState().dismiss('notification-2');

  expect(client.dismissNotification).toHaveBeenCalledWith({
    notificationId: 'notification-2',
  });
  expect(useNotificationsStore.getState().notifications).toEqual([]);
});

test('notifications store permanently deletes notifications', async () => {
  await useNotificationsStore.getState().load();

  await useNotificationsStore
    .getState()
    .deleteNotifications(['notification-1']);

  expect(client.deleteNotifications).toHaveBeenCalledWith({
    notificationIds: ['notification-1'],
  });
  expect(useNotificationsStore.getState().notifications).toEqual([]);

  useNotificationsStore.setState({
    aiModels: [],
    error: null,
    notifications: [notification],
    status: 'ready',
  });

  await useNotificationsStore.getState().deleteAll();

  expect(client.deleteNotifications).toHaveBeenLastCalledWith({
    notificationIds: ['notification-1'],
  });
  expect(useNotificationsStore.getState().notifications).toEqual([]);
});

test('notifications store keeps ongoing work unread and undeletable', async () => {
  useNotificationsStore.setState({
    aiModels: [activeModel],
    error: null,
    notifications: [ongoingNotification],
    status: 'ready',
  });

  await useNotificationsStore.getState().acknowledge(['notification-1']);
  await useNotificationsStore.getState().dismiss('notification-1');
  await useNotificationsStore
    .getState()
    .deleteNotifications(['notification-1']);
  await useNotificationsStore.getState().deleteAll();

  expect(client.acknowledgeNotifications).not.toHaveBeenCalled();
  expect(client.dismissNotification).not.toHaveBeenCalled();
  expect(client.deleteNotifications).not.toHaveBeenCalled();
});

test('notifications store protects queued downloads before models load', async () => {
  useNotificationsStore.setState({
    aiModels: [],
    error: null,
    notifications: [
      {
        ...ongoingNotification,
        status: 'info',
        title: 'Model download queued',
      },
    ],
    status: 'ready',
  });

  await useNotificationsStore.getState().acknowledge(['notification-1']);
  await useNotificationsStore.getState().dismiss('notification-1');
  await useNotificationsStore
    .getState()
    .deleteNotifications(['notification-1']);
  await useNotificationsStore.getState().deleteAll();

  expect(client.acknowledgeNotifications).not.toHaveBeenCalled();
  expect(client.dismissNotification).not.toHaveBeenCalled();
  expect(client.deleteNotifications).not.toHaveBeenCalled();
});

test('notifications store can acknowledge action results shown inside notification UI', async () => {
  const jobNotification: Notification = {
    ...notification,
    actions: [
      {
        handler: 'processing-job-retry',
        id: 'retry',
        label: 'Retry',
        payload: {
          jobId: 'job-1',
        },
        style: 'secondary',
      },
    ],
  };
  client.executeNotificationAction.mockResolvedValueOnce({
    notifications: [
      {
        ...notification,
        dismissedAtMs: 2,
        id: 'notification-1',
      },
      {
        ...notification,
        dismissedAtMs: null,
        id: 'notification-retry-result',
      },
    ],
  });
  client.acknowledgeNotifications.mockResolvedValueOnce({
    notifications: [
      {
        ...notification,
        dismissedAtMs: 2,
        id: 'notification-1',
      },
      {
        ...notification,
        dismissedAtMs: 3,
        id: 'notification-retry-result',
      },
    ],
  });
  useNotificationsStore.setState({
    error: null,
    notifications: [jobNotification],
    status: 'ready',
  });

  await useNotificationsStore
    .getState()
    .executeAction('notification-1', 'retry', {
      acknowledgeResult: true,
    });

  expect(client.executeNotificationAction).toHaveBeenCalledWith({
    actionId: 'retry',
    notificationId: 'notification-1',
  });
  expect(client.acknowledgeNotifications).toHaveBeenCalledWith({
    notificationIds: ['notification-retry-result'],
  });
  expect(
    useNotificationsStore
      .getState()
      .notifications.find(
        (candidate) => candidate.id === 'notification-retry-result',
      )?.dismissedAtMs,
  ).toBe(3);
});
