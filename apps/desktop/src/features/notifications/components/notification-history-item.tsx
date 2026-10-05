import { useCallback } from 'react';
import type { AiModelAsset, Notification } from '@daydream/shared';
import { Trash2 } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  isNotificationOngoing,
  visibleNotificationActions,
} from '../notification-lifecycle';
import { ModelDownloadNotificationProgress } from './model-download-notification-progress';

interface NotificationHistoryItemProps {
  actions: {
    deleteNotifications: (notificationIds: string[]) => Promise<void>;
    executeAction: (
      notificationId: string,
      actionId: string,
      options: { acknowledgeResult: boolean },
    ) => Promise<void>;
  };
  models: AiModelAsset[];
  notification: Notification;
}

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: '2-digit',
  minute: '2-digit',
});

export function NotificationHistoryItem({
  actions,
  models,
  notification,
}: NotificationHistoryItemProps) {
  const { deleteNotifications, executeAction } = actions;
  const handleDismiss = useCallback(() => {
    void deleteNotifications([notification.id]);
  }, [deleteNotifications, notification.id]);
  const handleExecuteAction = useCallback(
    (actionId: string) => {
      void executeAction(notification.id, actionId, {
        acknowledgeResult: true,
      });
    },
    [executeAction, notification.id],
  );
  const visibleActions = visibleNotificationActions(notification, models);
  const ongoing = isNotificationOngoing(notification, models);

  return (
    <article className="grid min-w-0 gap-2 border-b px-2 py-3 last:border-b-0">
      <div className="flex min-w-0 items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <h3 className="min-w-0 text-xs font-medium">
              {notification.title}
            </h3>
            {notification.dismissedAtMs === null ? (
              <span className="text-primary text-[10px] font-medium">New</span>
            ) : null}
            {notification.status !== 'info' ? (
              <span
                className={`text-[10px] font-medium ${statusTextClass(notification.status)}`}
              >
                {statusLabel(notification.status)}
              </span>
            ) : null}
          </div>
          <p className="text-muted-foreground mt-1 text-xs leading-5">
            {notification.message}
          </p>
        </div>

        <div className="flex shrink-0 items-center gap-2">
          <time
            className="text-muted-foreground text-[10px] tabular-nums"
            dateTime={new Date(notification.createdAtMs).toISOString()}
          >
            {timeFormatter.format(notification.createdAtMs)}
          </time>
          {!ongoing ? (
            <Button
              aria-label={`Permanently dismiss ${notification.title}`}
              onClick={handleDismiss}
              size="icon-sm"
              variant="ghost"
            >
              <Trash2 aria-hidden="true" />
            </Button>
          ) : null}
        </div>
      </div>

      <div className="text-muted-foreground flex flex-wrap items-center gap-1.5 text-[10px]">
        <span>{formatFacet(notification.source)}</span>
        <span aria-hidden="true">·</span>
        <span>{formatFacet(notification.topic)}</span>
      </div>

      <ModelDownloadNotificationProgress
        models={models}
        notification={notification}
      />

      {visibleActions.length > 0 ? (
        <div className="flex flex-wrap gap-1.5 pt-1">
          {visibleActions.map((action) => (
            <NotificationActionButton
              action={action}
              key={action.id}
              onExecute={handleExecuteAction}
            />
          ))}
        </div>
      ) : null}
    </article>
  );
}

function NotificationActionButton({
  action,
  onExecute,
}: {
  action: Notification['actions'][number];
  onExecute: (actionId: string) => void;
}) {
  const handleClick = useCallback(() => {
    onExecute(action.id);
  }, [action.id, onExecute]);

  return (
    <Button
      onClick={handleClick}
      size="sm"
      variant={
        action.style === 'destructive'
          ? 'destructive'
          : action.style === 'secondary'
            ? 'secondary'
            : 'outline'
      }
    >
      {action.label}
    </Button>
  );
}

function statusLabel(status: Notification['status']): string {
  switch (status) {
    case 'cancelled':
      return 'Cancelled';
    case 'failed':
      return 'Failed';
    case 'info':
      return 'Update';
    case 'running':
      return 'In progress';
    case 'succeeded':
      return 'Complete';
    case 'warning':
      return 'Needs attention';
  }
}

function statusTextClass(status: Notification['status']): string {
  if (status === 'failed') {
    return 'text-destructive';
  }
  if (status === 'running') {
    return 'text-primary';
  }
  return 'text-muted-foreground';
}

function formatFacet(value: string): string {
  return value
    .replace(/[-_]/g, ' ')
    .replace(/\b\w/g, (letter) => letter.toLocaleUpperCase());
}
