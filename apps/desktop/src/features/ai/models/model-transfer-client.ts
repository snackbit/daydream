import { invoke } from '@tauri-apps/api/core';
import type {
  AiModelAssetList,
  CancelAiModelDownloadInput,
  DeleteAiModelInput,
  DownloadAiModelInput,
  Notification,
} from '@daydream/shared';

export type ModelTransferAction =
  'download' | 'pause' | 'resume' | 'cancel' | 'delete' | 'remove';

export function pauseAiModelDownload(
  input: CancelAiModelDownloadInput,
): Promise<AiModelAssetList> {
  return invoke('pauseAiModelDownload', { input });
}
export function resumeAiModelDownload(
  input: DownloadAiModelInput,
): Promise<Notification> {
  return invoke('resumeAiModelDownload', { input });
}
export function removeAiModelPartial(
  input: DeleteAiModelInput,
): Promise<AiModelAssetList> {
  return invoke('removeAiModelPartial', { input });
}
