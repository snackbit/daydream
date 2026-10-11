import type { AppSettings, SettingsCredentialDrafts } from '@daydream/shared';

interface SaveQueueOwner {
  read: () => {
    draft: AppSettings | null;
    generation: number;
    activeSave: Promise<void> | null;
  };
  save: (
    settings: AppSettings,
    credentials: SettingsCredentialDrafts,
    applyStorageCleanup: boolean,
  ) => Promise<void>;
}

function remainingCredentials(
  pending: SettingsCredentialDrafts,
  submitted: SettingsCredentialDrafts,
): SettingsCredentialDrafts {
  const next: SettingsCredentialDrafts = {};
  for (const key of [
    'transcriptionKey',
    'visionKey',
    'recallEmbeddingKey',
    'recallGenerationKey',
  ] as const) {
    const value = pending[key];
    if (value !== undefined && value !== submitted[key]) next[key] = value;
  }
  const keys = Object.fromEntries(
    Object.entries(pending.modelKeys ?? {}).filter(
      ([id, key]) => key !== submitted.modelKeys?.[id],
    ),
  );
  if (Object.keys(keys).length) next.modelKeys = keys;
  return next;
}

/** One app-scoped writer. Component unmounts do not abandon accepted edits. */
export function createSettingsSaveQueue(owner: SaveQueueOwner) {
  let revision = 0,
    completed = 0,
    cleanupRevision = 0,
    appliedCleanupRevision = 0;
  let credentials: SettingsCredentialDrafts = {};
  let running: Promise<boolean> | null = null;
  let paused = false;
  let epoch = 0;

  function run(): Promise<boolean> {
    if (running) return running;
    if (paused) return Promise.resolve(false);
    const ownedEpoch = epoch;
    const generation = owner.read().generation;
    const operation = async () => {
      try {
        while (completed < revision) {
          const active = owner.read().activeSave;
          if (active) await active;
          if (epoch !== ownedEpoch || owner.read().generation !== generation)
            return false;
          const draft = owner.read().draft;
          if (!draft) return false;
          const submittedRevision = revision,
            submittedCleanup = cleanupRevision;
          const submittedCredentials = credentials;
          await owner.save(
            draft,
            submittedCredentials,
            submittedCleanup > appliedCleanupRevision,
          );
          if (epoch !== ownedEpoch || owner.read().generation !== generation)
            return false;
          completed = submittedRevision;
          appliedCleanupRevision = submittedCleanup;
          credentials = remainingCredentials(credentials, submittedCredentials);
        }
        return true;
      } catch {
        if (epoch === ownedEpoch) paused = true;
        return false;
      }
    };
    const promise = operation();
    running = promise;
    void promise.finally(() => {
      if (running === promise) running = null;
    });
    return promise;
  }

  return {
    enqueue(next: SettingsCredentialDrafts, applyStorageCleanup: boolean) {
      revision++;
      if (applyStorageCleanup) cleanupRevision++;
      const modelKeys = { ...credentials.modelKeys, ...next.modelKeys };
      credentials = {
        ...credentials,
        ...next,
        ...(Object.keys(modelKeys).length ? { modelKeys } : {}),
      };
      return run();
    },
    retry() {
      paused = false;
      return run();
    },
    reset() {
      epoch++;
      revision = 0;
      completed = 0;
      cleanupRevision = 0;
      appliedCleanupRevision = 0;
      credentials = {};
      paused = false;
      running = null;
    },
  };
}
