import { useEffect, useMemo, useState } from 'react';
import type { Notification } from '@daydream/shared';
import { Trash2 } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  ModelDownloadNotificationProgress,
  useAiModelProgressSubscription,
} from '@/features/notifications/components/model-download-notification-progress';
import {
  dismissableNotifications,
  isNotificationOngoing,
  visibleNotificationActions,
} from '@/features/notifications/notification-lifecycle';
import { useNotificationsStore } from '@/features/notifications/notifications-store';

interface NotificationGroup {
  id: string;
  label: string;
  notifications: Notification[];
}

const dateFormatter = new Intl.DateTimeFormat(undefined, {
  day: 'numeric',
  month: 'short',
  year: 'numeric',
});

const timeFormatter = new Intl.DateTimeFormat(undefined, {
  hour: '2-digit',
  minute: '2-digit',
});

export function NotificationsScreen() {
  const aiModels = useNotificationsStore((state) => state.aiModels);
  const deleteAll = useNotificationsStore((state) => state.deleteAll);
  const deleteNotifications = useNotificationsStore(
    (state) => state.deleteNotifications,
  );
  const error = useNotificationsStore((state) => state.error);
  const executeAction = useNotificationsStore((state) => state.executeAction);
  const load = useNotificationsStore((state) => state.load);
  const notifications = useNotificationsStore((state) => state.notifications);
  const status = useNotificationsStore((state) => state.status);
  const [nowMs] = useState(() => Date.now());

  useAiModelProgressSubscription();

  useEffect(() => {
    if (status === 'idle') {
      void load();
    }
  }, [load, status]);

  const groups = useMemo(
    () => groupNotificationsByDate(notifications, nowMs),
    [notifications, nowMs],
  );
  const deletableCount = dismissableNotifications(
    notifications,
    aiModels,
  ).length;

  return (
    <section className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-normal">
            Notifications
          </h1>
          <p className="text-muted-foreground mt-2 max-w-3xl text-sm leading-6">
            Local status history for model, capture, recorder, and processing
            work.
          </p>
        </div>

        <Button
          className="gap-2"
          disabled={deletableCount === 0 || status === 'loading'}
          onClick={() => void deleteAll()}
          variant="destructive"
        >
          <Trash2 className="size-3.5" />
          Dismiss all notifications
        </Button>
      </div>

      {status === 'loading' ? (
        <div className="text-muted-foreground border p-4 text-sm">
          Loading notifications.
        </div>
      ) : null}

      {error ? (
        <div className="border-destructive/40 bg-destructive/10 text-destructive border p-4 text-sm">
          {error}
        </div>
      ) : null}

      {groups.length === 0 && status !== 'loading' ? (
        <div className="text-muted-foreground border p-4 text-sm">
          No notifications yet.
        </div>
      ) : null}

      <div className="flex flex-col gap-6">
        {groups.map((group) => (
          <section className="flex flex-col gap-3" key={group.id}>
            <div className="flex items-center gap-3">
              <h2 className="text-muted-foreground text-xs font-medium tracking-normal uppercase">
                {group.label}
              </h2>
              <div className="border-border h-px flex-1 border-t" />
            </div>

            <div className="divide-y border">
              {group.notifications.map((notification) => (
                <article className="bg-card p-4" key={notification.id}>
                  <div className="flex items-start justify-between gap-4">
                    <div className="min-w-0 flex-1">
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="text-sm font-medium">
                          {notification.title}
                        </h3>
                        <Badge variant={statusVariant(notification.status)}>
                          {notification.status}
                        </Badge>
                        {notification.dismissedAtMs === null ? (
                          <Badge variant="outline">New</Badge>
                        ) : null}
                      </div>

                      <p className="text-muted-foreground mt-2 text-sm leading-6">
                        {notification.message}
                      </p>
                      <ModelDownloadNotificationProgress
                        models={aiModels}
                        notification={notification}
                      />
                    </div>

                    <div className="flex shrink-0 items-center gap-2">
                      <time
                        className="text-muted-foreground text-xs"
                        dateTime={new Date(
                          notification.createdAtMs,
                        ).toISOString()}
                      >
                        {timeFormatter.format(notification.createdAtMs)}
                      </time>
                      {isNotificationOngoing(notification, aiModels) ? null : (
                        <Button
                          aria-label={`Permanently dismiss ${notification.title}`}
                          onClick={() =>
                            void deleteNotifications([notification.id])
                          }
                          size="icon-sm"
                          variant="ghost"
                        >
                          <Trash2 className="size-3" />
                        </Button>
                      )}
                    </div>
                  </div>

                  <div className="text-muted-foreground mt-3 flex flex-wrap items-center gap-2 text-xs">
                    <span>{notification.source}</span>
                    <span aria-hidden="true">/</span>
                    <span>{notification.topic}</span>
                  </div>

                  {visibleNotificationActions(notification, aiModels).length >
                  0 ? (
                    <div className="mt-3 flex flex-wrap gap-2">
                      {visibleNotificationActions(notification, aiModels).map(
                        (action) => (
                          <Button
                            key={action.id}
                            onClick={() =>
                              void executeAction(notification.id, action.id, {
                                acknowledgeResult: true,
                              })
                            }
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
                        ),
                      )}
                    </div>
                  ) : null}
                </article>
              ))}
            </div>
          </section>
        ))}
      </div>
    </section>
  );
}

export function groupNotificationsByDate(
  notifications: Notification[],
  nowMs: number,
): NotificationGroup[] {
  const sortedNotifications = [...notifications].sort(
    (first, second) => second.createdAtMs - first.createdAtMs,
  );
  const groups = new Map<string, NotificationGroup>();

  for (const notification of sortedNotifications) {
    const dateKey = localDateKey(notification.createdAtMs);

    if (!groups.has(dateKey)) {
      groups.set(dateKey, {
        id: dateKey,
        label: dateLabel(notification.createdAtMs, nowMs),
        notifications: [],
      });
    }

    groups.get(dateKey)!.notifications.push(notification);
  }

  return [...groups.values()];
}

function dateLabel(timestampMs: number, nowMs: number): string {
  const timestampDay = startOfLocalDay(timestampMs);
  const today = startOfLocalDay(nowMs);
  const yesterday = today - 86_400_000;

  if (timestampDay === today) {
    return 'Today';
  }

  if (timestampDay === yesterday) {
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

function statusVariant(
  status: string,
): 'default' | 'destructive' | 'outline' | 'secondary' {
  if (status === 'failed') {
    return 'destructive';
  }
  if (status === 'succeeded') {
    return 'secondary';
  }

  return 'outline';
}
