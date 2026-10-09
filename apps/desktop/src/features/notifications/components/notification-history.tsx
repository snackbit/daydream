import { useMemo, useState } from 'react';
import type { AiModelAsset, Notification } from '@daydream/shared';
import { AlertCircle, Clock3 } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { NotificationHistoryItem } from './notification-history-item';

interface NotificationHistoryProps {
  actions: {
    deleteNotifications: (notificationIds: string[]) => Promise<void>;
    executeAction: (
      notificationId: string,
      actionId: string,
      options: { acknowledgeResult: boolean },
    ) => Promise<void>;
  };
  error: string | null;
  models: AiModelAsset[];
  notifications: Notification[];
  onRetry: () => void;
  status: 'error' | 'idle' | 'loading' | 'ready';
}

interface NotificationGroup {
  id: string;
  label: string;
  notifications: Notification[];
}

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  day: 'numeric',
  month: 'long',
  year: 'numeric',
});

export function NotificationHistory({
  actions,
  error,
  models,
  notifications,
  onRetry,
  status,
}: NotificationHistoryProps) {
  const [nowMs] = useState(() => Date.now());
  const groups = useMemo(
    () => groupNotificationsByDate(notifications, nowMs),
    [notifications, nowMs],
  );
  const loading = status === 'idle' || status === 'loading';

  return (
    <div className="grid gap-4">
      {error ? (
        <Alert className="p-3" variant="destructive">
          <AlertCircle aria-hidden="true" className="size-3.5" />
          <AlertTitle>Could not update notifications</AlertTitle>
          <div className="flex flex-wrap items-start justify-between gap-3">
            <AlertDescription>{error}</AlertDescription>
            <Button onClick={onRetry} size="sm" variant="outline">
              Try again
            </Button>
          </div>
        </Alert>
      ) : null}

      {loading && notifications.length === 0 ? (
        <div
          aria-busy="true"
          className="bg-muted rounded-panel flex min-h-24 items-center gap-3 p-6"
          role="status"
        >
          <Clock3
            aria-hidden="true"
            className="text-muted-foreground size-3.5"
          />
          <span className="text-muted-foreground text-xs">
            Loading local status updates…
          </span>
        </div>
      ) : null}

      {loading && notifications.length > 0 ? (
        <p className="text-muted-foreground text-xs" role="status">
          Refreshing status updates…
        </p>
      ) : null}

      {groups.length === 0 && !loading && status === 'ready' ? (
        <div className="bg-muted rounded-panel p-6" role="status">
          <h2 className="text-sm font-medium">All caught up</h2>
          <p className="text-muted-foreground mt-2 text-sm leading-6">
            Capture, local processing, and model download updates will appear
            here when there is something to review.
          </p>
        </div>
      ) : null}

      {groups.map((group) => (
        <section
          aria-labelledby={`notification-group-${group.id}`}
          className="grid gap-2"
          key={group.id}
        >
          <div className="flex items-center justify-between gap-3">
            <h2
              className="text-sm font-medium"
              id={`notification-group-${group.id}`}
            >
              {group.label}
            </h2>
            <span className="text-muted-foreground text-xs tabular-nums">
              {group.notifications.length}{' '}
              {group.notifications.length === 1 ? 'update' : 'updates'}
            </span>
          </div>

          <div className="bg-muted rounded-panel divide-y overflow-hidden">
            {group.notifications.map((notification) => (
              <NotificationHistoryItem
                actions={actions}
                key={notification.id}
                models={models}
                notification={notification}
              />
            ))}
          </div>
        </section>
      ))}
    </div>
  );
}

export function groupNotificationsByDate(
  notifications: Notification[],
  nowMs: number,
): NotificationGroup[] {
  const sortedNotifications = [...notifications].sort(
    (first, second) => second.createdAtMs - first.createdAtMs,
  );
  const today = startOfLocalDay(nowMs);
  const yesterdayDate = new Date(today);
  yesterdayDate.setDate(yesterdayDate.getDate() - 1);
  const yesterday = yesterdayDate.getTime();
  const groups = new Map<string, NotificationGroup>();

  for (const notification of sortedNotifications) {
    const dateKey = localDateKey(notification.createdAtMs);
    let group = groups.get(dateKey);
    if (!group) {
      group = {
        id: dateKey,
        label: dateLabel(notification.createdAtMs, today, yesterday),
        notifications: [],
      };
      groups.set(dateKey, group);
    }
    group.notifications.push(notification);
  }

  return [...groups.values()];
}

function dateLabel(
  timestampMs: number,
  todayMs: number,
  yesterdayMs: number,
): string {
  const timestampDay = startOfLocalDay(timestampMs);

  if (timestampDay === todayMs) {
    return 'Today';
  }
  if (timestampDay === yesterdayMs) {
    return 'Yesterday';
  }

  return dateFormatter.format(timestampMs);
}

function localDateKey(timestampMs: number): string {
  const date = new Date(timestampMs);
  return [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, '0'),
    String(date.getDate()).padStart(2, '0'),
  ].join('-');
}

function startOfLocalDay(timestampMs: number): number {
  const date = new Date(timestampMs);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}
