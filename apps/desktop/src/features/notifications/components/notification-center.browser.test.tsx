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

const aiClient = vi.hoisted(() => ({
  cancelAiModelDownload: vi.fn(),
  downloadAiModel: vi.fn(),
  listAiModels: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
}));

vi.mock('@/features/ai/ai-client', () => aiClient);
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

const activeModel = {
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
} as const;

const activeNotification = {
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
  aiClient.downloadAiModel.mockResolvedValue(notification);
  aiClient.cancelAiModelDownload.mockResolvedValue(notification);
  aiClient.listAiModels.mockResolvedValue({ models: [] });
  aiClient.listenToAiModelsChanged.mockResolvedValue(() => undefined);
  useNotificationsStore.setState({
    aiModels: [],
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
    expect(aiClient.downloadAiModel).toHaveBeenCalledWith({
      modelId: 'small',
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

test('NotificationCenter keeps ongoing downloads unread and not dismissable', async () => {
  client.getNotifications.mockResolvedValueOnce({
    notifications: [activeNotification],
  });
  aiClient.listAiModels.mockResolvedValueOnce({ models: [activeModel] });
  useNotificationsStore.setState({
    aiModels: [activeModel],
    error: null,
    notifications: [],
    status: 'idle',
  });

  render(<NotificationCenter />);

  const trigger = await screen.findByRole('button', {
    name: /notifications, 1 unread/i,
  });
  fireEvent.click(trigger);

  expect(await screen.findByText('Model download started')).toBeTruthy();
  expect(client.acknowledgeNotifications).not.toHaveBeenCalled();
  expect(
    screen.queryByRole('button', {
      name: /dismiss model download started/i,
    }),
  ).toBeNull();
});

test('NotificationCenter keeps queued downloads cancellable before models load', async () => {
  client.getNotifications.mockResolvedValueOnce({
    notifications: [
      {
        ...activeNotification,
        status: 'info',
        title: 'Model download queued',
      },
    ],
  });
  aiClient.listAiModels.mockResolvedValueOnce({ models: [] });

  render(<NotificationCenter />);

  const trigger = await screen.findByRole('button', {
    name: /notifications, 1 unread/i,
  });
  fireEvent.click(trigger);

  expect(await screen.findByText('Model download queued')).toBeTruthy();
  expect(screen.getByRole('button', { name: 'Cancel' })).toBeTruthy();
  expect(screen.queryByRole('button', { name: 'Retry' })).toBeNull();
  expect(client.acknowledgeNotifications).not.toHaveBeenCalled();
  expect(
    screen.queryByRole('button', {
      name: /dismiss model download queued/i,
    }),
  ).toBeNull();
});
