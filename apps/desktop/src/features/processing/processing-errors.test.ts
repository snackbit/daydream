import { expect, test } from 'vitest';
import {
  describeProcessingError,
  processingFailureSummary,
} from './processing-errors';

test.each([
  ['recall_embedding_vulkan_unavailable', 'ai', 'recall'],
  ['recall_embedding_model_missing', 'ai', 'recall'],
  ['recall_embedding_worker_unavailable', 'ai', 'recall'],
  ['ai_vision_subscription_usage_limit', 'ai', 'models'],
  ['ai_vision_subscription_usage_unavailable', 'ai', 'models'],
  ['ai_vision_subscription_reconnect', 'ai', 'models'],
  ['ai_maintenance_active', null, null],
  ['ai_processing_retry_unavailable', null, null],
  ['ai_processing_selection_invalid', null, null],
  ['recall_privacy_changed', 'ai', 'recall'],
  ['recall_stale_input', 'ai', 'recall'],
  ['recall_provider_auth_failed', 'ai', 'recall'],
  ['recall_credential_missing', 'ai', 'recall'],
  ['recall_model_unavailable', 'ai', 'recall'],
  ['recall_backend_unavailable', 'ai', 'recall'],
  ['recall_provider_request_invalid', 'ai', 'recall'],
  ['ai_transcription_vad_model_missing', null, null],
  ['ai_transcription_vad_failed', null, null],
  ['ai_storage_failed', 'storage', null],
  ['ai_disk_full', 'storage', null],
  ['ai_processing_privacy_changed', 'record', null],
  ['ai_processing_protection_pending', 'record', null],
  ['ai_processing_model_unavailable', 'ai', 'models'],
  ['ai_processing_model_missing', 'ai', 'models'],
  ['ai_processing_local_model_unavailable', 'ai', 'models'],
  ['ai_ocr_tesseract_failed', 'ai', 'ocr'],
  ['ai_ocr_tessdata_missing', 'ai', 'ocr'],
  ['ai_ocr_engine_invalid', 'ai', 'ocr'],
  ['ai_ocr_disabled', 'ai', 'ocr'],
  ['ai_vision_credential_unavailable', 'ai', 'vision'],
  ['ai_vlm_auth_failed', 'ai', 'vision'],
  ['ai_vision_configuration_changed', 'ai', 'vision'],
  ['ai_vision_remote_unconfigured', 'ai', 'vision'],
  ['ai_vision_model_required', 'ai', 'vision'],
  ['ai_vision_config_invalid', 'ai', 'vision'],
  ['ai_vision_endpoint_invalid', 'ai', 'vision'],
  ['ai_vision_model_not_found', 'ai', 'vision'],
  ['ai_vision_rate_limited', 'ai', 'vision'],
  ['ai_vision_response_truncated', 'ai', 'vision'],
  ['ai_vision_response_invalid', 'ai', 'vision'],
  ['ai_transcription_schema_invalid', 'ai', 'transcription'],
  ['ai_ocr_document_json_invalid', 'ai', 'ocr'],
  ['ai_transcription_envelope_invalid', 'ai', 'transcription'],
  ['ai_processing_job_worker_failed', 'ai', 'transcription'],
  ['ai_processing_chunk_worker_failed', 'ai', 'transcription'],
  ['ai_transcription_local_worker_crashed', 'ai', 'transcription'],
  ['ai_ocr_frame_extract_timeout', 'ai', 'ocr'],
  ['ai_vision_request_failed', 'ai', 'vision'],
  ['ai_vision_server_failed', 'ai', 'vision'],
  ['ai_vision_provider_failed', 'ai', 'vision'],
  ['ai_processing_retry_pending', 'ai', 'transcription'],
  ['ai_activity_worker_failed', 'ai', 'processing'],
  ['synthetic_unknown_failure', 'ai', 'transcription'],
] as const)(
  'routes %s recovery to its owning feature',
  (code, tab, section) => {
    const result = describeProcessingError(code);
    expect(result.settingsTab).toBe(tab);
    expect(result.settingsSection).toBe(section);
    expect(result.settingsLabel === null).toBe(tab === null);
    expect(result.summary).not.toContain(code);
  },
);

test('embedding guidance distinguishes a missing model from an unavailable GPU worker', () => {
  expect(
    describeProcessingError('recall_embedding_vulkan_unavailable').summary,
  ).toContain('CPU');
  expect(
    describeProcessingError('recall_embedding_model_missing').summary,
  ).toContain('Download');
});

test('untrusted exceptions never become user-facing troubleshooting content', () => {
  for (const value of [
    null,
    undefined,
    42,
    {},
    { code: 42 },
    new Error('Synthetic private detail'),
  ]) {
    expect(processingFailureSummary(value)).toBe(
      'Processing could not complete this action. Retry to reconnect.',
    );
  }
  expect(
    processingFailureSummary({
      code: 'recall_embedding_vulkan_unavailable',
      message: 'Synthetic private detail',
    }),
  ).toBe(
    describeProcessingError('recall_embedding_vulkan_unavailable').summary,
  );
});
