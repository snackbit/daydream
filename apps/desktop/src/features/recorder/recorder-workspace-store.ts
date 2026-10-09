import { create } from 'zustand';
import { recorderWorkspaceInitialState } from './recorder-workspace-initial-state';
import { createRecorderWorkspaceLifecycleActions } from './recorder-workspace-lifecycle-actions';
import { createRecorderWorkspacePersistenceActions } from './recorder-workspace-persistence-actions';
import { createRecorderWorkspacePreviewActions } from './recorder-workspace-preview-actions';
import { createRecorderWorkspaceSceneActions } from './recorder-workspace-scene-actions';
import type { RecorderWorkspaceState } from './recorder-workspace-store-types';

export type {
  AsyncStatus,
  RecorderLifecycleAction,
  RecorderWorkspacePatch,
  RecorderWorkspaceSet,
  RecorderWorkspaceState,
} from './recorder-workspace-store-types';

export function createRecorderWorkspaceStore() {
  return create<RecorderWorkspaceState>((set, get) => ({
    ...recorderWorkspaceInitialState,
    ...createRecorderWorkspacePersistenceActions(set, get),
    ...createRecorderWorkspaceSceneActions(set, get),
    ...createRecorderWorkspacePreviewActions(set, get),
    ...createRecorderWorkspaceLifecycleActions(set, get),
    invalidatePreferences: () =>
      set({
        ...recorderWorkspaceInitialState,
        loadStatus: 'loading',
        workspaceRevision: get().workspaceRevision + 1,
      }),
  }));
}
export const useRecorderWorkspaceStore = createRecorderWorkspaceStore();
