import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
  AppSettingsLimits,
} from '@daydream/shared';
import { AiActivityProcessingCard } from './ai-activity-processing-card';
import { AiModelCacheCard } from './ai-model-cache-card';
import { AiTranscriptionCard } from './ai-transcription-card';
import { AiVisionCard } from './ai-vision-card';

interface AiSettingsTabProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  models: AiModelAsset[];
  onCancelModelDownload: (modelId: string) => void;
  onChangeTranscriptionKey: (value: string) => void;
  onChangeVisionKey: (value: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
  providerStatus: AiProviderStatus | null;
  transcriptionKeyDraft: string;
  visionKeyDraft: string;
}

export function AiSettingsTab({
  draft,
  limits,
  models,
  onCancelModelDownload,
  onChangeTranscriptionKey,
  onChangeVisionKey,
  onDeleteModel,
  onDownloadModel,
  onUpdateDraft,
  providerStatus,
  transcriptionKeyDraft,
  visionKeyDraft,
}: AiSettingsTabProps) {
  const whisperModels = models.filter(
    (model) => model.provider === 'local-whisper-cpp',
  );

  return (
    <div className="grid gap-4">
      <AiTranscriptionCard
        draft={draft}
        limits={limits}
        models={whisperModels}
        onCancelModelDownload={onCancelModelDownload}
        onChangeTranscriptionKey={onChangeTranscriptionKey}
        onDeleteModel={onDeleteModel}
        onDownloadModel={onDownloadModel}
        onUpdateDraft={onUpdateDraft}
        providerStatus={providerStatus}
        transcriptionKeyDraft={transcriptionKeyDraft}
      />

      <AiModelCacheCard
        models={whisperModels}
        onCancelModelDownload={onCancelModelDownload}
        onDeleteModel={onDeleteModel}
        onDownloadModel={onDownloadModel}
      />

      <AiVisionCard
        draft={draft}
        models={models}
        onCancelModelDownload={onCancelModelDownload}
        onChangeVisionKey={onChangeVisionKey}
        onDeleteModel={onDeleteModel}
        onDownloadModel={onDownloadModel}
        onUpdateDraft={onUpdateDraft}
        providerStatus={providerStatus}
        visionKeyDraft={visionKeyDraft}
      />

      <AiActivityProcessingCard
        draft={draft}
        limits={limits}
        onUpdateDraft={onUpdateDraft}
      />
    </div>
  );
}
