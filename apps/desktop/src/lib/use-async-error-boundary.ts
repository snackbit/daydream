import { useCallback, useState } from 'react';

export function useAsyncErrorBoundary(): (
  action: () => Promise<unknown>,
) => void {
  const [error, setError] = useState<unknown>(null);

  if (error) {
    throw error;
  }

  return useCallback((action: () => Promise<unknown>) => {
    void action().catch(setError);
  }, []);
}
