import type { AiModelAsset, Notification } from '@daydream/shared';
import { expect, test } from 'vitest';
import {
  dismissableNotifications,
  isNotificationOngoing,
  visibleNotificationActions,
} from './notification-lifecycle';

const model: AiModelAsset = {
  checksum: null,
  checksumAlgorithm: null,
  createdAtMs: 1,
  downloadedBytes: 0,
  downloadStatus: 'cancelled',
  etag: null,
  errorCode: null,
  id: 'small',
  label: 'Whisper small',
  localPath: null,
  provider: 'local-whisper-cpp',
  selfTestStatus: 'not-run',
  sizeBytes: 100,
  sourceUrl: null,
  totalBytes: null,
  updatedAtMs: 1,
  verificationStatus: 'unknown',
};

const notification: Notification = {
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
  message: 'Model download state changed.',
  source: 'ai',
  status: 'running',
  title: 'Model download started',
  topic: 'model-download',
  updatedAtMs: 1,
};

test('visibleNotificationActions matches model download actions to current model state', () => {
  expect(
    visibleNotificationActions(notification, [
      {
        ...model,
        downloadStatus: 'downloading',
      },
    ]).map((action) => action.id),
  ).toEqual(['cancel']);

  expect(
    visibleNotificationActions(notification, [model]).map(
      (action) => action.id,
    ),
  ).toEqual(['retry']);
});

test('queued model download notifications stay ongoing before models load', () => {
  const queuedNotification = {
    ...notification,
    status: 'info',
    title: 'Model download queued',
  } satisfies Notification;

  expect(isNotificationOngoing(queuedNotification, [])).toBe(true);
  expect(
    visibleNotificationActions(queuedNotification, []).map(
      (action) => action.id,
    ),
  ).toEqual(['cancel']);
  expect(dismissableNotifications([queuedNotification], [])).toEqual([]);
});

test('failed model download notifications stay retryable before models load', () => {
  const failedNotification = {
    ...notification,
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
    status: 'failed',
    title: 'Model download failed',
  } satisfies Notification;

  expect(isNotificationOngoing(failedNotification, [])).toBe(false);
  expect(
    visibleNotificationActions(failedNotification, []).map(
      (action) => action.id,
    ),
  ).toEqual(['retry']);
  expect(dismissableNotifications([failedNotification], [])).toEqual([
    failedNotification,
  ]);
});
