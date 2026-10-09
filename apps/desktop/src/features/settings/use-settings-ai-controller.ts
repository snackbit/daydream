import { useCallback, useEffect, useRef, useState } from 'react';
import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
  InputActivityStatus,
  SettingsCredentialDrafts,
} from '@daydream/shared';
import {
  cancelAiModelDownload,
  deleteAiModel,
  deleteAiProviderSecret,
  downloadAiModel,
  getAiProviderStatus,
  listenToAiModelsChanged,
  listAiModels,
} from '@/features/ai/ai-client';
import {
  pauseAiModelDownload,
  resumeAiModelDownload,
  removeAiModelPartial,
} from '@/features/ai/models/model-transfer-client';
import type { ModelTransferAction } from '@/features/ai/models/model-transfer-client';
import { modelTransferError } from '@/features/ai/models/model-transfer-format';
import {
  getInputActivityStatus,
  repairInputActivityPermissions,
} from '@/features/settings/settings-client';
import { screenErrorMessage } from '@/features/settings/settings-screen-utils';

interface AiSettingsSavePreparation {
  credentials: SettingsCredentialDrafts;
  settings: AppSettings;
}

interface UseSettingsAiControllerInput {
  loadNotifications: () => Promise<void>;
  selectedAiModelId: string | null;
}

export function useSettingsAiController({
  loadNotifications,
  selectedAiModelId,
}: UseSettingsAiControllerInput) {
  const [aiActionError, setAiActionError] = useState<string | null>(null);
  const [scaffoldError, setScaffoldError] = useState<string | null>(null);
  const [aiModels, setAiModels] = useState<AiModelAsset[]>([]);
  const [modelActionErrors, setModelActionErrors] = useState<
    Record<string, string>
  >({});
  const pendingModels = useRef(new Set<string>());
  const refreshOwnership = useRef(0);
  const alive = useRef(true);
  const [aiProviderStatus, setAiProviderStatus] =
    useState<AiProviderStatus | null>(null);
  const [inputActivityRepairMessage, setInputActivityRepairMessage] = useState<
    string | null
  >(null);
  const [inputActivityRepairing, setInputActivityRepairing] = useState(false);
  const [inputActivityStatus, setInputActivityStatus] =
    useState<InputActivityStatus | null>(null);
  const [transcriptionKeyDraft, setTranscriptionKeyDraft] = useState('');
  const [visionKeyDraft, setVisionKeyDraft] = useState('');
  const [recallEmbeddingKeyDraft, setRecallEmbeddingKeyDraft] = useState('');
  const [recallGenerationKeyDraft, setRecallGenerationKeyDraft] = useState('');
  const pendingSecretRefsForDeletion = useRef<string[]>([]);

  const refreshAiScaffolds = useCallback(async () => {
    const request = ++refreshOwnership.current;
    try {
      const [modelList, providerStatus, nextInputActivityStatus] =
        await Promise.allSettled([
          listAiModels(),
          getAiProviderStatus(),
          getInputActivityStatus(),
        ]);
      if (!alive.current || request !== refreshOwnership.current) return;
      if (modelList.status === 'fulfilled') setAiModels(modelList.value.models);
      if (providerStatus.status === 'fulfilled')
        setAiProviderStatus(providerStatus.value);
      if (nextInputActivityStatus.status === 'fulfilled')
        setInputActivityStatus(nextInputActivityStatus.value);
      setScaffoldError(
        [modelList, providerStatus, nextInputActivityStatus].some(
          (result) => result.status === 'rejected',
        )
          ? 'Some AI status could not be refreshed. Refocus this window to retry.'
          : null,
      );
    } catch (error) {
      if (alive.current && request === refreshOwnership.current)
        setScaffoldError(screenErrorMessage(error));
    }
  }, []);

  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
      refreshOwnership.current += 1;
    };
  }, []);

  useEffect(() => {
    if (!selectedAiModelId) {
      return;
    }

    const timeoutId = window.setTimeout(() => void refreshAiScaffolds(), 0);

    return () => window.clearTimeout(timeoutId);
  }, [refreshAiScaffolds, selectedAiModelId]);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    let disposed = false;
    let selectedStatus: string | null = null;
    let providerRequest = 0;

    void listenToAiModelsChanged((modelList) => {
      if (disposed) return;
      refreshOwnership.current += 1;
      setAiModels(modelList.models);
      const selected = modelList.models.find(
        (model) => model.id === selectedAiModelId,
      );
      const nextStatus = selected
        ? `${selected.downloadStatus}:${selected.selfTestStatus}`
        : null;
      if (
        selected &&
        selectedStatus !== nextStatus &&
        ['available', 'failed', 'cancelled', 'paused'].includes(
          selected.downloadStatus,
        )
      ) {
        const request = ++providerRequest;
        void getAiProviderStatus()
          .then((status) => {
            if (!disposed && request === providerRequest)
              setAiProviderStatus(status);
          })
          .catch(() => {
            if (!disposed)
              setScaffoldError(
                'AI readiness could not be refreshed. Refocus this window to retry.',
              );
          });
      }
      selectedStatus = nextStatus;
    }).then((nextUnlisten) => {
      if (disposed) {
        nextUnlisten();
      } else {
        unlisten = nextUnlisten;
      }
    });

    return () => {
      disposed = true;
      unlisten?.();
    };
  }, [selectedAiModelId]);

  useEffect(() => {
    function handleFocus() {
      void refreshAiScaffolds();
    }

    window.addEventListener('focus', handleFocus);

    return () => window.removeEventListener('focus', handleFocus);
  }, [refreshAiScaffolds]);

  async function prepareAiSettingsForSave(
    settings: AppSettings,
  ): Promise<AiSettingsSavePreparation> {
    setAiActionError(null);
    return {
      settings,
      credentials: {
        ...(transcriptionKeyDraft.trim()
          ? { transcriptionKey: transcriptionKeyDraft }
          : {}),
        ...(visionKeyDraft.trim() ? { visionKey: visionKeyDraft } : {}),
        ...(recallEmbeddingKeyDraft.trim()
          ? { recallEmbeddingKey: recallEmbeddingKeyDraft }
          : {}),
        ...(recallGenerationKeyDraft.trim()
          ? { recallGenerationKey: recallGenerationKeyDraft }
          : {}),
      },
    };
  }

  async function runAiModelAction(
    action: () => Promise<unknown>,
    modelId?: string,
  ) {
    const key = modelId ?? 'model-action';
    if (pendingModels.current.has(key)) return;
    pendingModels.current.add(key);
    try {
      if (modelId)
        setModelActionErrors((current) => {
          const next = { ...current };
          delete next[modelId];
          return next;
        });
      else setAiActionError(null);
      await action();
      await Promise.all([
        refreshAiScaffolds(),
        loadNotifications().catch(() => {
          if (alive.current)
            setScaffoldError(
              'Notifications could not be refreshed. Your model action was saved.',
            );
        }),
      ]);
    } catch (error) {
      if (!alive.current) return;
      if (modelId)
        setModelActionErrors((current) => ({
          ...current,
          [modelId]: modelTransferError(error),
        }));
      else setAiActionError(modelTransferError(error));
    } finally {
      pendingModels.current.delete(key);
    }
  }

  async function refreshInputActivityStatus() {
    try {
      setAiActionError(null);
      setInputActivityStatus(await getInputActivityStatus());
    } catch (error) {
      setAiActionError(screenErrorMessage(error));
    }
  }

  async function runModelTransferAction(
    modelId: string,
    action: ModelTransferAction,
  ) {
    const commands = {
      download: downloadAiModel,
      cancel: cancelAiModelDownload,
      delete: deleteAiModel,
      pause: pauseAiModelDownload,
      resume: resumeAiModelDownload,
      remove: removeAiModelPartial,
    };
    return runAiModelAction(() => commands[action]({ modelId }), modelId);
  }

  async function repairInputActivity() {
    try {
      setAiActionError(null);
      setInputActivityRepairMessage(null);
      setInputActivityRepairing(true);
      const result = await repairInputActivityPermissions();
      setInputActivityStatus(result.status);
      setInputActivityRepairMessage(result.message);
      if (!result.success) {
        setAiActionError(result.message);
      }
    } catch (error) {
      setAiActionError(screenErrorMessage(error));
    } finally {
      setInputActivityRepairing(false);
    }
  }

  async function deletePendingAiSecrets() {
    const secretRefs = pendingSecretRefsForDeletion.current;
    pendingSecretRefsForDeletion.current = [];

    await deleteAiSecrets(secretRefs);
  }

  const clearSecretDrafts = useCallback(() => {
    setTranscriptionKeyDraft('');
    setVisionKeyDraft('');
    setRecallEmbeddingKeyDraft('');
    setRecallGenerationKeyDraft('');
  }, []);
  function clearSavedSecretDrafts(credentials: SettingsCredentialDrafts) {
    setTranscriptionKeyDraft((current) =>
      current === credentials.transcriptionKey ? '' : current,
    );
    setVisionKeyDraft((current) =>
      current === credentials.visionKey ? '' : current,
    );
    setRecallEmbeddingKeyDraft((current) =>
      current === credentials.recallEmbeddingKey ? '' : current,
    );
    setRecallGenerationKeyDraft((current) =>
      current === credentials.recallGenerationKey ? '' : current,
    );
  }

  const clearPendingSecretRefsForDeletion = useCallback(() => {
    pendingSecretRefsForDeletion.current = [];
  }, []);

  function setPendingSecretRefsForDeletion(secretRefs: string[]) {
    pendingSecretRefsForDeletion.current = secretRefs;
  }

  return {
    aiActionError: aiActionError ?? scaffoldError,
    aiModels,
    modelActionErrors,
    aiProviderStatus,
    clearPendingSecretRefsForDeletion,
    clearSecretDrafts,
    clearSavedSecretDrafts,
    deletePendingAiSecrets,
    hasAiSecretDraft: Boolean(
      transcriptionKeyDraft.trim() ||
      visionKeyDraft.trim() ||
      recallEmbeddingKeyDraft.trim() ||
      recallGenerationKeyDraft.trim(),
    ),
    prepareAiSettingsForSave,
    refreshAiScaffolds,
    refreshInputActivityStatus,
    repairInputActivity,
    runAiModelAction,
    runModelTransferAction,
    setPendingSecretRefsForDeletion,
    setTranscriptionKeyDraft,
    setVisionKeyDraft,
    setRecallEmbeddingKeyDraft,
    setRecallGenerationKeyDraft,
    recallEmbeddingKeyDraft,
    recallGenerationKeyDraft,
    inputActivityRepairMessage,
    inputActivityRepairing,
    inputActivityStatus,
    transcriptionKeyDraft,
    visionKeyDraft,
  };
}

async function deleteAiSecrets(secretRefs: string[]) {
  await Promise.all(
    secretRefs.map((secretRef) => deleteAiProviderSecret({ secretRef })),
  );
}

export const settingsAiModelActions = {
  cancelAiModelDownload,
  deleteAiModel,
  downloadAiModel,
};
