import type {
  AiModelAsset,
  Notification,
  NotificationAction,
} from '@daydream/shared';

export function notificationModelId(notification: Notification): string | null {
  return (
    notification.actions.find((action) => action.payload.modelId)?.payload
      .modelId ?? null
  );
}

export function isNotificationOngoing(
  notification: Notification,
  models: AiModelAsset[],
): boolean {
  const modelId = notificationModelId(notification);
  const model = modelId
    ? models.find((candidate) => candidate.id === modelId)
    : null;

  if (model) {
    return (
      model.downloadStatus === 'downloading' ||
      model.downloadStatus === 'queued'
    );
  }

  return isFallbackActiveModelDownloadNotification(notification);
}

export function visibleNotificationActions(
  notification: Notification,
  models: AiModelAsset[],
): NotificationAction[] {
  return notification.actions.filter((action) => {
    if (
      action.handler !== 'model-download-cancel' &&
      action.handler !== 'model-download-retry'
    ) {
      return true;
    }

    const modelId = action.payload.modelId;
    const model = modelId
      ? models.find((candidate) => candidate.id === modelId)
      : null;
    const isActive = model
      ? model.downloadStatus === 'downloading' ||
        model.downloadStatus === 'queued'
      : isFallbackActiveModelDownloadNotification(notification);

    return action.handler === 'model-download-cancel' ? isActive : !isActive;
  });
}

export function dismissableNotifications(
  notifications: Notification[],
  models: AiModelAsset[],
): Notification[] {
  return notifications.filter(
    (notification) => !isNotificationOngoing(notification, models),
  );
}

function isFallbackActiveModelDownloadNotification(
  notification: Notification,
): boolean {
  return (
    notification.source === 'ai' &&
    notification.topic === 'model-download' &&
    (notification.status === 'info' || notification.status === 'running') &&
    notification.actions.some(
      (action) =>
        action.handler === 'model-download-cancel' &&
        action.payload.modelId !== undefined,
    )
  );
}
