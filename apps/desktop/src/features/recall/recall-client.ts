import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  AcceptRecallSuggestionInput,
  ControlRecallJobInput,
  DismissRecallSuggestionInput,
  ListRecallDaysInput,
  RecallDayInput,
  RecallDay,
  RecallDayCard,
  RecallEvidence,
  RecallJob,
  RecallProviderTestResult,
  RecallSearchResult,
  RecallStatus,
  RecallTask,
  RecallSearchInput,
  RecallStateChanged,
  ResolveRecallEvidenceInput,
  SetRecallDayReviewedInput,
  SetRecallFeedbackInput,
  SetRecallSessionCorrectionInput,
  TestRecallProviderInput,
  UpdateRecallTaskInput,
} from '@daydream/shared';
import {
  decodeRecallDay,
  decodeRecallDays,
  decodeRecallJob,
  decodeRecallProviderTestResult,
  decodeRecallSearchResult,
  decodeRecallStateChanged,
  decodeRecallStatus,
  decodeRecallTask,
  decodeResolvedRecallEvidence,
} from './recall-decoders';

export async function listRecallDays(
  input: ListRecallDaysInput,
): Promise<RecallDayCard[]> {
  return decodeRecallDays(await invoke<unknown>('listRecallDays', { input }));
}
export async function getRecallDay(input: RecallDayInput): Promise<RecallDay> {
  return decodeRecallDay(await invoke<unknown>('getRecallDay', { input }));
}
export async function searchRecallEvidence(
  input: RecallSearchInput,
): Promise<RecallSearchResult> {
  return decodeRecallSearchResult(
    await invoke<unknown>('searchRecallEvidence', { input }),
  );
}
export async function resolveRecallEvidence(
  input: ResolveRecallEvidenceInput,
): Promise<RecallEvidence | null> {
  return decodeResolvedRecallEvidence(
    await invoke<unknown>('resolveRecallEvidence', { input }),
  );
}
export async function regenerateRecallDay(
  input: RecallDayInput,
): Promise<RecallJob> {
  return decodeRecallJob(
    await invoke<unknown>('regenerateRecallDay', { input }),
  );
}
export async function setRecallFeedback(
  input: SetRecallFeedbackInput,
): Promise<RecallDay> {
  return decodeRecallDay(await invoke<unknown>('setRecallFeedback', { input }));
}
export async function setRecallSessionCorrection(
  input: SetRecallSessionCorrectionInput,
): Promise<RecallDay> {
  return decodeRecallDay(
    await invoke<unknown>('setRecallSessionCorrection', { input }),
  );
}
export async function acceptRecallSuggestion(
  input: AcceptRecallSuggestionInput,
): Promise<RecallTask> {
  return decodeRecallTask(
    await invoke<unknown>('acceptRecallSuggestion', { input }),
  );
}
export async function updateRecallTask(
  input: UpdateRecallTaskInput,
): Promise<RecallTask> {
  return decodeRecallTask(await invoke<unknown>('updateRecallTask', { input }));
}
export async function dismissRecallSuggestion(
  input: DismissRecallSuggestionInput,
): Promise<RecallDay> {
  return decodeRecallDay(
    await invoke<unknown>('dismissRecallSuggestion', { input }),
  );
}
export async function setRecallDayReviewed(
  input: SetRecallDayReviewedInput,
): Promise<RecallDay> {
  return decodeRecallDay(
    await invoke<unknown>('setRecallDayReviewed', { input }),
  );
}
export async function getRecallStatus(): Promise<RecallStatus> {
  return decodeRecallStatus(await invoke<unknown>('getRecallStatus'));
}
export async function cancelRecallSearch(): Promise<void> {
  const result = await invoke<unknown>('cancelRecallSearch');
  if (result !== null && result !== undefined) {
    throw new Error(
      'Recall returned an invalid response. Reload to reconnect.',
    );
  }
}
export async function testRecallProvider(
  input: TestRecallProviderInput,
): Promise<RecallProviderTestResult> {
  return decodeRecallProviderTestResult(
    await invoke<unknown>('testRecallProvider', { input }),
  );
}
export async function controlRecallJob(
  input: ControlRecallJobInput,
): Promise<RecallJob> {
  return decodeRecallJob(await invoke<unknown>('controlRecallJob', { input }));
}
export async function listenToRecallStateChanged(
  handler: (event: RecallStateChanged) => void,
  onInvalid?: (error: Error) => void,
): Promise<UnlistenFn> {
  return listen<unknown>('recall-state-changed', (event) => {
    let metadata: RecallStateChanged;
    try {
      metadata = decodeRecallStateChanged(event.payload);
    } catch {
      const error = new Error(
        'Recall returned an invalid response. Reload to reconnect.',
      );
      if (onInvalid) {
        onInvalid(error);
        return;
      }
      throw error;
    }
    handler(metadata);
  });
}
