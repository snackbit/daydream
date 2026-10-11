import { useCallback, useState } from 'react';

export function useModelKeyDrafts() {
  const [modelKeyDrafts, setModelKeyDrafts] = useState<Record<string, string>>(
    {},
  );
  const changeModelKey = useCallback(
    (id: string, value: string) =>
      setModelKeyDrafts((current) => ({ ...current, [id]: value })),
    [],
  );
  const clearModelKeys = useCallback(() => setModelKeyDrafts({}), []);
  function clearSavedModelKeys(submitted: Record<string, string>) {
    setModelKeyDrafts((current) =>
      Object.fromEntries(
        Object.entries(current).filter(([id, key]) => submitted[id] !== key),
      ),
    );
  }
  return {
    modelKeyDrafts,
    changeModelKey,
    clearModelKeys,
    clearSavedModelKeys,
    hasModelKeyDraft: Object.values(modelKeyDrafts).some((key) => key.trim()),
  };
}
