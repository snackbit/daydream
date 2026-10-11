import type {
  AcknowledgeNotificationsInput,
  DeleteNotificationsInput,
  DismissNotificationInput,
  ExecuteNotificationActionInput,
  NotificationList,
} from '@daydream/shared';
import { invoke } from '@/lib/diagnostics/invoke';

export async function getNotifications(): Promise<NotificationList> {
  return invoke<NotificationList>('getNotifications');
}

export async function acknowledgeNotifications(
  input: AcknowledgeNotificationsInput,
): Promise<NotificationList> {
  return invoke<NotificationList>('acknowledgeNotifications', { input });
}

export async function dismissNotification(
  input: DismissNotificationInput,
): Promise<NotificationList> {
  return invoke<NotificationList>('dismissNotification', { input });
}

export async function deleteNotifications(
  input: DeleteNotificationsInput,
): Promise<NotificationList> {
  return invoke<NotificationList>('deleteNotifications', { input });
}

export async function deleteAllNotifications(): Promise<NotificationList> {
  return invoke<NotificationList>('deleteAllNotifications');
}

export async function executeNotificationAction(
  input: ExecuteNotificationActionInput,
): Promise<NotificationList> {
  return invoke<NotificationList>('executeNotificationAction', { input });
}
