import type { Notification } from '@daydream/shared';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import { NotificationsScreen } from './NotificationsScreen';

const client = vi.hoisted(() => ({
  acknowledgeNotifications: vi.fn(),
  deleteAllNotifications: vi.fn(),
  deleteNotifications: vi.fn(),
  dismissNotification: vi.fn(),
  executeNotificationAction: vi.fn(),
  getNotifications: vi.fn(),
}));

vi.mock('@/features/notifications/notifications-client', () => client);

const todayMs = new Date('2026-06-09T10:00:00').getTime();
const yesterdayMs = new Date('2026-06-08T10:00:00').getTime();

const unseenNotification: Notification = {
  actions: [],
  createdAtMs: todayMs,
  dismissedAtMs: null,
  id: 'notification-unseen',
  message: 'A new local status update.',
  source: 'system',
  status: 'info',
  title: 'New update',
  topic: 'status',
  updatedAtMs: todayMs,
};

const seenNotification: Notification = {
  actions: [
    {
      handler: 'model-download-retry',
      id: 'retry',
      label: 'Retry',
      payload: {
        modelId: 'tiny',
      },
      style: 'secondary',
    },
  ],
  createdAtMs: yesterdayMs,
  dismissedAtMs: yesterdayMs + 1_000,
  id: 'notification-seen',
  message: 'An older local status update.',
  source: 'ai',
  status: 'failed',
  title: 'Older update',
  topic: 'model-download',
  updatedAtMs: yesterdayMs,
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-06-09T12:00:00'));
  vi.resetAllMocks();
  client.deleteAllNotifications.mockResolvedValue({ notifications: [] });
  client.deleteNotifications.mockResolvedValue({
    notifications: [unseenNotification],
  });
  useNotificationsStore.setState({
    error: null,
    notifications: [],
    status: 'idle',
  });
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
});

test('NotificationsScreen groups all notifications by local date', () => {
  useNotificationsStore.setState({
    error: null,
    notifications: [seenNotification, unseenNotification],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  expect(screen.getByRole('heading', { name: 'Today' })).toBeTruthy();
  expect(screen.getByRole('heading', { name: 'Yesterday' })).toBeTruthy();
  expect(screen.getByText('New update')).toBeTruthy();
  expect(screen.getByText('Older update')).toBeTruthy();
  expect(screen.getByText('New')).toBeTruthy();
  expect(screen.getByText('model-download')).toBeTruthy();
});

test('NotificationsScreen permanently dismisses one or all notifications', async () => {
  vi.useRealTimers();
  useNotificationsStore.setState({
    error: null,
    notifications: [seenNotification, unseenNotification],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  fireEvent.click(
    screen.getByRole('button', {
      name: /permanently dismiss older update/i,
    }),
  );

  await waitFor(() =>
    expect(client.deleteNotifications).toHaveBeenCalledWith({
      notificationIds: ['notification-seen'],
    }),
  );

  fireEvent.click(
    screen.getByRole('button', { name: 'Dismiss all notifications' }),
  );

  await waitFor(() => expect(client.deleteAllNotifications).toHaveBeenCalled());
});

test('NotificationsScreen acknowledges retry results while the history is open', async () => {
  vi.useRealTimers();
  client.executeNotificationAction.mockResolvedValue({
    notifications: [
      seenNotification,
      {
        ...seenNotification,
        createdAtMs: todayMs,
        dismissedAtMs: null,
        id: 'notification-retry-result',
        title: 'Retry result',
        updatedAtMs: todayMs,
      },
    ],
  });
  client.acknowledgeNotifications.mockResolvedValue({
    notifications: [
      seenNotification,
      {
        ...seenNotification,
        createdAtMs: todayMs,
        dismissedAtMs: todayMs + 1_000,
        id: 'notification-retry-result',
        title: 'Retry result',
        updatedAtMs: todayMs,
      },
    ],
  });
  useNotificationsStore.setState({
    error: null,
    notifications: [seenNotification],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

  await waitFor(() =>
    expect(client.acknowledgeNotifications).toHaveBeenCalledWith({
      notificationIds: ['notification-retry-result'],
    }),
  );
  expect(await screen.findByText('Retry result')).toBeTruthy();
  expect(screen.queryByText('New')).toBeNull();
});

test('NotificationsScreen shows an empty history state', () => {
  useNotificationsStore.setState({
    error: null,
    notifications: [],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  expect(screen.getByText('No notifications yet.')).toBeTruthy();
});
