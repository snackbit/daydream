import { useEffect } from 'react';
import { LoaderCircle, RotateCcw, Save } from 'lucide-react';
import { PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
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
    inputActivityRepairMessage,
    inputActivityRepairing,
    inputActivityStatus,
    requestReset,
    requestSave,
    resetOpen,
    refreshInputActivityStatus,
    repairInputActivity,
    runAiModelAction,
    savedScene,
    saving,
    setCadenceMode,
    setResetOpen,
    setTranscriptionKeyDraft,
    setVisionKeyDraft,
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
    void runAiModelAction(() => cancelAiModelDownload({ modelId }));
  }

  function handleDeleteAiModel(modelId: string) {
    void runAiModelAction(() => deleteAiModel({ modelId }));
  }

  function handleDownloadAiModel(modelId: string) {
    void runAiModelAction(() => downloadAiModel({ modelId }));
  }

  function handleRequestSave() {
    void requestSave();
  }

  function handleOpenReset() {
    setResetOpen(true);
  }

  function handleResetLeaveBlocker() {
    blocker.reset?.();
  }

  const remoteProcessingEnabled = Boolean(
    draft &&
    (draft.ai.transcription.provider === 'openai-compatible' ||
      draft.ai.vision.provider === 'openai-compatible'),
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
    <PageLayout className="max-w-none gap-3 lg:gap-3">
      <div className="bg-background/95 sticky top-0 z-20 flex flex-wrap items-center justify-between gap-x-4 gap-y-2 border-b py-2 backdrop-blur-sm">
        <div className="flex min-w-0 flex-wrap items-center gap-x-3 gap-y-1">
          <h1 className="text-[13px] font-semibold">Settings</h1>
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
        </div>
        <div className="flex shrink-0 items-center gap-1.5">
          <Button
            disabled={busy || !draft}
            onClick={handleOpenReset}
            size="sm"
            variant="outline"
          >
            <RotateCcw className="size-3" />
            Reset
          </Button>
          <Button disabled={!canSave} onClick={handleRequestSave} size="sm">
            {saving || busy ? (
              <LoaderCircle className="size-3.5 animate-spin" />
            ) : (
              <Save className="size-3.5" />
            )}
            {saving ? 'Saving…' : busy ? 'Checking…' : 'Save'}
          </Button>
        </div>
        <p className="text-muted-foreground w-full text-[10px] leading-4">
          {remoteProcessingEnabled
            ? 'Remote AI is selected. Review the data sent in AI/LLM.'
            : 'Local processing is the default. Remote AI is opt-in in AI/LLM.'}
        </p>
      </div>

      <SettingsScreenDialogs
        actions={{
          confirmCleanup,
          discardAndLeaveSettings,
          dismissCleanupConfirmation,
          requestReset,
          resetLeaveBlocker: handleResetLeaveBlocker,
          saveAndLeaveSettings,
          setResetOpen,
        }}
        model={{
          blockerStatus: blocker.status,
          busy,
          canSave,
          cleanupConfirmation,
          resetOpen,
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
        <div className="text-muted-foreground border p-4 text-sm">
          {loadStatus === 'error'
            ? 'Settings are unavailable.'
            : 'Loading settings.'}
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
            deleteAiModel: handleDeleteAiModel,
            downloadAiModel: handleDownloadAiModel,
            refreshInputActivityStatus: refreshInputActivityStatus,
            repairInputActivity,
            updateDraft,
          }}
          busy={busy}
          model={{
            aiModels,
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
          }}
        />
      )}
    </PageLayout>
  );
}
