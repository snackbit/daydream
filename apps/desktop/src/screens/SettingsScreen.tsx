import { useEffect } from 'react';
import { LoaderCircle, Save } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { SettingsDangerZone } from '@/features/settings/components/settings-danger-zone';
import { SettingsScreenDialogs } from '@/features/settings/screen/settings-screen-dialogs';
import { SettingsScreenTabs } from '@/features/settings/screen/settings-screen-tabs';
import type {
  SettingsTab,
  SettingsFocus,
} from '@/features/settings/settings-location';
import { settingsAiModelActions } from '@/features/settings/use-settings-ai-controller';
import { useSettingsScreenController } from '@/features/settings/use-settings-screen-controller';

export interface SettingsScreenProps {
  tab?: SettingsTab | undefined;
  focusSection?: SettingsFocus | null | undefined;
  onTabChange?: ((tab: SettingsTab) => void) | undefined;
}
export function SettingsScreen({
  tab,
  focusSection,
  onTabChange,
}: SettingsScreenProps = {}) {
  const {
    aiActionError,
    aiModels,
    aiProviderStatus,
    blocker,
    busy,
    cadenceMode,
    canSave,
    cleanupConfirmation,
    confirmCleanup,
    discardAndLeaveSettings,
    dismissCleanupConfirmation,
    draft,
    error,
    loadStatus,
    retryLoadSettings,
    inputActivityRepairMessage,
    inputActivityRepairing,
    inputActivityStatus,
    requestSave,
    refreshInputActivityStatus,
    repairInputActivity,
    runAiModelAction,
    runModelTransferAction,
    modelActionErrors,
    savedScene,
    saving,
    setCadenceMode,
    setTranscriptionKeyDraft,
    setVisionKeyDraft,
    recallEmbeddingKeyDraft,
    recallGenerationKeyDraft,
    setRecallEmbeddingKeyDraft,
    setRecallGenerationKeyDraft,
    settingsState,
    successMessage,
    transcriptionKeyDraft,
    updateDraft,
    validationError,
    visionKeyDraft,
    saveAndLeaveSettings,
  } = useSettingsScreenController();
  const loaded = draft !== null && settingsState !== null;
  useEffect(() => {
    if (!loaded || !focusSection) return;
    const frame = window.requestAnimationFrame(() => {
      const section = document.getElementById(`ai-${focusSection}`);
      section?.scrollIntoView({ block: 'start' });
      section?.focus({ preventScroll: true });
    });
    return () => window.cancelAnimationFrame(frame);
  }, [focusSection, loaded, tab]);
  const { cancelAiModelDownload, deleteAiModel, downloadAiModel } =
    settingsAiModelActions;
  function handleCancelAiModelDownload(modelId: string) {
    void runAiModelAction(() => cancelAiModelDownload({ modelId }), modelId);
  }

  function handleDeleteAiModel(modelId: string) {
    void runAiModelAction(() => deleteAiModel({ modelId }), modelId);
  }

  function handleDownloadAiModel(modelId: string) {
    void runAiModelAction(() => downloadAiModel({ modelId }), modelId);
  }

  function handleRequestSave() {
    void requestSave();
  }

  function handleResetLeaveBlocker() {
    blocker.reset?.();
  }

  const remoteProcessingEnabled = Boolean(
    draft &&
    (draft.ai.transcription.provider === 'openai-compatible' ||
      draft.ai.vision.provider === 'openai-compatible' ||
      (draft.ai.recall.embedding.provider === 'openai-compatible' &&
        draft.ai.recall.embedding.enabled) ||
      draft.ai.recall.generation.enabled),
  );
  const saveStatus = saving
    ? 'Saving changes…'
    : busy
      ? 'Checking storage changes…'
      : validationError
        ? 'Review settings before saving'
        : canSave
          ? 'Unsaved changes'
          : successMessage
            ? 'Saved just now'
            : loadStatus === 'ready'
              ? 'All changes saved'
              : 'Loading settings';

  return (
    <PageLayout>
      <PageHeader
        title="Settings"
        description="Make Daydream work for your day. Changes are saved together."
        actions={
          <div className="flex flex-wrap items-center gap-3">
            <span
              aria-live="polite"
              className={
                validationError
                  ? 'text-destructive text-xs'
                  : 'text-muted-foreground text-xs'
              }
              role="status"
            >
              {saving || busy ? (
                <LoaderCircle className="mr-1 inline size-3 animate-spin" />
              ) : null}
              {saveStatus}
            </span>
            <Button disabled={!canSave} onClick={handleRequestSave} size="sm">
              {saving || busy ? (
                <LoaderCircle className="size-3.5 animate-spin" />
              ) : (
                <Save className="size-3.5" />
              )}
              {saving ? 'Saving…' : busy ? 'Checking…' : 'Save'}
            </Button>
          </div>
        }
      />
      <p className="text-muted-foreground text-sm leading-6">
        {remoteProcessingEnabled
          ? 'Remote AI is selected. Review the data sent in AI/LLM.'
          : 'Local processing is the default. Remote AI is opt-in in AI/LLM.'}
      </p>

      <SettingsScreenDialogs
        actions={{
          confirmCleanup,
          discardAndLeaveSettings,
          dismissCleanupConfirmation,
          resetLeaveBlocker: handleResetLeaveBlocker,
          saveAndLeaveSettings,
        }}
        model={{
          blockerStatus: blocker.status,
          busy,
          canSave,
          cleanupConfirmation,
          saving,
          validationError,
        }}
      />

      {error ? (
        <Alert variant="destructive">
          <AlertTitle>Settings failed</AlertTitle>
          <AlertDescription>{error}</AlertDescription>
        </Alert>
      ) : null}
      {successMessage ? (
        <p aria-live="polite" className="text-muted-foreground text-xs">
          {successMessage}
        </p>
      ) : null}
      {validationError ? (
        <p className="text-destructive text-xs" role="alert">
          {validationError}
        </p>
      ) : null}
      {aiActionError ? (
        <Alert variant="destructive">
          <AlertTitle>AI settings failed</AlertTitle>
          <AlertDescription>{aiActionError}</AlertDescription>
        </Alert>
      ) : null}

      {!draft || !settingsState ? (
        <div
          className="bg-muted text-muted-foreground rounded-panel grid gap-3 p-6 text-sm"
          role="status"
        >
          {loadStatus === 'error'
            ? 'Settings are unavailable.'
            : 'Loading settings.'}
          {loadStatus === 'error' ? (
            <Button
              className="w-fit"
              onClick={retryLoadSettings}
              variant="outline"
            >
              Try again
            </Button>
          ) : null}
        </div>
      ) : (
        <SettingsScreenTabs
          tab={tab}
          onTabChange={onTabChange}
          actions={{
            cancelAiModelDownload: handleCancelAiModelDownload,
            changeCadenceMode: setCadenceMode,
            changeTranscriptionKey: setTranscriptionKeyDraft,
            changeVisionKey: setVisionKeyDraft,
            changeRecallEmbeddingKey: setRecallEmbeddingKeyDraft,
            changeRecallGenerationKey: setRecallGenerationKeyDraft,
            deleteAiModel: handleDeleteAiModel,
            downloadAiModel: handleDownloadAiModel,
            transferAiModel: runModelTransferAction,
            refreshInputActivityStatus: refreshInputActivityStatus,
            repairInputActivity,
            updateDraft,
          }}
          busy={busy}
          model={{
            aiModels,
            modelActionErrors,
            aiProviderStatus,
            cadenceMode,
            draft,
            inputActivityRepairMessage,
            inputActivityRepairing,
            inputActivityStatus,
            limits: settingsState.limits,
            savedScene,
            transcriptionKeyDraft,
            visionKeyDraft,
            recallEmbeddingKeyDraft,
            recallGenerationKeyDraft,
          }}
        />
      )}
      <SettingsDangerZone disabled={busy || !loaded} />
    </PageLayout>
  );
}
