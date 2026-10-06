export interface ProcessingErrorDescription {
  summary: string;
  settingsTab: 'ai' | 'record' | 'storage' | null;
  settingsSection:
    'transcription' | 'vision' | 'ocr' | 'models' | 'processing' | null;
  settingsLabel: string | null;
}

export function describeProcessingError(
  code: string,
): ProcessingErrorDescription {
  if (code.includes('_vad_model_'))
    return {
      summary:
        'The bundled speech detector is missing or damaged. Repair the application installation, then retry this audio section.',
      settingsTab: null,
      settingsSection: null,
      settingsLabel: null,
    };
  if (code.includes('_vad_'))
    return {
      summary:
        'Speech detection could not finish this audio section. Retry it; other processing can continue.',
      settingsTab: null,
      settingsSection: null,
      settingsLabel: null,
    };
  if (code.includes('storage') || code.includes('disk'))
    return {
      summary:
        'Processing could not read or save local data. Check available storage and file access.',
      settingsTab: 'storage',
      settingsSection: null,
      settingsLabel: 'Open storage settings',
    };
  if (code.includes('privacy') || code.includes('protection'))
    return {
      summary:
        'Processing is waiting for recording privacy protection to be resolved. Hidden sources remain excluded.',
      settingsTab: 'record',
      settingsSection: null,
      settingsLabel: 'Review recording protection',
    };
  if (
    code.includes('model_unavailable') ||
    code.includes('model_missing') ||
    code.includes('local_model')
  )
    return {
      summary:
        'The selected transcription model is not ready. Download or verify it, then retry with current settings.',
      settingsTab: 'ai',
      settingsSection: 'models',
      settingsLabel: 'Manage local models',
    };
  if (
    code.includes('tesseract') ||
    code.includes('tessdata') ||
    code === 'ai_ocr_engine_invalid' ||
    code === 'ai_ocr_disabled'
  )
    return {
      summary:
        'Local OCR is unavailable or incorrectly configured. Check its engine and language resources.',
      settingsTab: 'ai',
      settingsSection: 'ocr',
      settingsLabel: 'Review OCR settings',
    };
  const vision = code.includes('vision') || code.includes('vlm');
  const target = vision
    ? 'vision'
    : code.includes('_ocr_')
      ? 'ocr'
      : 'transcription';
  let summary: string;
  if (code.includes('credential'))
    summary =
      'The saved credential could not be opened. Save it again in provider settings, then retry with current settings.';
  else if (code.includes('auth'))
    summary =
      'The provider rejected authentication. Update the saved credential, then retry with current settings.';
  else if (code.includes('configuration_changed'))
    summary =
      'Provider settings changed during processing. Retry the failed sections with current settings to use the updated configuration.';
  else if (
    code.includes('unconfigured') ||
    code.includes('required') ||
    code.includes('config_invalid') ||
    code.includes('endpoint') ||
    code.includes('model_not_found')
  )
    summary =
      'This processing stage needs a valid provider, endpoint, and model configuration.';
  else if (code.includes('rate_limited'))
    summary =
      'The provider is limiting requests. Processing will retry after the waiting period.';
  else if (code.includes('truncated'))
    summary =
      'The provider cut its response short. Choose a model that can return the complete result, then retry.';
  else if (
    code.includes('response_invalid') ||
    code.includes('schema_invalid') ||
    code.includes('document_json_invalid') ||
    code.includes('envelope_invalid')
  )
    summary = vision
      ? 'The vision provider returned an unreadable OCR document. Try a compatible vision model or switch OCR to the local engine.'
      : 'The transcription provider returned an invalid result. Review the selected model and retry this section.';
  else if (code.includes('local_worker_crashed'))
    summary =
      'The local Whisper worker stopped unexpectedly. Retry; if it keeps failing, review the local backend and consider CPU.';
  else if (code.includes('timeout'))
    summary =
      'This operation exceeded its time limit. Retry the affected section; other work can continue.';
  else if (
    code.includes('request_failed') ||
    code.includes('server_failed') ||
    code.includes('provider_failed')
  )
    summary =
      'The provider could not process this section. Retry it; completed results remain available.';
  else if (code.includes('pending'))
    summary =
      'An active operation is still stopping. Wait for it to finish, then retry the action.';
  else
    summary =
      'This section could not be processed. Retry it or review the technical details.';
  return {
    summary,
    settingsTab: 'ai',
    settingsSection: target,
    settingsLabel: vision
      ? 'Review vision settings'
      : 'Review transcription settings',
  };
}

export function processingFailureSummary(error: unknown): string {
  if (
    typeof error === 'object' &&
    error !== null &&
    'code' in error &&
    typeof error.code === 'string'
  )
    return describeProcessingError(error.code).summary;
  return error instanceof Error
    ? error.message
    : 'Processing could not complete this action. Retry to reconnect.';
}
