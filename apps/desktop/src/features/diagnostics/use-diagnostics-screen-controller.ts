import { useCallback, useEffect, useState } from 'react';
import type { DaydreamDiagnostics } from '@daydream/shared';
import { getDebugLogBundle } from './diagnostics-client';
import { useDiagnosticsStore } from './diagnostics-store';

type CopyStatus = 'copied' | 'failed' | 'idle';

export interface DiagnosticsScreenModel {
  bundleCopyStatus: CopyStatus;
  copyStatus: CopyStatus;
  diagnostics: DaydreamDiagnostics | null;
  error: string | null;
  status: 'error' | 'idle' | 'loading' | 'ready';
}

export interface DiagnosticsScreenActions {
  copyDebugLogBundle: () => Promise<void>;
  copyRawDiagnosticsJson: () => Promise<void>;
  refresh: () => void;
}

export interface DiagnosticsScreenController {
  actions: DiagnosticsScreenActions;
  model: DiagnosticsScreenModel;
}

export function useDiagnosticsScreenController(
  debugEnabled: boolean,
): DiagnosticsScreenController {
  const diagnostics = useDiagnosticsStore((state) => state.diagnostics);
  const error = useDiagnosticsStore((state) => state.error);
  const load = useDiagnosticsStore((state) => state.load);
  const status = useDiagnosticsStore((state) => state.status);
  const [copyStatus, setCopyStatus] = useState<CopyStatus>('idle');
  const [bundleCopyStatus, setBundleCopyStatus] = useState<CopyStatus>('idle');

  useEffect(() => {
    if (debugEnabled && status === 'idle') {
      void load().catch(() => undefined);
    }
  }, [debugEnabled, load, status]);

  const copyRawDiagnosticsJson = useCallback(async () => {
    if (!diagnostics) {
      return;
    }

    try {
      await navigator.clipboard.writeText(JSON.stringify(diagnostics, null, 2));
      setCopyStatus('copied');
    } catch (_error) {
      setCopyStatus('failed');
    }
  }, [diagnostics]);

  const copyDebugLogBundle = useCallback(async () => {
    try {
      const bundle = await getDebugLogBundle();
      await navigator.clipboard.writeText(JSON.stringify(bundle, null, 2));
      setBundleCopyStatus('copied');
    } catch (_error) {
      setBundleCopyStatus('failed');
    }
  }, []);

  const refresh = useCallback(() => {
    void load().catch(() => undefined);
  }, [load]);

  return {
    actions: {
      copyDebugLogBundle,
      copyRawDiagnosticsJson,
      refresh,
    },
    model: {
      bundleCopyStatus,
      copyStatus,
      diagnostics,
      error,
      status,
    },
  };
}
