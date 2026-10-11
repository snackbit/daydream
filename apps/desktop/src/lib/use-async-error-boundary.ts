import { useCallback, useState } from 'react';
import { reportFrontendFailure } from './diagnostics/failure-logging';

export function useAsyncErrorBoundary(): (
  action: () => Promise<unknown>,
) => void {
  const [error, setError] = useState<unknown>(null);

  if (error) {
    throw error;
  }

  return useCallback((action: () => Promise<unknown>) => {
    void action().catch((error: unknown) => {
      reportFrontendFailure('frontend_action', 'execute', error);
      setError(error);
    });
  }, []);
}
