import type { AiModelAsset } from '@daydream/shared';

export function formatModelBytes(bytes: number): string {
  if (bytes >= 1024 ** 3) return `${(bytes / 1024 ** 3).toFixed(1)} GiB`;
  if (bytes >= 1024 ** 2) return `${(bytes / 1024 ** 2).toFixed(1)} MiB`;
  if (bytes >= 1024) return `${Math.round(bytes / 1024)} KiB`;
  return `${bytes} B`;
}
export function modelTransferPercent(model: AiModelAsset): number | null {
  return model.totalBytes && model.totalBytes > 0
    ? Math.max(
        0,
        Math.min(100, (model.downloadedBytes / model.totalBytes) * 100),
      )
    : null;
}
export function modelTransferState(model: AiModelAsset): string {
  if (model.selfTestStatus === 'running') return 'Testing model';
  switch (model.transfer?.phase) {
    case 'connecting':
      return 'Connecting';
    case 'restarting':
      return 'Restarting download from the beginning';
    case 'stalled':
      return 'Waiting for the server';
    case 'pausing':
      return 'Saving partial data';
    case 'verifying':
      return 'Checking download';
    case 'testing':
      return 'Testing model';
  }
  switch (model.downloadStatus) {
    case 'available':
      return model.selfTestStatus === 'failed'
        ? 'Downloaded · model test failed'
        : 'Downloaded';
    case 'paused':
      return 'Paused · partial data saved';
    case 'queued':
      return 'Queued';
    case 'cancelled':
      return 'Cancelled · partial data saved';
    case 'failed':
      return 'Download needs attention';
    case 'downloading':
      return 'Downloading';
    case 'missing':
      return 'Not downloaded';
  }
}
export function modelTransferError(error: unknown): string {
  const code =
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string'
      ? error.code
      : null;
  if (code === 'ai_storage_failed')
    return 'The download could not be saved. Check available disk space, then retry.';
  if (code === 'ai_maintenance_active')
    return 'Maintenance is in progress. Wait for it to finish, then retry this model action.';
  if (code === 'ai_model_download_busy')
    return 'Pause the download and wait for its data to be saved, then retry.';
  if (
    code === 'ai_model_checksum_failed' ||
    code === 'ai_ocr_tesseract_checksum_failed' ||
    code === 'ai_model_download_range_invalid'
  )
    return 'The downloaded data could not be verified. Remove partial data, then retry.';
  if (code === 'ai_ocr_tesseract_runtime_unavailable')
    return 'A managed OCR runtime is not available for this platform.';
  return 'The model action could not be completed. Your saved partial data can be retried.';
}
