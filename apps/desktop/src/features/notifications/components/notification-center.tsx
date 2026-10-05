import { useCallback, useEffect, useState } from 'react';
import { Link } from '@tanstack/react-router';
import type { Notification } from '@daydream/shared';
import { Bell, X } from 'lucide-react';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import {
  Popover,
  PopoverContent,
  PopoverTrigger,
} from '@/components/ui/popover';
import { ScrollArea } from '@/components/ui/scroll-area';
import {
  isNotificationOngoing,
  visibleNotificationActions,
} from '../notification-lifecycle';
import { useNotificationsStore } from '../notifications-store';
import {
  ModelDownloadNotificationProgress,
  useAiModelProgressSubscription,
} from './model-download-notification-progress';

export function NotificationCenter() {
  const acknowledge = useNotificationsStore((state) => state.acknowledge);
  const aiModels = useNotificationsStore((state) => state.aiModels);
  const error = useNotificationsStore((state) => state.error);
  const executeAction = useNotificationsStore((state) => state.executeAction);
  const load = useNotificationsStore((state) => state.load);
  const notifications = useNotificationsStore((state) => state.notifications);
  const status = useNotificationsStore((state) => state.status);
  const [open, setOpen] = useState(false);
  const [popoverNotifications, setPopoverNotifications] = useState<
    Notification[]
  >([]);

  useAiModelProgressSubscription();

  useEffect(() => {
    if (status === 'idle') {
      void load().catch(() => undefined);
    }
  }, [load, status]);

  const handleRetry = useCallback(async () => {
    await load().catch(() => undefined);
    setPopoverNotifications(
      useNotificationsStore
        .getState()
        .notifications.filter(
          (notification) => notification.dismissedAtMs === null,
        ),
    );
  }, [load]);

  const unseenNotifications = notifications.filter(
    (notification) => notification.dismissedAtMs === null,
  );
  const seenCount = notifications.length - unseenNotifications.length;
  const unreadCount = unseenNotifications.length;
  const visibleNotifications = open
    ? popoverNotifications
    : unseenNotifications;

  function updateOpen(nextOpen: boolean) {
    setOpen(nextOpen);

    if (!nextOpen) {
      setPopoverNotifications([]);
      return;
    }

    setPopoverNotifications(unseenNotifications);
    void acknowledge(
      unseenNotifications.map((notification) => notification.id),
    ).catch(() => undefined);
  }

  function acknowledgeAndHide(notificationId: string) {
    setPopoverNotifications((currentNotifications) =>
      currentNotifications.filter(
        (notification) => notification.id !== notificationId,
      ),
    );
    void acknowledge([notificationId]).catch(() => undefined);
  }

  return (
    <Popover onOpenChange={updateOpen} open={open}>
      <PopoverTrigger asChild>
        <Button
          aria-label={`Notifications${unreadCount > 0 ? `, ${unreadCount} unread` : ''}`}
          className="relative"
          size="icon"
          variant="outline"
        >
          <Bell className="size-4" />
          {unreadCount > 0 ? (
            <Badge
              aria-hidden="true"
              className="absolute -top-1 -right-1 h-4 min-w-4 px-1 text-[10px] leading-4"
            >
              {unreadCount}
            </Badge>
          ) : null}
        </Button>
      </PopoverTrigger>
      <PopoverContent
        align="end"
        className="w-[calc(100vw-1.5rem)] max-w-96 p-0"
      >
        <div className="flex items-center justify-between border-b px-3 py-2">
          <div>
            <h2 className="text-sm font-medium">Notifications</h2>
            <p className="text-muted-foreground text-xs">
              Persistent local status updates.
            </p>
          </div>
          <div className="flex items-center gap-2">
            {status === 'loading' ? (
              <Badge variant="outline">Loading</Badge>
            ) : null}
            <Button asChild size="sm" variant="outline">
              <Link onClick={() => setOpen(false)} to="/notifications">
                View all
              </Link>
            </Button>
          </div>
        </div>

        {error ? (
          <div className="flex items-start justify-between gap-2 border-b px-3 py-2">
            <p className="text-destructive min-w-0 text-xs" role="alert">
              {error}
            </p>
            <Button onClick={handleRetry} size="sm" variant="outline">
              Retry
            </Button>
          </div>
        ) : null}

        {visibleNotifications.length === 0 ? (
          <div className="text-muted-foreground px-3 py-6 text-sm">
            {error
              ? 'Status unavailable.'
              : seenCount > 0
                ? `No new notifications. ${seenCount} older seen ${
                    seenCount === 1 ? 'notification is' : 'notifications are'
                  } in history.`
                : 'No notifications.'}
          </div>
        ) : (
          <ScrollArea className="max-h-96 overflow-auto">
            <div className="divide-y">
              {visibleNotifications.map((notification) => (
                <article className="p-3" key={notification.id}>
                  <div className="flex items-start gap-2">
                    <div className="min-w-0 flex-1">
                      <div className="flex items-center gap-2">
                        <h3 className="truncate text-sm font-medium">
                          {notification.title}
                        </h3>
                        <Badge variant={statusVariant(notification.status)}>
                          {notification.status}
                        </Badge>
                      </div>
                      <p className="text-muted-foreground mt-1 text-xs leading-5">
                        {notification.message}
                      </p>
                      <ModelDownloadNotificationProgress
                        models={aiModels}
                        notification={notification}
                      />
                    </div>
                    {isNotificationOngoing(notification, aiModels) ? null : (
                      <Button
                        aria-label={`Dismiss ${notification.title}`}
                        onClick={() => acknowledgeAndHide(notification.id)}
                        size="icon"
                        variant="ghost"
                      >
                        <X className="size-3.5" />
                      </Button>
                    )}
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
                              }).catch(() => undefined)
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
          </ScrollArea>
        )}
      </PopoverContent>
    </Popover>
  );
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
