import { invoke } from '@tauri-apps/api/core';
import type {
  RecoverLibraryRecordingInput,
  RecordingRecoveryResult,
} from '@daydream/shared';

export function recoverLibraryRecording(
  input: RecoverLibraryRecordingInput,
): Promise<RecordingRecoveryResult> {
  return invoke('recoverLibraryRecording', { input });
}
