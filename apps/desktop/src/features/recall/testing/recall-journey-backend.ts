import type { RecallTask } from '@daydream/shared';
import {
  defaultSettingsState,
  scene,
  availableInputActivityStatus,
} from '@/features/settings/testing/settings-browser-fixtures';
import { readyStatus } from '@/screens/capture-browser-fixture';
import {
  recallDayFixture,
  recallEvidenceFixture,
  recallSearchFixture,
  recallStatusFixture,
} from './recall-browser-fixtures';

function nativeInputRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function input(args: unknown): Record<string, unknown> {
  if (typeof args !== 'object' || args === null || !('input' in args))
    throw new Error('Missing synthetic IPC input');
  const value = args.input;
  if (!nativeInputRecord(value)) throw new Error('Invalid synthetic IPC input');
  return value;
}
function text(value: unknown): string {
  if (typeof value !== 'string') throw new Error('Invalid synthetic string');
  return value;
}
function optionalText(value: unknown): string | null {
  return value === null ? null : text(value);
}

/** Synthetic native replies only. Components, stores, router and client decoders remain real. */
export function createRecallJourneyBackend() {
  const state = {
    day: structuredClone(recallDayFixture),
    status: structuredClone(recallStatusFixture),
    search: structuredClone(recallSearchFixture),
    searchNext: structuredClone(recallSearchFixture),
    failures: new Set<string>(),
  };
  async function invoke(command: string, args?: unknown): Promise<unknown> {
    if (state.failures.delete(command))
      throw new Error('Synthetic local operation failed');
    switch (command) {
      case 'getRecallDay':
        return structuredClone(state.day);
      case 'getRecallStatus':
        return structuredClone(state.status);
      case 'listRecallDays':
        return [structuredClone(state.day.day)];
      case 'searchRecallEvidence':
        return structuredClone(
          input(args).cursor ? state.searchNext : state.search,
        );
      case 'cancelRecallSearch':
        return undefined;
      case 'listLibraryRecordings':
        return [];
      case 'getAppSettings':
        return structuredClone(defaultSettingsState);
      case 'getInputActivityStatus':
        return structuredClone(availableInputActivityStatus);
      case 'getRecorderScene':
        return structuredClone(scene);
      case 'getRecorderStatus':
        return structuredClone(readyStatus);
      case 'getRecorderPerformanceStats':
        return {
          state: 'idle',
          desiredFps: null,
          segmentStartedAtMs: null,
          timing: null,
          updatedAtMs: 0,
        };
      case 'getAppStartupStatus':
        return {
          phase: 'ready',
          activeStageId: null,
          error: null,
          warnings: [],
          stages: [],
        };
      case 'getNotifications':
        return { notifications: [] };
      case 'listBackgroundTasks':
        return { tasks: [], nextCursor: null };
      case 'listAiModels':
        return { models: [] };
      case 'setRecallSessionCorrection': {
        const values = input(args);
        state.day.sessions = state.day.sessions.map((session) => ({
          ...session,
          correction: optionalText(values.correction),
          titleCorrection: optionalText(values.titleCorrection),
          categoryCorrection: optionalText(values.categoryCorrection),
        }));
        return structuredClone(state.day);
      }
      case 'setRecallFeedback': {
        const values = input(args);
        const rating = values.rating;
        if (rating !== null && rating !== 'helpful' && rating !== 'unhelpful')
          throw new Error('Invalid synthetic rating');
        state.day.feedback = [
          {
            targetId: state.day.day.dayId,
            rating,
            comment: text(values.comment),
          },
        ];
        return structuredClone(state.day);
      }
      case 'setRecallDayReviewed': {
        const values = input(args);
        if (typeof values.reviewed !== 'boolean')
          throw new Error('Invalid synthetic review');
        state.day.day.reviewed = values.reviewed;
        return structuredClone(state.day);
      }
      case 'acceptRecallSuggestion': {
        const task: RecallTask = {
          id: 'task-journey',
          dayId: state.day.day.dayId,
          title: 'Review focus behavior',
          notes: '',
          status: 'open',
          citations: [recallEvidenceFixture],
          sourceWithheld: false,
          updatedAtMs: 2,
        };
        state.day.tasks = [task];
        state.day.suggestions = state.day.suggestions.map((suggestion) => ({
          ...suggestion,
          acceptedTaskId: task.id,
        }));
        return structuredClone(task);
      }
      case 'dismissRecallSuggestion': {
        state.day.suggestions = state.day.suggestions.map((suggestion) => ({
          ...suggestion,
          dismissed: true,
        }));
        return structuredClone(state.day);
      }
      case 'updateRecallTask': {
        const values = input(args);
        const status = values.status;
        if (status !== 'open' && status !== 'done' && status !== 'dismissed')
          throw new Error('Invalid synthetic task state');
        state.day.tasks = state.day.tasks.map((task) => ({
          ...task,
          title: text(values.title),
          notes: text(values.notes),
          status,
        }));
        return structuredClone(state.day.tasks[0]);
      }
      case 'regenerateRecallDay':
        return {
          id: 'generation-journey',
          dayId: state.day.day.dayId,
          kind: 'generation',
          status: 'pending',
          progress: 0,
          errorCode: null,
          updatedAtMs: 3,
        };
      default:
        throw new Error(`Unconfigured synthetic boundary: ${command}`);
    }
  }
  return { state, invoke };
}
