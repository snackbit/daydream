import type { UnlistenFn } from '@tauri-apps/api/event';
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
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';
import { listen } from '@/lib/diagnostics/listen';
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
  return decodeResponse(
    'listRecallDays',
    await invoke<unknown>('listRecallDays', { input }),
    decodeRecallDays,
  );
}
export async function getRecallDay(input: RecallDayInput): Promise<RecallDay> {
  return decodeResponse(
    'getRecallDay',
    await invoke<unknown>('getRecallDay', { input }),
    decodeRecallDay,
  );
}
export async function searchRecallEvidence(
  input: RecallSearchInput,
): Promise<RecallSearchResult> {
  return decodeResponse(
    'searchRecallEvidence',
    await invoke<unknown>('searchRecallEvidence', { input }),
    decodeRecallSearchResult,
  );
}
export async function resolveRecallEvidence(
  input: ResolveRecallEvidenceInput,
): Promise<RecallEvidence | null> {
  return decodeResponse(
    'resolveRecallEvidence',
    await invoke<unknown>('resolveRecallEvidence', { input }),
    decodeResolvedRecallEvidence,
  );
}
export async function regenerateRecallDay(
  input: RecallDayInput,
): Promise<RecallJob> {
  return decodeResponse(
    'regenerateRecallDay',
    await invoke<unknown>('regenerateRecallDay', { input }),
    decodeRecallJob,
  );
}
export async function setRecallFeedback(
  input: SetRecallFeedbackInput,
): Promise<RecallDay> {
  return decodeResponse(
    'setRecallFeedback',
    await invoke<unknown>('setRecallFeedback', { input }),
    decodeRecallDay,
  );
}
export async function setRecallSessionCorrection(
  input: SetRecallSessionCorrectionInput,
): Promise<RecallDay> {
  return decodeResponse(
    'setRecallSessionCorrection',
    await invoke<unknown>('setRecallSessionCorrection', { input }),
    decodeRecallDay,
  );
}
export async function acceptRecallSuggestion(
  input: AcceptRecallSuggestionInput,
): Promise<RecallTask> {
  return decodeResponse(
    'acceptRecallSuggestion',
    await invoke<unknown>('acceptRecallSuggestion', { input }),
    decodeRecallTask,
  );
}
export async function updateRecallTask(
  input: UpdateRecallTaskInput,
): Promise<RecallTask> {
  return decodeResponse(
    'updateRecallTask',
    await invoke<unknown>('updateRecallTask', { input }),
    decodeRecallTask,
  );
}
export async function dismissRecallSuggestion(
  input: DismissRecallSuggestionInput,
): Promise<RecallDay> {
  return decodeResponse(
    'dismissRecallSuggestion',
    await invoke<unknown>('dismissRecallSuggestion', { input }),
    decodeRecallDay,
  );
}
export async function setRecallDayReviewed(
  input: SetRecallDayReviewedInput,
): Promise<RecallDay> {
  return decodeResponse(
    'setRecallDayReviewed',
    await invoke<unknown>('setRecallDayReviewed', { input }),
    decodeRecallDay,
  );
}
export async function getRecallStatus(): Promise<RecallStatus> {
  return decodeResponse(
    'getRecallStatus',
    await invoke<unknown>('getRecallStatus'),
    decodeRecallStatus,
  );
}
export async function cancelRecallSearch(): Promise<void> {
  const result = await invoke<unknown>('cancelRecallSearch');
  decodeResponse('cancelRecallSearch', result, (result) => {
    if (result !== null && result !== undefined) {
      throw new Error(
        'Recall returned an invalid response. Reload to reconnect.',
      );
    }
  });
}
export async function testRecallProvider(
  input: TestRecallProviderInput,
): Promise<RecallProviderTestResult> {
  const result = decodeResponse(
    'testRecallProvider',
    await invoke<unknown>('testRecallProvider', { input }),
    decodeRecallProviderTestResult,
  );
  if (result.status === 'failed')
    reportFrontendFailure('testRecallProvider', 'execute', result.errorCode, {
      elapsedMs: result.elapsedMs,
      deduplicate: false,
    });
  return result;
}
export async function controlRecallJob(
  input: ControlRecallJobInput,
): Promise<RecallJob> {
  return decodeResponse(
    'controlRecallJob',
    await invoke<unknown>('controlRecallJob', { input }),
    decodeRecallJob,
  );
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
      reportFrontendFailure('recall-state-changed', 'decode', error, {
        deduplicate: false,
      });
      if (onInvalid) {
        onInvalid(error);
        return;
      }
      throw error;
    }
    handler(metadata);
  });
}
