import type React from 'react';
import {
  cleanup,
  fireEvent,
  render,
  screen,
  waitFor,
} from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { useNotificationsStore } from '../notifications-store';
import { NotificationCenter } from './notification-center';

const client = vi.hoisted(() => ({
  acknowledgeNotifications: vi.fn(),
  deleteAllNotifications: vi.fn(),
  deleteNotifications: vi.fn(),
  dismissNotification: vi.fn(),
  executeNotificationAction: vi.fn(),
  getNotifications: vi.fn(),
}));

vi.mock('../notifications-client', () => client);
vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    onClick,
    to,
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    to: string;
  }) => (
    <a href={to} onClick={onClick}>
      {children}
    </a>
  ),
}));

const notification = {
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
  message: 'Model download is not implemented yet.',
  source: 'ai',
  status: 'failed',
  title: 'Download unavailable',
  topic: 'model-download',
  updatedAtMs: 1,
} as const;

const succeededNotification = {
  ...notification,
  actions: [],
  id: 'notification-succeeded',
  message: 'Transcription finished.',
  status: 'succeeded',
  title: 'Processing complete',
  topic: 'processing-job',
} as const;

const seenNotification = {
  ...notification,
  dismissedAtMs: 2,
  id: 'notification-seen',
} as const;

beforeEach(() => {
  vi.resetAllMocks();
  client.getNotifications.mockResolvedValue({
    notifications: [notification],
  });
  client.acknowledgeNotifications.mockResolvedValue({
    notifications: [
      {
        ...notification,
        dismissedAtMs: 2,
      },
    ],
  });
  client.executeNotificationAction.mockResolvedValue({
    notifications: [
      {
        ...notification,
        id: 'notification-2',
      },
    ],
  });
  client.dismissNotification.mockResolvedValue({
    notifications: [],
  });
  useNotificationsStore.setState({
    error: null,
    notifications: [],
    status: 'idle',
  });
});

afterEach(() => {
  cleanup();
});

test('NotificationCenter opens, runs an action, and dismisses notification', async () => {
  render(<NotificationCenter />);

  const trigger = await screen.findByRole('button', {
    name: /notifications, 1 unread/i,
  });
  fireEvent.click(trigger);

  expect(await screen.findByText('Download unavailable')).toBeTruthy();
  expect(
    screen.getByRole<HTMLAnchorElement>('link', { name: 'View all' }).href,
  ).toContain('/notifications');

  await waitFor(() =>
    expect(client.acknowledgeNotifications).toHaveBeenCalledWith({
      notificationIds: ['notification-1'],
    }),
  );
  expect(
    await screen.findByRole('button', {
      name: 'Notifications',
    }),
  ).toBeTruthy();
  expect(screen.getByText('Download unavailable')).toBeTruthy();

  fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

  await waitFor(() =>
    expect(client.executeNotificationAction).toHaveBeenCalledWith({
      actionId: 'retry',
      notificationId: 'notification-1',
    }),
  );

  fireEvent.click(
    await screen.findByRole('button', {
      name: /dismiss download unavailable/i,
    }),
  );

  await waitFor(() =>
    expect(client.acknowledgeNotifications).toHaveBeenLastCalledWith({
      notificationIds: ['notification-1'],
    }),
  );
});

test('NotificationCenter shows succeeded processing notifications', async () => {
  client.getNotifications.mockResolvedValueOnce({
    notifications: [succeededNotification],
  });

  render(<NotificationCenter />);

  const trigger = await screen.findByRole('button', {
    name: /notifications, 1 unread/i,
  });
  fireEvent.click(trigger);

  expect(await screen.findByText('Processing complete')).toBeTruthy();
  expect(screen.getByText('succeeded')).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Retry' })).toBeNull();
});

test('NotificationCenter shows older seen notification count when there is no new work', async () => {
  client.getNotifications.mockResolvedValueOnce({
    notifications: [seenNotification],
  });

  render(<NotificationCenter />);

  const trigger = await screen.findByRole('button', {
    name: 'Notifications',
  });
  fireEvent.click(trigger);

  expect(
    await screen.findByText(
      'No new notifications. 1 older seen notification is in history.',
    ),
  ).toBeTruthy();
});
