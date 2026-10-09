import { useCallback, useEffect } from 'react';
import { Trash2 } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { NotificationHistory } from '@/features/notifications/components/notification-history';
import {
  dismissableNotifications,
  isNotificationOngoing,
} from '@/features/notifications/notification-lifecycle';
import { useNotificationsStore } from '@/features/notifications/notifications-store';

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
  const deletableCount = dismissableNotifications(
    notifications,
    aiModels,
  ).length;
  const ongoingCount = notifications.filter((notification) =>
    isNotificationOngoing(notification, aiModels),
  ).length;

  useEffect(() => {
    if (status === 'idle') {
      void load().catch(() => undefined);
    }
  }, [load, status]);

  const handleClearCompleted = useCallback(() => {
    void deleteAll().catch(() => undefined);
  }, [deleteAll]);

  const handleRetry = useCallback(() => {
    void load().catch(() => undefined);
  }, [load]);

  return (
    <PageLayout>
      <PageHeader
        title="Notifications"
        description="Local status history for capture, recorder, model, and processing work."
        actions={
          <div className="grid justify-items-start gap-1">
            <Button
              className="gap-2"
              disabled={deletableCount === 0 || status === 'loading'}
              onClick={handleClearCompleted}
              variant="outline"
            >
              <Trash2 aria-hidden="true" className="size-3.5" />
              {deletableCount > 0
                ? `Clear ${deletableCount} ${deletableCount === 1 ? 'update' : 'updates'}`
                : 'No updates to clear'}
            </Button>
            <span className="text-muted-foreground text-xs">
              {ongoingCount > 0
                ? `${ongoingCount} active ${ongoingCount === 1 ? 'task remains' : 'tasks remain'} until completion.`
                : notifications.length > 0
                  ? 'Status history is stored locally.'
                  : 'New status updates will appear here.'}
            </span>
          </div>
        }
      />

      <NotificationHistory
        actions={{ deleteNotifications, executeAction }}
        error={error}
        models={aiModels}
        notifications={notifications}
        onRetry={handleRetry}
        status={status}
      />
    </PageLayout>
  );
}
