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
  const [aiModels, setAiModels] = useState<AiModelAsset[]>([]);
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
  const pendingSecretRefsForDeletion = useRef<string[]>([]);

  const refreshAiScaffolds = useCallback(async () => {
    try {
      const [modelList, providerStatus, nextInputActivityStatus] =
        await Promise.all([
          listAiModels(),
          getAiProviderStatus(),
          getInputActivityStatus(),
        ]);
      setAiActionError(null);
      setAiModels(modelList.models);
      setAiProviderStatus(providerStatus);
      setInputActivityStatus(nextInputActivityStatus);
    } catch (error) {
      setAiActionError(screenErrorMessage(error));
    }
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

    void listenToAiModelsChanged((modelList) => {
      setAiModels(modelList.models);
      void getAiProviderStatus()
        .then(setAiProviderStatus)
        .catch((error) => setAiActionError(screenErrorMessage(error)));
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
  }, []);

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
      },
    };
  }

  async function runAiModelAction(action: () => Promise<unknown>) {
    try {
      setAiActionError(null);
      await action();
      await Promise.all([refreshAiScaffolds(), loadNotifications()]);
    } catch (error) {
      setAiActionError(screenErrorMessage(error));
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

  function clearSecretDrafts() {
    setTranscriptionKeyDraft('');
    setVisionKeyDraft('');
  }

  function clearPendingSecretRefsForDeletion() {
    pendingSecretRefsForDeletion.current = [];
  }

  function setPendingSecretRefsForDeletion(secretRefs: string[]) {
    pendingSecretRefsForDeletion.current = secretRefs;
  }

  return {
    aiActionError,
    aiModels,
    aiProviderStatus,
    clearPendingSecretRefsForDeletion,
    clearSecretDrafts,
    deletePendingAiSecrets,
    hasAiSecretDraft: Boolean(
      transcriptionKeyDraft.trim() || visionKeyDraft.trim(),
    ),
    prepareAiSettingsForSave,
    refreshAiScaffolds,
    refreshInputActivityStatus,
    repairInputActivity,
    runAiModelAction,
    setPendingSecretRefsForDeletion,
    setTranscriptionKeyDraft,
    setVisionKeyDraft,
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
