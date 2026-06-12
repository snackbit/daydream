export type NotificationSource =
  | 'ai'
  | 'capture'
  | 'library'
  | 'recorder'
  | 'settings'
  | 'system';

export type NotificationStatus =
  | 'cancelled'
  | 'failed'
  | 'info'
  | 'running'
  | 'succeeded'
  | 'warning';

export type NotificationActionStyle = 'default' | 'destructive' | 'secondary';

export type NotificationActionHandler =
  | 'model-download-cancel'
  | 'model-download-retry'
  | 'processing-job-cancel'
  | 'processing-job-pause'
  | 'processing-job-resume'
  | 'processing-job-retry';

export interface NotificationAction {
  handler: NotificationActionHandler;
  id: string;
  label: string;
  payload: Record<string, string>;
  style: NotificationActionStyle;
}

export interface Notification {
  actions: NotificationAction[];
  createdAtMs: number;
  dismissedAtMs: number | null;
  id: string;
  message: string;
  source: NotificationSource;
  status: NotificationStatus;
  title: string;
  topic: string;
  updatedAtMs: number;
}

export interface NotificationList {
  notifications: Notification[];
}

export interface AcknowledgeNotificationsInput {
  notificationIds: string[];
}

export interface DismissNotificationInput {
  notificationId: string;
}

export interface DeleteNotificationsInput {
  notificationIds: string[];
}

export interface ExecuteNotificationActionInput {
  actionId: string;
  notificationId: string;
}
