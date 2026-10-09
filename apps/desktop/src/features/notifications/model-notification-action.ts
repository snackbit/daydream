import type { NotificationAction } from '@daydream/shared';
import {
  cancelAiModelDownload,
  downloadAiModel,
} from '@/features/ai/ai-client';
import {
  pauseAiModelDownload,
  resumeAiModelDownload,
} from '@/features/ai/models/model-transfer-client';
import { modelTransferError } from '@/features/ai/models/model-transfer-format';

export async function executeModelNotificationAction(
  action: NotificationAction | undefined,
  notificationId: string,
): Promise<boolean> {
  const modelId = action?.payload.modelId;
  if (!modelId || !action) return false;
  const input = { modelId, notificationId };
  try {
    switch (action.handler) {
      case 'model-download-retry':
        await downloadAiModel(input);
        return true;
      case 'model-download-cancel':
        await cancelAiModelDownload(input);
        return true;
      case 'model-download-pause':
        await pauseAiModelDownload(input);
        return true;
      case 'model-download-resume':
        await resumeAiModelDownload(input);
        return true;
      default:
        return false;
    }
  } catch (error) {
    throw new Error(modelTransferError(error), { cause: error });
  }
}
