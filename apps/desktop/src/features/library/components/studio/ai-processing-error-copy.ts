export function aiProcessingErrorSummary(errorCode: string): string {
  if (errorCode === 'ai_transcription_provider_failed') {
    return 'Local Whisper transcription failed. If you are using Vulkan or CUDA, switch local transcription backend to CPU in Settings if this keeps happening.';
  }
  if (errorCode === 'ai_transcription_local_worker_crashed') {
    return 'Local Whisper worker crashed. Retry the job; if it keeps crashing, switch local transcription backend to CPU in Settings.';
  }

  return errorCode;
}

export function aiProcessingErrorDetails(errorCode: string): string {
  const summary = aiProcessingErrorSummary(errorCode);

  return summary === errorCode ? errorCode : `${summary} (${errorCode})`;
}
