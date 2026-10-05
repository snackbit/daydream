import { RotateCcw, Save } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from '@/components/ui/dialog';
import { FieldSet } from '@/components/ui/field';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AiSettingsTab } from '@/features/settings/components/ai-settings-tab';
import { GeneralSettingsTab } from '@/features/settings/components/general-settings-tab';
import {
  cadenceModeForFps,
  RecordSettingsTab,
} from '@/features/settings/components/record-settings-tab';
import { SettingsLeaveDialog } from '@/features/settings/components/settings-leave-dialog';
import { StorageSettingsTab } from '@/features/settings/components/storage-settings-tab';
import {
  estimateOneHourRecordingBytes,
  formatSceneDimensions,
  formatStorageSize,
} from '@/features/settings/settings-format';
import { formatSessionCount } from '@/features/settings/settings-screen-utils';
import { settingsAiModelActions } from '@/features/settings/use-settings-ai-controller';
import { useSettingsScreenController } from '@/features/settings/use-settings-screen-controller';

export function SettingsScreen() {
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
  const { cancelAiModelDownload, deleteAiModel, downloadAiModel } =
    settingsAiModelActions;

  return (
    <PageLayout>
      <PageHeader
        title="Settings"
        description="Capture and library defaults stored locally in Daydream."
        actions={
          <>
            <Dialog open={resetOpen} onOpenChange={setResetOpen}>
              <DialogTrigger asChild>
                <Button disabled={busy || !draft} variant="outline">
                  <RotateCcw className="size-3.5" />
                  Reset
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>Reset settings?</DialogTitle>
                  <DialogDescription>
                    This restores capture and storage settings to defaults. It
                    does not delete recordings or scenes.
                  </DialogDescription>
                </DialogHeader>
                <DialogFooter>
                  <DialogClose asChild>
                    <Button disabled={busy} variant="outline">
                      Cancel
                    </Button>
                  </DialogClose>
                  <Button
                    disabled={busy}
                    onClick={() => void requestReset()}
                    variant="destructive"
                  >
                    Reset settings
                  </Button>
                </DialogFooter>
              </DialogContent>
            </Dialog>
            <Button disabled={!canSave} onClick={() => void requestSave()}>
              <Save className="size-3.5" />
              Save
            </Button>
          </>
        }
      />

      <Dialog
        open={Boolean(cleanupConfirmation)}
        onOpenChange={(open) => {
          if (!open) {
            dismissCleanupConfirmation();
          }
        }}
      >
        <DialogContent>
          <DialogHeader>
            <DialogTitle>Delete old recordings?</DialogTitle>
            <DialogDescription>
              This would delete{' '}
              {cleanupConfirmation
                ? formatSessionCount(cleanupConfirmation.preview.recordingCount)
                : '0 sessions'}{' '}
              ({formatStorageSize(cleanupConfirmation?.preview.sizeBytes ?? 0)})
              from the local library.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button
              disabled={saving}
              onClick={dismissCleanupConfirmation}
              variant="outline"
            >
              Cancel
            </Button>
            <Button
              disabled={saving}
              onClick={() => void confirmCleanup()}
              variant="destructive"
            >
              {cleanupConfirmation?.action === 'reset'
                ? 'Delete and reset'
                : 'Delete and save'}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <SettingsLeaveDialog
        busy={busy}
        canSave={canSave}
        onDiscard={discardAndLeaveSettings}
        onOpenChange={(open) => {
          if (!open && !busy) {
            blocker.reset?.();
          }
        }}
        onSave={() => void saveAndLeaveSettings()}
        onStay={() => blocker.reset?.()}
        open={blocker.status === 'blocked' && !cleanupConfirmation}
        validationError={validationError}
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
        <FieldSet disabled={busy} aria-busy={busy}>
          <Tabs defaultValue="general">
            <TabsList aria-label="Settings sections">
              <TabsTrigger value="general">General</TabsTrigger>
              <TabsTrigger value="record">Record</TabsTrigger>
              <TabsTrigger value="storage">Storage/Library</TabsTrigger>
              <TabsTrigger value="ai">AI/LLM</TabsTrigger>
            </TabsList>

            <TabsContent value="general">
              <GeneralSettingsTab draft={draft} onUpdateDraft={updateDraft} />
            </TabsContent>

            <TabsContent value="record">
              <RecordSettingsTab
                cadenceMode={cadenceMode ?? cadenceModeForFps(draft.record.fps)}
                draft={draft}
                limits={settingsState.limits}
                sceneDimensions={formatSceneDimensions(savedScene)}
                oneHourEstimate={formatStorageSize(
                  estimateOneHourRecordingBytes(draft, savedScene),
                )}
                onChangeCadenceMode={setCadenceMode}
                onUpdateDraft={updateDraft}
              />
            </TabsContent>

            <TabsContent value="storage">
              <StorageSettingsTab
                draft={draft}
                limits={settingsState.limits}
                onUpdateDraft={updateDraft}
              />
            </TabsContent>

            <TabsContent value="ai">
              <AiSettingsTab
                draft={draft}
                limits={settingsState.limits}
                models={aiModels}
                onCancelModelDownload={(modelId) =>
                  void runAiModelAction(() =>
                    cancelAiModelDownload({ modelId }),
                  )
                }
                onChangeTranscriptionKey={setTranscriptionKeyDraft}
                onChangeVisionKey={setVisionKeyDraft}
                onDeleteModel={(modelId) =>
                  void runAiModelAction(() => deleteAiModel({ modelId }))
                }
                onDownloadModel={(modelId) =>
                  void runAiModelAction(() => downloadAiModel({ modelId }))
                }
                onRefreshInputActivityStatus={() =>
                  void refreshInputActivityStatus()
                }
                onRepairInputActivity={() => void repairInputActivity()}
                onUpdateDraft={updateDraft}
                inputActivityRepairMessage={inputActivityRepairMessage}
                inputActivityRepairing={inputActivityRepairing}
                inputActivityStatus={inputActivityStatus}
                providerStatus={aiProviderStatus}
                transcriptionKeyDraft={transcriptionKeyDraft}
                visionKeyDraft={visionKeyDraft}
              />
            </TabsContent>
          </Tabs>
        </FieldSet>
      )}
    </PageLayout>
  );
}
