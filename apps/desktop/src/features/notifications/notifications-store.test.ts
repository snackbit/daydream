import type { Notification } from '@daydream/shared';
import { beforeEach, expect, test, vi } from 'vitest';
import { useNotificationsStore } from './notifications-store';

const client = vi.hoisted(() => ({
  acknowledgeNotifications: vi.fn(),
  deleteAllNotifications: vi.fn(),
  deleteNotifications: vi.fn(),
  dismissNotification: vi.fn(),
  executeNotificationAction: vi.fn(),
  getNotifications: vi.fn(),
}));

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
  useNotificationsStore.setState({
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

  expect(client.executeNotificationAction).toHaveBeenCalledWith({
    actionId: 'retry',
    notificationId: 'notification-1',
  });
  expect(useNotificationsStore.getState().notifications[0]?.id).toBe(
    'notification-2',
  );

  await useNotificationsStore.getState().acknowledge(['notification-2']);

  expect(client.acknowledgeNotifications).toHaveBeenCalledWith({
    notificationIds: ['notification-2'],
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
    error: null,
    notifications: [notification],
    status: 'ready',
  });

  await useNotificationsStore.getState().deleteAll();

  expect(client.deleteAllNotifications).toHaveBeenCalled();
  expect(useNotificationsStore.getState().notifications).toEqual([]);
});

test('notifications store can acknowledge action results shown inside notification UI', async () => {
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
