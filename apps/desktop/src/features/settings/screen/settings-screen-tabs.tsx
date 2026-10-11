import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
  AppSettingsLimits,
  InputActivityStatus,
  RecorderScene,
} from '@daydream/shared';
import type { ModelTransferRowProps } from '@/features/ai/models/model-transfer-row';
import type { CadenceMode } from '../components/record-settings-tab';

export interface SettingsScreenTabsModel {
  modelKeyDrafts?: Readonly<Record<string, string>>;
  aiModels: AiModelAsset[];
  modelActionErrors?: Readonly<Record<string, string>> | undefined;
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
  recallEmbeddingKeyDraft: string;
  recallGenerationKeyDraft: string;
}

export interface SettingsScreenTabsActions {
  changeModelKey?: (id: string, value: string) => void;
  transferAiModel?: ModelTransferRowProps['onTransferAction'];
  cancelAiModelDownload: (modelId: string) => void;
  changeCadenceMode: (mode: CadenceMode) => void;
  changeTranscriptionKey: (value: string) => void;
  changeVisionKey: (value: string) => void;
  changeRecallEmbeddingKey: (value: string) => void;
  changeRecallGenerationKey: (value: string) => void;
  deleteAiModel: (modelId: string) => void;
  downloadAiModel: (modelId: string) => void;
  refreshInputActivityStatus: () => void;
  repairInputActivity: () => void;
  updateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
}
