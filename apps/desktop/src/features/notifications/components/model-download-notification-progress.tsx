import { useEffect } from 'react';
import type { AiModelAsset, Notification } from '@daydream/shared';
import { listenToAiModelsChanged } from '@/features/ai/ai-client';
import { ModelTransferProgress } from '@/features/ai/models/model-transfer-progress';
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

  return (
    <div className="mt-3 grid gap-1.5">
      <ModelTransferProgress model={model} />
    </div>
  );
}

function isActiveDownload(model: AiModelAsset): boolean {
  return (
    model.downloadStatus === 'downloading' ||
    model.downloadStatus === 'queued' ||
    model.downloadStatus === 'paused'
  );
}
