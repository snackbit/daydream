import { useEffect } from 'react';
import type { AiModelAsset, Notification } from '@daydream/shared';
import { Progress } from '@/components/ui/progress';
import { listenToAiModelsChanged } from '@/features/ai/ai-client';
import { useNotificationsStore } from '../notifications-store';

interface ModelDownloadNotificationProgressProps {
  models: AiModelAsset[];
  notification: Notification;
}

export function useAiModelProgressSubscription() {
  const refreshAiModels = useNotificationsStore(
    (state) => state.refreshAiModels,
  );
  const updateAiModels = useNotificationsStore((state) => state.updateAiModels);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;

    void refreshAiModels();
    void listenToAiModelsChanged((models) => updateAiModels(models)).then(
      (cleanup) => {
        if (active) {
          unlisten = cleanup;
        } else {
          cleanup();
        }
      },
    );

    return () => {
      active = false;
      unlisten?.();
    };
  }, [refreshAiModels, updateAiModels]);
}

export function ModelDownloadNotificationProgress({
  models,
  notification,
}: ModelDownloadNotificationProgressProps) {
  const modelId = notification.actions.find((action) => action.payload.modelId)
    ?.payload.modelId;
  const model = models.find((candidate) => candidate.id === modelId);

  if (!model || !isActiveDownload(model)) {
    return null;
  }

  const progress = downloadProgressPercent(model);

  return (
    <div className="mt-3 grid gap-1.5">
      <div className="text-muted-foreground flex items-center justify-between gap-2 text-xs">
        <span>{modelStatusText(model)}</span>
        <span>{downloadProgressLabel(model)}</span>
      </div>
      <Progress
        aria-label={`${model.label} download progress`}
        className="h-1.5"
        value={progress}
      />
    </div>
  );
}

function isActiveDownload(model: AiModelAsset): boolean {
  return (
    model.downloadStatus === 'downloading' || model.downloadStatus === 'queued'
  );
}

function modelStatusText(model: AiModelAsset): string {
  return model.downloadStatus === 'queued' ? 'Queued' : 'Downloading';
}

function downloadProgressLabel(model: AiModelAsset): string {
  if (model.downloadStatus === 'queued') {
    return 'Waiting';
  }

  if (model.totalBytes && model.totalBytes > 0) {
    const percent = downloadProgressPercent(model);

    return `${formatBytes(model.downloadedBytes)} of ${formatBytes(
      model.totalBytes,
    )} (${percent}%)`;
  }

  return `${formatBytes(model.downloadedBytes)} downloaded`;
}

function downloadProgressPercent(model: AiModelAsset): number {
  if (!model.totalBytes || model.totalBytes <= 0) {
    return 0;
  }

  return Math.min(
    100,
    Math.max(0, Math.round((model.downloadedBytes / model.totalBytes) * 100)),
  );
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${Math.round(bytes / (1024 * 1024))} MiB`;
  }

  return `${bytes} B`;
}
