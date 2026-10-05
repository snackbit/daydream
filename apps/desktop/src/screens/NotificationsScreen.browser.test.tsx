import type { AiModelAsset, Notification } from '@daydream/shared';
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

const aiClient = vi.hoisted(() => ({
  cancelAiModelDownload: vi.fn(),
  downloadAiModel: vi.fn(),
  listAiModels: vi.fn(),
  listenToAiModelsChanged: vi.fn(),
}));

vi.mock('@/features/ai/ai-client', () => aiClient);
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

const activeModel: AiModelAsset = {
  checksum: null,
  checksumAlgorithm: null,
  createdAtMs: todayMs,
  downloadedBytes: 10,
  downloadStatus: 'downloading',
  etag: null,
  errorCode: null,
  id: 'tiny',
  label: 'Whisper tiny',
  localPath: null,
  provider: 'local-whisper-cpp',
  selfTestStatus: 'not-run',
  sizeBytes: 100,
  sourceUrl: null,
  totalBytes: 100,
  updatedAtMs: todayMs,
  verificationStatus: 'pending',
};

const activeNotification: Notification = {
  ...seenNotification,
  actions: [
    {
      handler: 'model-download-cancel',
      id: 'cancel',
      label: 'Cancel',
      payload: {
        modelId: 'tiny',
      },
      style: 'destructive',
    },
  ],
  createdAtMs: todayMs,
  dismissedAtMs: null,
  id: 'notification-active',
  message: 'Whisper tiny is downloading.',
  status: 'running',
  title: 'Model download started',
  updatedAtMs: todayMs,
};

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date('2026-06-09T12:00:00'));
  vi.resetAllMocks();
  client.deleteAllNotifications.mockResolvedValue({ notifications: [] });
  client.deleteNotifications.mockResolvedValue({
    notifications: [unseenNotification],
  });
  aiClient.downloadAiModel.mockResolvedValue(seenNotification);
  aiClient.cancelAiModelDownload.mockResolvedValue(seenNotification);
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
  vi.useRealTimers();
});

test('NotificationsScreen groups all notifications by local date', () => {
  useNotificationsStore.setState({
    aiModels: [],
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
  expect(screen.getByText('Model Download')).toBeTruthy();
});

test('NotificationsScreen permanently dismisses one or all notifications', async () => {
  vi.useRealTimers();
  useNotificationsStore.setState({
    aiModels: [],
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

  fireEvent.click(screen.getByRole('button', { name: 'Clear 1 update' }));

  await waitFor(() =>
    expect(client.deleteNotifications).toHaveBeenLastCalledWith({
      notificationIds: ['notification-unseen'],
    }),
  );
});

test('NotificationsScreen acknowledges retry results while the history is open', async () => {
  vi.useRealTimers();
  const retryResult = {
    ...seenNotification,
    createdAtMs: todayMs,
    dismissedAtMs: null,
    id: 'notification-retry-result',
    title: 'Retry result',
    updatedAtMs: todayMs,
  };
  client.getNotifications.mockResolvedValue({
    notifications: [seenNotification, retryResult],
  });
  client.acknowledgeNotifications.mockResolvedValue({
    notifications: [
      seenNotification,
      {
        ...retryResult,
        dismissedAtMs: todayMs + 1_000,
      },
    ],
  });
  useNotificationsStore.setState({
    aiModels: [],
    error: null,
    notifications: [seenNotification],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  fireEvent.click(screen.getByRole('button', { name: 'Retry' }));

  await waitFor(() =>
    expect(aiClient.downloadAiModel).toHaveBeenCalledWith({
      modelId: 'tiny',
      notificationId: 'notification-seen',
    }),
  );
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
    aiModels: [],
    error: null,
    notifications: [],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  expect(screen.getByRole('status').textContent).toContain('All caught up');
});

test('NotificationsScreen retries a failed history load and shows recovered updates', async () => {
  vi.useRealTimers();
  client.getNotifications
    .mockRejectedValueOnce(new Error('Notifications are unavailable.'))
    .mockResolvedValueOnce({ notifications: [unseenNotification] });
  useNotificationsStore.setState({
    aiModels: [],
    error: null,
    notifications: [],
    status: 'idle',
  });

  render(<NotificationsScreen />);

  expect(
    await screen.findByText('Could not update notifications'),
  ).toBeTruthy();
  fireEvent.click(screen.getByRole('button', { name: 'Try again' }));

  expect(await screen.findByText('New update')).toBeTruthy();
  await waitFor(() => expect(client.getNotifications).toHaveBeenCalledTimes(2));
});

test('NotificationsScreen keeps ongoing downloads unread and undeletable', async () => {
  vi.useRealTimers();
  useNotificationsStore.setState({
    aiModels: [activeModel],
    error: null,
    notifications: [activeNotification],
    status: 'ready',
  });

  render(<NotificationsScreen />);

  expect(screen.getByText('Model download started')).toBeTruthy();
  expect(screen.getByText('New')).toBeTruthy();
  expect(
    screen.queryByRole('button', {
      name: /permanently dismiss model download started/i,
    }),
  ).toBeNull();
  const dismissAllButton = screen.getByRole('button', {
    name: 'No updates to clear',
  }) as HTMLButtonElement;
  expect(dismissAllButton.disabled).toBe(true);
});
