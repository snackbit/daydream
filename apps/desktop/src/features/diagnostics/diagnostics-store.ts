import type { DaydreamDiagnostics } from '@daydream/shared';
import { create } from 'zustand';
import {
  runAsyncStoreAction,
  unknownErrorMessage,
} from '@/lib/async-store-action';
import { getDiagnostics } from './diagnostics-client';

type DiagnosticsStatus = 'idle' | 'loading' | 'ready' | 'error';

interface DiagnosticsState {
  diagnostics: DaydreamDiagnostics | null;
  error: string | null;
  load: () => Promise<void>;
  status: DiagnosticsStatus;
}

export const useDiagnosticsStore = create<DiagnosticsState>((set) => ({
  diagnostics: null,
  error: null,
  async load() {
    await runAsyncStoreAction({
      before: () => set({ error: null, status: 'loading' }),
      failure: (error) =>
        set({
          error: unknownErrorMessage(error),
          status: 'error',
        }),
      run: () => getDiagnostics(),
      success: (diagnostics) => set({ diagnostics, status: 'ready' }),
    });
  },
  status: 'idle',
}));
