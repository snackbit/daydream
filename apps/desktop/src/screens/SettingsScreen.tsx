import { useState } from 'react';
import { LoaderCircle, Check } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { SettingsDangerZone } from '@/features/settings/components/settings-danger-zone';
import { SettingsScreenDialogs } from '@/features/settings/screen/settings-screen-dialogs';
import type {
  SettingsScreenTabsModel,
  SettingsScreenTabsActions,
} from '@/features/settings/screen/settings-screen-tabs';
import {
  parseSettingsLocation,
  type SettingsTab,
  type SettingsFocus,
  type SettingsSection,
  type SettingsDetail,
} from '@/features/settings/settings-location';
import { useSettingsStore } from '@/features/settings/settings-store';
import { settingsAiModelActions } from '@/features/settings/use-settings-ai-controller';
import { useSettingsScreenController } from '@/features/settings/use-settings-screen-controller';
import { ModelsAccounts } from '@/features/settings/workspace/accounts/models-accounts';
import { FeatureEditor } from '@/features/settings/workspace/feature-editor';
import { FeatureOverview } from '@/features/settings/workspace/features/feature-overview';
import { GeneralPreferences } from '@/features/settings/workspace/pages/general-preferences';
import { PrivacyPreferences } from '@/features/settings/workspace/pages/privacy-preferences';
import { RecordingPreferences } from '@/features/settings/workspace/pages/recording-preferences';
import { StoragePreferences } from '@/features/settings/workspace/pages/storage-preferences';
import {
  SettingsNavigation,
  settingsSections,
} from '@/features/settings/workspace/settings-navigation';

const descriptions = {
  general: 'Appearance and app preferences.',
  recording: 'Capture cadence for new recordings.',
  privacy: 'Choose what Daydream can capture.',
  ai: 'Choose how your recordings become searchable.',
  models: 'Your accounts, APIs and local models.',
  storage: 'Manage recording retention and library size.',
} satisfies Record<SettingsSection, string>;
export interface SettingsScreenProps {
  tab?: SettingsTab | undefined;
  focusSection?: SettingsFocus | null | undefined;
  onTabChange?: ((tab: SettingsTab) => void) | undefined;
  section?: SettingsSection | undefined;
  detail?: SettingsDetail | null | undefined;
  onLocationChange?:
    | ((section: SettingsSection, detail: SettingsDetail | null) => void)
    | undefined;
}
export function SettingsScreen({
  tab,
  focusSection,
  onTabChange,
  section,
  detail,
  onLocationChange,
}: SettingsScreenProps = {}) {
  const controller = useSettingsScreenController();
  const legacy = parseSettingsLocation({ tab, focus: focusSection });
  const [localSection, setLocalSection] = useState(legacy.section);
  const [localDetail, setLocalDetail] = useState(legacy.detail);
  const active = section ?? (onTabChange ? legacy.section : localSection);
  const selectedDetail = detail === undefined ? localDetail : detail;
  const { draft, settingsState } = controller;
  const loaded = draft !== null && settingsState !== null;
  function navigate(
    next: SettingsSection,
    nextDetail: SettingsDetail | null = null,
  ) {
    setLocalSection(next);
    setLocalDetail(nextDetail);
    onLocationChange?.(next, nextDetail);
    if (next !== active)
      onTabChange?.(
        next === 'recording' || next === 'privacy'
          ? 'record'
          : next === 'models'
            ? 'ai'
            : next,
      );
  }
  function changeSection(next: SettingsSection) {
    navigate(next);
  }
  function openDetail(next: SettingsDetail) {
    navigate(active, next);
  }
  function closeDetail() {
    navigate(active);
  }
  function openProtection() {
    openDetail('protection');
  }
  function cancelDownload(modelId: string) {
    void controller.runAiModelAction(
      () => settingsAiModelActions.cancelAiModelDownload({ modelId }),
      modelId,
    );
  }
  function deleteModel(modelId: string) {
    void controller.runAiModelAction(
      () => settingsAiModelActions.deleteAiModel({ modelId }),
      modelId,
    );
  }
  function downloadModel(modelId: string) {
    void controller.runAiModelAction(
      () => settingsAiModelActions.downloadAiModel({ modelId }),
      modelId,
    );
  }
  function retrySave() {
    void controller.requestSave();
  }
  function revert() {
    useSettingsStore.getState().revertAutosave();
  }
  function resetBlocker() {
    controller.blocker.reset?.();
  }
  const model: SettingsScreenTabsModel | null =
    draft && settingsState
      ? {
          modelKeyDrafts: controller.modelKeyDrafts,
          aiModels: controller.aiModels,
          modelActionErrors: controller.modelActionErrors,
          aiProviderStatus: controller.aiProviderStatus,
          cadenceMode: controller.cadenceMode,
          draft,
          inputActivityRepairMessage: controller.inputActivityRepairMessage,
          inputActivityRepairing: controller.inputActivityRepairing,
          inputActivityStatus: controller.inputActivityStatus,
          limits: settingsState.limits,
          savedScene: controller.savedScene,
          transcriptionKeyDraft: controller.transcriptionKeyDraft,
          visionKeyDraft: controller.visionKeyDraft,
          recallEmbeddingKeyDraft: controller.recallEmbeddingKeyDraft,
          recallGenerationKeyDraft: controller.recallGenerationKeyDraft,
        }
      : null;
  const actions: SettingsScreenTabsActions = {
    changeModelKey: controller.changeModelKey,
    cancelAiModelDownload: cancelDownload,
    changeCadenceMode: controller.setCadenceMode,
    changeTranscriptionKey: controller.setTranscriptionKeyDraft,
    changeVisionKey: controller.setVisionKeyDraft,
    changeRecallEmbeddingKey: controller.setRecallEmbeddingKeyDraft,
    changeRecallGenerationKey: controller.setRecallGenerationKeyDraft,
    deleteAiModel: deleteModel,
    downloadAiModel: downloadModel,
    transferAiModel: controller.runModelTransferAction,
    refreshInputActivityStatus: controller.refreshInputActivityStatus,
    repairInputActivity: controller.repairInputActivity,
    updateDraft: controller.updateDraft,
  };
  return (
    <PageLayout className="max-w-[1120px] gap-7">
      <PageHeader
        title="Settings"
        actions={
          <span
            role="status"
            aria-live="polite"
            className="text-muted-foreground flex items-center gap-1.5 pt-2 text-xs"
          >
            {controller.saving ? (
              <>
                <LoaderCircle
                  aria-hidden="true"
                  className="size-3 animate-spin"
                />
                Saving…
              </>
            ) : controller.error ? (
              'Changes need attention'
            ) : loaded ? (
              <>
                <Check aria-hidden="true" className="size-3" />
                Saved
              </>
            ) : (
              'Loading…'
            )}
          </span>
        }
      />
      <SettingsDangerZone
        scope="recovery"
        disabled={controller.busy || !loaded}
      />
      <div className="grid min-w-0 gap-7 lg:grid-cols-[176px_minmax(0,1fr)] lg:gap-10">
        <SettingsNavigation section={active} onChange={changeSection} />
        <div className="max-w-[800px] min-w-0">
          <div className="mb-7">
            <h2 className="text-xl font-medium tracking-tight">
              {settingsSections.find((item) => item.id === active)?.label}
            </h2>
            <p className="text-muted-foreground mt-2 text-sm">
              {descriptions[active]}
            </p>
          </div>
          {controller.error ? (
            <Alert variant="destructive" className="mb-6">
              <AlertTitle>Could not save changes</AlertTitle>
              <AlertDescription>
                {controller.error}
                <div className="mt-3 flex gap-2">
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={controller.busy}
                    onClick={retrySave}
                  >
                    Retry
                  </Button>
                  <Button
                    variant="ghost"
                    size="sm"
                    disabled={controller.busy}
                    onClick={revert}
                  >
                    Revert changes
                  </Button>
                </div>
              </AlertDescription>
            </Alert>
          ) : null}
          {controller.aiActionError ? (
            <Alert variant="destructive" className="mb-6">
              <AlertTitle>AI setup needs attention</AlertTitle>
              <AlertDescription>{controller.aiActionError}</AlertDescription>
            </Alert>
          ) : null}
          {!model ? (
            <div role="status" className="text-muted-foreground py-8 text-sm">
              {controller.loadStatus === 'error' ? (
                <>
                  <p>Settings are unavailable.</p>
                  <Button
                    className="mt-3"
                    variant="outline"
                    onClick={controller.retryLoadSettings}
                  >
                    Try again
                  </Button>
                </>
              ) : (
                'Loading settings…'
              )}
            </div>
          ) : active === 'general' ? (
            <>
              <GeneralPreferences
                draft={model.draft}
                onUpdate={controller.updateDraft}
              />
              <SettingsDangerZone scope="general" disabled={controller.busy} />
            </>
          ) : active === 'recording' ? (
            <RecordingPreferences
              draft={model.draft}
              limits={model.limits}
              scene={model.savedScene}
              onUpdate={controller.updateDraft}
            />
          ) : active === 'privacy' ? (
            <PrivacyPreferences
              model={model}
              actions={actions}
              onOpenProtection={openProtection}
            />
          ) : active === 'ai' ? (
            <FeatureOverview
              draft={model.draft}
              models={model.aiModels}
              onOpen={openDetail}
              onUpdate={controller.updateDraft}
            />
          ) : active === 'models' ? (
            <ModelsAccounts model={model} actions={actions} />
          ) : (
            <>
              <StoragePreferences
                draft={model.draft}
                limits={model.limits}
                onUpdate={controller.updateStorage}
              />
              <SettingsDangerZone scope="storage" disabled={controller.busy} />
            </>
          )}
        </div>
      </div>
      {model && selectedDetail ? (
        <FeatureEditor
          key={`${selectedDetail}-${controller.preferenceRevision}`}
          detail={selectedDetail}
          model={model}
          actions={actions}
          onClose={closeDetail}
          onDiscard={() => {
            controller.clearSecretDrafts();
            if (!useSettingsStore.getState().isDirty)
              useSettingsStore.getState().revertAutosave();
          }}
          onSave={controller.savePreparedSettings}
        />
      ) : null}
      <SettingsScreenDialogs
        actions={{
          confirmCleanup: controller.confirmCleanup,
          discardAndLeaveSettings: controller.discardAndLeaveSettings,
          dismissCleanupConfirmation: controller.dismissCleanupConfirmation,
          resetLeaveBlocker: resetBlocker,
          saveAndLeaveSettings: controller.saveAndLeaveSettings,
        }}
        model={{
          blockerStatus: controller.blocker.status,
          busy: controller.busy,
          canSave: controller.canSave,
          cleanupConfirmation: controller.cleanupConfirmation,
          saving: controller.saving,
          validationError: controller.validationError,
        }}
      />
    </PageLayout>
  );
}
