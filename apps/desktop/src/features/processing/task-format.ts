import type { BackgroundTask, ProcessingJobStatus } from '@daydream/shared';

export function taskStatusLabel(status: ProcessingJobStatus): string {
  switch (status) {
    case 'completed-with-errors':
      return 'Completed with errors';
    case 'succeeded':
      return 'Complete';
    case 'running':
      return 'Processing';
    case 'queued':
      return 'Queued';
    case 'paused':
      return 'Paused';
    case 'failed':
      return 'Needs attention';
    case 'cancelled':
      return 'Cancelled';
  }
}

export function taskNeedsAttention(task: BackgroundTask): boolean {
  return (
    task.status === 'failed' ||
    task.status === 'completed-with-errors' ||
    task.stages.some((stage) => stage.failed > 0)
  );
}
export function stageLabel(kind: string): string {
  switch (kind) {
    case 'recall-index':
    case 'recall-indexing':
      return 'Recall indexing';
    case 'recall-generation':
      return 'Day generation';
    case 'audio':
    case 'transcription':
      return 'Transcription';
    case 'ocr':
      return 'OCR';
    case 'vlm':
      return 'Screen observations';
    case 'activity':
      return 'Activity analysis';
    case 'model-download':
      return 'Model download';
    case 'waveform':
    case 'waveforms':
      return 'Waveforms';
    case 'thumbnail':
    case 'thumbnails':
      return 'Thumbnails';
    case 'frame-index':
      return 'Frame index';
    default:
      return 'AI processing';
  }
}
export function taskPhaseLabel(task: BackgroundTask): string {
  switch (task.phase) {
    case 'detecting-speech':
      return 'Checking for speech';
    case 'transcribing':
      return 'Transcribing speech';
    case 'recognizing-text':
      return 'Recognizing screen text';
    case 'waiting-for-recording':
      return 'Caught up · waiting for the next segment';
    case 'waiting-for-audio-context':
      return 'Waiting for the next finalized audio section';
    case 'media-preparation':
      return 'Preparing playback';
    case 'waveforms':
      return 'Generating waveforms';
    case 'thumbnails':
      return 'Generating thumbnails';
    case 'frame-index':
      return 'Reading video timestamps';
    case 'retrying':
      return 'Waiting to retry';
    case 'verifying':
      return 'Verifying the model';
    case 'starting':
      return 'Preparing source';
    default:
      return taskStatusLabel(task.status);
  }
}
export function formatElapsed(milliseconds: number): string {
  const seconds = Math.floor(Math.max(0, milliseconds) / 1_000);
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.floor(seconds / 60);
  return minutes < 60
    ? `${minutes}m ${seconds % 60}s`
    : `${Math.floor(minutes / 60)}h ${minutes % 60}m`;
}
export function formatEta(task: BackgroundTask): string {
  if (task.inputOpen) return 'Live input';
  if (task.etaMs === null && task.kind.startsWith('recall-'))
    return 'Time estimate unavailable';
  if (task.etaMs === null)
    return task.status === 'running' ? 'Estimating remaining time' : '—';
  return `About ${formatElapsed(task.etaMs)} remaining`;
}
