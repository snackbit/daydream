import { useCallback, useState } from 'react';
import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
  AppSettingsLimits,
  InputActivityStatus,
  RecorderScene,
} from '@daydream/shared';
import { BrainCircuit, HardDrive, Settings2, Video } from 'lucide-react';
import { FieldGroup, FieldSet } from '@/components/ui/field';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { AiSettingsTab } from '@/features/settings/components/ai-settings-tab';
import { GeneralSettingsTab } from '@/features/settings/components/general-settings-tab';
import {
  cadenceModeForFps,
  RecordSettingsTab,
  type CadenceMode,
} from '@/features/settings/components/record-settings-tab';
import { StorageSettingsTab } from '@/features/settings/components/storage-settings-tab';
import {
  estimateOneHourRecordingBytes,
  formatSceneDimensions,
  formatStorageSize,
} from '@/features/settings/settings-format';
import { settingsTab, type SettingsTab } from '../settings-location';

export interface SettingsScreenTabsModel {
  aiModels: AiModelAsset[];
  aiProviderStatus: AiProviderStatus | null;
  cadenceMode: CadenceMode | null;
  draft: AppSettings;
  inputActivityRepairMessage: string | null;
  inputActivityRepairing: boolean;
  inputActivityStatus: InputActivityStatus | null;
  limits: AppSettingsLimits;
  savedScene: RecorderScene | null;
  transcriptionKeyDraft: string;
  visionKeyDraft: string;
}

export interface SettingsScreenTabsActions {
  cancelAiModelDownload: (modelId: string) => void;
  changeCadenceMode: (mode: CadenceMode) => void;
  changeTranscriptionKey: (value: string) => void;
  changeVisionKey: (value: string) => void;
  deleteAiModel: (modelId: string) => void;
  downloadAiModel: (modelId: string) => void;
  refreshInputActivityStatus: () => void;
  repairInputActivity: () => void;
  updateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}

export interface SettingsScreenTabsProps {
  actions: SettingsScreenTabsActions;
  busy: boolean;
  model: SettingsScreenTabsModel;
  tab?: SettingsTab | undefined;
  onTabChange?: ((tab: SettingsTab) => void) | undefined;
}

export function SettingsScreenTabs({
  actions,
  busy,
  model,
  tab,
  onTabChange,
}: SettingsScreenTabsProps) {
  const [localTab, setLocalTab] = useState<SettingsTab>('general');
  const handleTabChange = useCallback(
    (value: string) => {
      const next = settingsTab(value);
      setLocalTab(next);
      onTabChange?.(next);
    },
    [onTabChange],
  );
  function handleCancelModelDownload(modelId: string) {
    actions.cancelAiModelDownload(modelId);
  }

  function handleDeleteModel(modelId: string) {
    actions.deleteAiModel(modelId);
  }

  function handleDownloadModel(modelId: string) {
    actions.downloadAiModel(modelId);
  }

  return (
    <FieldSet aria-busy={busy} className="min-w-0" disabled={busy}>
      <FieldGroup className="gap-3">
        <Tabs
          className="gap-3"
          value={tab ?? localTab}
          onValueChange={handleTabChange}
        >
          <TabsList
            variant="underline"
            aria-label="Settings sections"
            className="max-w-full gap-0.5 overflow-x-auto"
          >
            <TabsTrigger
              className="shrink-0 px-2.5 text-xs sm:px-3"
              value="general"
            >
              <Settings2 aria-hidden="true" className="size-3.5" />
              General
            </TabsTrigger>
            <TabsTrigger
              className="shrink-0 px-2.5 text-xs sm:px-3"
              value="record"
            >
              <Video aria-hidden="true" className="size-3.5" />
              Record
            </TabsTrigger>
            <TabsTrigger
              className="shrink-0 px-2.5 text-xs sm:px-3"
              value="storage"
            >
              <HardDrive aria-hidden="true" className="size-3.5" />
              Storage/Library
            </TabsTrigger>
            <TabsTrigger className="shrink-0 px-2.5 text-xs sm:px-3" value="ai">
              <BrainCircuit aria-hidden="true" className="size-3.5" />
              AI/LLM
            </TabsTrigger>
          </TabsList>

          <TabsContent className="min-w-0" value="general">
            <GeneralSettingsTab
              draft={model.draft}
              onUpdateDraft={actions.updateDraft}
            />
          </TabsContent>

          <TabsContent className="min-w-0" value="record">
            <RecordSettingsTab
              cadenceMode={
                model.cadenceMode ?? cadenceModeForFps(model.draft.record.fps)
              }
              draft={model.draft}
              limits={model.limits}
              sceneDimensions={formatSceneDimensions(model.savedScene)}
              oneHourEstimate={formatStorageSize(
                estimateOneHourRecordingBytes(model.draft, model.savedScene),
              )}
              onChangeCadenceMode={actions.changeCadenceMode}
              onUpdateDraft={actions.updateDraft}
            />
          </TabsContent>

          <TabsContent className="min-w-0" value="storage">
            <StorageSettingsTab
              draft={model.draft}
              limits={model.limits}
              onUpdateDraft={actions.updateDraft}
            />
          </TabsContent>

          <TabsContent className="min-w-0" value="ai">
            <AiSettingsTab
              draft={model.draft}
              limits={model.limits}
              models={model.aiModels}
              sourceCount={
                model.savedScene?.independentVideo
                  ? model.savedScene.sources.length
                  : model.savedScene?.sources.length
                    ? 1
                    : 0
              }
              onCancelModelDownload={handleCancelModelDownload}
              onChangeTranscriptionKey={actions.changeTranscriptionKey}
              onChangeVisionKey={actions.changeVisionKey}
              onDeleteModel={handleDeleteModel}
              onDownloadModel={handleDownloadModel}
              onRefreshInputActivityStatus={actions.refreshInputActivityStatus}
              onRepairInputActivity={actions.repairInputActivity}
              onUpdateDraft={actions.updateDraft}
              inputActivityRepairMessage={model.inputActivityRepairMessage}
              inputActivityRepairing={model.inputActivityRepairing}
              inputActivityStatus={model.inputActivityStatus}
              providerStatus={model.aiProviderStatus}
              transcriptionKeyDraft={model.transcriptionKeyDraft}
              visionKeyDraft={model.visionKeyDraft}
            />
          </TabsContent>
        </Tabs>
      </FieldGroup>
    </FieldSet>
  );
}
