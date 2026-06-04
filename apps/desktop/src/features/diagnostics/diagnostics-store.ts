import type { DaydreamDiagnostics } from '@daydream/shared';
import { create } from 'zustand';
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
    set({ error: null, status: 'loading' });

    try {
      const diagnostics = await getDiagnostics();
      set({ diagnostics, status: 'ready' });
    } catch (error) {
      set({
        error: error instanceof Error ? error.message : String(error),
        status: 'error',
      });
    }
  },
  status: 'idle',
}));
