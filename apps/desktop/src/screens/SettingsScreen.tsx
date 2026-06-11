import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useBlocker } from '@tanstack/react-router';
import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
  AppSettingsStorageCleanupPreview,
} from '@daydream/shared';
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
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  cancelAiModelDownload,
  deleteAiModel,
  deleteAiProviderSecret,
  downloadAiModel,
  getAiProviderStatus,
  listenToAiModelsChanged,
  listAiModels,
  saveAiProviderSecret,
} from '@/features/ai/ai-client';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import { AiSettingsTab } from '@/features/settings/components/ai-settings-tab';
import { GeneralSettingsTab } from '@/features/settings/components/general-settings-tab';
import {
  cadenceModeForFps,
  RecordSettingsTab,
  type CadenceMode,
} from '@/features/settings/components/record-settings-tab';
import { SettingsLeaveDialog } from '@/features/settings/components/settings-leave-dialog';
import { StorageSettingsTab } from '@/features/settings/components/storage-settings-tab';
import {
  collectSupersededAiSecretRefs,
  settingsForCleanupPreview,
} from '@/features/settings/settings-ai-secret-utils';
import {
  estimateOneHourRecordingBytes,
  formatSceneDimensions,
  formatStorageSize,
} from '@/features/settings/settings-format';
import {
  formatSessionCount,
  screenErrorMessage,
  validateDraftSettings,
} from '@/features/settings/settings-screen-utils';
import { useSettingsStore } from '@/features/settings/settings-store';

type CleanupConfirmationAction = 'reset' | 'save';

interface CleanupConfirmation {
  action: CleanupConfirmationAction;
  proceedAfterSuccess: boolean;
  preview: AppSettingsStorageCleanupPreview;
  settings: AppSettings;
}

interface AiSettingsSavePreparation {
  createdSecretRefs: string[];
  secretRefsToDeleteAfterSave: string[];
  settings: AppSettings;
}

export function SettingsScreen() {
  const draft = useSettingsStore((state) => state.draft);
  const error = useSettingsStore((state) => state.error);
  const isDirty = useSettingsStore((state) => state.isDirty);
  const load = useSettingsStore((state) => state.load);
  const loadStatus = useSettingsStore((state) => state.loadStatus);
  const mutationStatus = useSettingsStore((state) => state.mutationStatus);
  const previewStorageCleanup = useSettingsStore(
    (state) => state.previewStorageCleanup,
  );
  const reset = useSettingsStore((state) => state.reset);
  const save = useSettingsStore((state) => state.save);
  const savedScene = useSettingsStore((state) => state.savedScene);
  const setDraft = useSettingsStore((state) => state.setDraft);
  const settingsState = useSettingsStore((state) => state.settingsState);
  const successMessage = useSettingsStore((state) => state.successMessage);
  const loadNotifications = useNotificationsStore((state) => state.load);
  const [cadenceMode, setCadenceMode] = useState<CadenceMode | null>(null);
  const [cleanupConfirmation, setCleanupConfirmation] =
    useState<CleanupConfirmation | null>(null);
  const [cleanupPreviewing, setCleanupPreviewing] = useState(false);
  const [aiActionError, setAiActionError] = useState<string | null>(null);
  const [aiModels, setAiModels] = useState<AiModelAsset[]>([]);
  const [aiProviderStatus, setAiProviderStatus] =
    useState<AiProviderStatus | null>(null);
  const [resetOpen, setResetOpen] = useState(false);
  const [transcriptionKeyDraft, setTranscriptionKeyDraft] = useState('');
  const [visionKeyDraft, setVisionKeyDraft] = useState('');
  const pendingSecretRefsForDeletion = useRef<string[]>([]);
  const selectedAiModelId = draft?.ai.transcription.localModelId ?? null;

  useEffect(() => {
    if (loadStatus === 'idle') {
      void load();
    }
  }, [load, loadStatus]);

  const validationError = useMemo(
    () =>
      draft && settingsState
        ? validateDraftSettings(draft, settingsState.limits)
        : null,
    [draft, settingsState],
  );
  const saving = mutationStatus === 'loading';
  const busy = saving || cleanupPreviewing;
  const hasAiSecretDraft = Boolean(
    transcriptionKeyDraft.trim() || visionKeyDraft.trim(),
  );
  const hasUnsavedSettings = isDirty || hasAiSecretDraft;
  const canSave = Boolean(
    draft && hasUnsavedSettings && !validationError && !busy,
  );

  const blocker = useBlocker({
    enableBeforeUnload: hasUnsavedSettings,
    shouldBlockFn: ({ current, next }) =>
      hasUnsavedSettings &&
      current.pathname === '/settings' &&
      next.pathname !== '/settings',
    withResolver: true,
  });

  const refreshAiScaffolds = useCallback(async () => {
    try {
      const [modelList, providerStatus] = await Promise.all([
        listAiModels(),
        getAiProviderStatus(),
      ]);
      setAiActionError(null);
      setAiModels(modelList.models);
      setAiProviderStatus(providerStatus);
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

  function updateDraft(updater: (draft: AppSettings) => AppSettings) {
    if (!draft) {
      return;
    }

    setDraft(updater(draft));
  }

  async function requestSave() {
    if (!draft) {
      return;
    }

    await runWithCleanupConfirmation('save', settingsForCleanupPreview(draft), {
      proceedAfterSuccess: false,
    });
  }

  async function requestReset() {
    if (!settingsState) {
      return;
    }

    pendingSecretRefsForDeletion.current = collectSupersededAiSecretRefs(
      settingsState.settings,
      settingsState.defaults,
    );
    await runWithCleanupConfirmation('reset', settingsState.defaults, {
      proceedAfterSuccess: false,
    });
  }

  async function runWithCleanupConfirmation(
    action: CleanupConfirmationAction,
    settings: AppSettings,
    { proceedAfterSuccess }: { proceedAfterSuccess: boolean },
  ): Promise<boolean> {
    setCleanupPreviewing(true);
    try {
      const preview = await previewStorageCleanup(settings);
      if (!preview) {
        return false;
      }
      if (preview.recordingCount > 0) {
        if (action === 'reset') {
          setResetOpen(false);
        }
        setCleanupConfirmation({
          action,
          preview,
          proceedAfterSuccess,
          settings,
        });
        return false;
      }

      if (action === 'save') {
        return await savePreparedSettings(settings);
      }

      return await confirmReset();
    } finally {
      setCleanupPreviewing(false);
    }
  }

  async function confirmReset(): Promise<boolean> {
    await reset();
    await deletePendingAiSecrets();
    await refreshAiScaffolds();
    setCadenceMode(null);
    setResetOpen(false);
    return true;
  }

  async function confirmCleanup() {
    const confirmation = cleanupConfirmation;
    if (!confirmation) {
      return;
    }

    setCleanupConfirmation(null);
    if (confirmation.action === 'save') {
      const saved = await savePreparedSettings(confirmation.settings);
      if (saved && confirmation.proceedAfterSuccess) {
        blocker.proceed?.();
      }
    } else {
      await confirmReset();
    }
  }

  async function savePreparedSettings(settings: AppSettings): Promise<boolean> {
    const preparation = await prepareAiSettingsForSave(settings);
    if (!preparation) {
      return false;
    }

    try {
      await save(preparation.settings);
    } catch {
      await deleteAiSecretsAfterFailure(preparation.createdSecretRefs);
      return false;
    }

    pendingSecretRefsForDeletion.current =
      preparation.secretRefsToDeleteAfterSave;
    await deletePendingAiSecrets();
    setTranscriptionKeyDraft('');
    setVisionKeyDraft('');
    await refreshAiScaffolds();
    return true;
  }

  async function deletePendingAiSecrets() {
    const secretRefs = pendingSecretRefsForDeletion.current;
    pendingSecretRefsForDeletion.current = [];

    await deleteAiSecrets(secretRefs);
  }

  async function prepareAiSettingsForSave(
    settings: AppSettings,
  ): Promise<AiSettingsSavePreparation | null> {
    let nextSettings = settings;
    const createdSecretRefs: string[] = [];
    setAiActionError(null);
    try {
      if (settings.ai.transcription.provider !== 'openai-compatible') {
        nextSettings = {
          ...nextSettings,
          ai: {
            ...nextSettings.ai,
            transcription: {
              ...nextSettings.ai.transcription,
              remoteApiKeyRef: null,
            },
          },
        };
      } else if (transcriptionKeyDraft.trim()) {
        const secret = await saveAiProviderSecret({
          plaintext: transcriptionKeyDraft,
          role: 'transcription',
        });
        createdSecretRefs.push(secret.secretRef);
        nextSettings = {
          ...nextSettings,
          ai: {
            ...nextSettings.ai,
            transcription: {
              ...nextSettings.ai.transcription,
              remoteApiKeyRef: secret.secretRef,
            },
          },
        };
      }

      if (settings.ai.vision.provider !== 'openai-compatible') {
        nextSettings = {
          ...nextSettings,
          ai: {
            ...nextSettings.ai,
            vision: {
              ...nextSettings.ai.vision,
              apiKeyRef: null,
            },
          },
        };
      } else if (visionKeyDraft.trim()) {
        const secret = await saveAiProviderSecret({
          plaintext: visionKeyDraft,
          role: 'vision',
        });
        createdSecretRefs.push(secret.secretRef);
        nextSettings = {
          ...nextSettings,
          ai: {
            ...nextSettings.ai,
            vision: {
              ...nextSettings.ai.vision,
              apiKeyRef: secret.secretRef,
            },
          },
        };
      }

      return {
        createdSecretRefs,
        secretRefsToDeleteAfterSave: collectSupersededAiSecretRefs(
          settingsState?.settings ?? null,
          nextSettings,
        ),
        settings: nextSettings,
      };
    } catch (error) {
      await deleteAiSecretsAfterFailure(createdSecretRefs);
      setAiActionError(screenErrorMessage(error));
      return null;
    }
  }

  async function deleteAiSecretsAfterFailure(secretRefs: string[]) {
    try {
      await deleteAiSecrets(secretRefs);
    } catch (error) {
      setAiActionError(screenErrorMessage(error));
    }
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

  async function saveAndLeaveSettings() {
    if (!draft) {
      return;
    }

    const saved = await runWithCleanupConfirmation(
      'save',
      settingsForCleanupPreview(draft),
      { proceedAfterSuccess: true },
    );
    if (saved) {
      blocker.proceed?.();
    }
  }

  function discardAndLeaveSettings() {
    if (settingsState) {
      setDraft(settingsState.settings);
    }
    setTranscriptionKeyDraft('');
    setVisionKeyDraft('');
    pendingSecretRefsForDeletion.current = [];
    blocker.proceed?.();
  }

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
          if (!open && !saving) {
            setCleanupConfirmation(null);
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
              onClick={() => setCleanupConfirmation(null)}
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
                void runAiModelAction(() => cancelAiModelDownload({ modelId }))
              }
              onChangeTranscriptionKey={setTranscriptionKeyDraft}
              onChangeVisionKey={setVisionKeyDraft}
              onDeleteModel={(modelId) =>
                void runAiModelAction(() => deleteAiModel({ modelId }))
              }
              onDownloadModel={(modelId) =>
                void runAiModelAction(() => downloadAiModel({ modelId }))
              }
              onUpdateDraft={updateDraft}
              providerStatus={aiProviderStatus}
              transcriptionKeyDraft={transcriptionKeyDraft}
              visionKeyDraft={visionKeyDraft}
            />
          </TabsContent>
        </Tabs>
      )}
    </PageLayout>
  );
}

async function deleteAiSecrets(secretRefs: string[]) {
  await Promise.all(
    secretRefs.map((secretRef) => deleteAiProviderSecret({ secretRef })),
  );
}
