import { beforeEach, expect, test, vi } from 'vitest';
import {
  acknowledgeNotifications,
  deleteAllNotifications,
  deleteNotifications,
  dismissNotification,
  executeNotificationAction,
  getNotifications,
} from './notifications-client';

const { invokeMock } = vi.hoisted(() => ({
  invokeMock: vi.fn(),
}));

vi.mock('@tauri-apps/api/core', () => ({
  invoke: invokeMock,
}));

beforeEach(() => {
  invokeMock.mockReset();
  invokeMock.mockResolvedValue({ notifications: [] });
});

test('notifications client uses generic camelCase command names', async () => {
  await getNotifications();
  await acknowledgeNotifications({ notificationIds: ['notification-1'] });
  await dismissNotification({ notificationId: 'notification-1' });
  await deleteNotifications({ notificationIds: ['notification-1'] });
  await deleteAllNotifications();
  await executeNotificationAction({
    actionId: 'retry',
    notificationId: 'notification-1',
  });

  expect(invokeMock).toHaveBeenNthCalledWith(1, 'getNotifications');
  expect(invokeMock).toHaveBeenNthCalledWith(2, 'acknowledgeNotifications', {
    input: { notificationIds: ['notification-1'] },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(3, 'dismissNotification', {
    input: { notificationId: 'notification-1' },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(4, 'deleteNotifications', {
    input: { notificationIds: ['notification-1'] },
  });
  expect(invokeMock).toHaveBeenNthCalledWith(5, 'deleteAllNotifications');
  expect(invokeMock).toHaveBeenNthCalledWith(6, 'executeNotificationAction', {
    input: {
      actionId: 'retry',
      notificationId: 'notification-1',
    },
  });
});
