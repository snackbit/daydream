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
}

export function SettingsScreenTabs({
  actions,
  busy,
  model,
}: SettingsScreenTabsProps) {
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
        <Tabs className="gap-3" defaultValue="general">
          <TabsList
            aria-label="Settings sections"
            className="h-8 max-w-full justify-start gap-0.5 overflow-x-auto rounded-none border-b bg-transparent p-0"
          >
            <TabsTrigger
              className="data-[state=active]:border-primary h-8 shrink-0 rounded-none border-b-2 border-b-transparent bg-transparent px-2.5 text-xs data-[state=active]:bg-transparent data-[state=active]:shadow-none sm:px-3"
              value="general"
            >
              <Settings2 aria-hidden="true" className="size-3.5" />
              General
            </TabsTrigger>
            <TabsTrigger
              className="data-[state=active]:border-primary h-8 shrink-0 rounded-none border-b-2 border-b-transparent bg-transparent px-2.5 text-xs data-[state=active]:bg-transparent data-[state=active]:shadow-none sm:px-3"
              value="record"
            >
              <Video aria-hidden="true" className="size-3.5" />
              Record
            </TabsTrigger>
            <TabsTrigger
              className="data-[state=active]:border-primary h-8 shrink-0 rounded-none border-b-2 border-b-transparent bg-transparent px-2.5 text-xs data-[state=active]:bg-transparent data-[state=active]:shadow-none sm:px-3"
              value="storage"
            >
              <HardDrive aria-hidden="true" className="size-3.5" />
              Storage/Library
            </TabsTrigger>
            <TabsTrigger
              className="data-[state=active]:border-primary h-8 shrink-0 rounded-none border-b-2 border-b-transparent bg-transparent px-2.5 text-xs data-[state=active]:bg-transparent data-[state=active]:shadow-none sm:px-3"
              value="ai"
            >
              <BrainCircuit aria-hidden="true" className="size-3.5" />
              AI/LLM
            </TabsTrigger>
          </TabsList>

          <TabsContent className="max-w-4xl min-w-0" value="general">
            <GeneralSettingsTab
              draft={model.draft}
              onUpdateDraft={actions.updateDraft}
            />
          </TabsContent>

          <TabsContent className="max-w-4xl min-w-0" value="record">
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

          <TabsContent className="max-w-4xl min-w-0" value="storage">
            <StorageSettingsTab
              draft={model.draft}
              limits={model.limits}
              onUpdateDraft={actions.updateDraft}
            />
          </TabsContent>

          <TabsContent className="max-w-4xl min-w-0" value="ai">
            <AiSettingsTab
              draft={model.draft}
              limits={model.limits}
              models={model.aiModels}
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
