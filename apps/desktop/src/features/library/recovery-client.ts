import type {
  RecoverLibraryRecordingInput,
  RecordingRecoveryResult,
} from '@daydream/shared';
import { invoke } from '@/lib/diagnostics/invoke';

export function recoverLibraryRecording(
  input: RecoverLibraryRecordingInput,
): Promise<RecordingRecoveryResult> {
  return invoke('recoverLibraryRecording', { input });
}
