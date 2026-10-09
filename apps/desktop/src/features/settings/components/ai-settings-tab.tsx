import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
  AppSettingsLimits,
  InputActivityStatus,
} from '@daydream/shared';
import type { ModelTransferRowProps } from '@/features/ai/models/model-transfer-row';
import { AiActivityProcessingCard } from './ai-activity-processing-card';
import { AiModelCacheCard } from './ai-model-cache-card';
import { AiTranscriptionCard } from './ai-transcription-card';
import { AiVisionCard } from './ai-vision-card';

interface AiSettingsTabProps {
  draft: AppSettings;
  limits: AppSettingsLimits;
  models: AiModelAsset[];
  modelActionErrors?: Readonly<Record<string, string>> | undefined;
  onTransferAction?: ModelTransferRowProps['onTransferAction'];
  sourceCount: number;
  onCancelModelDownload: (modelId: string) => void;
  onChangeTranscriptionKey: (value: string) => void;
  onChangeVisionKey: (value: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  onRefreshInputActivityStatus: () => void;
  onRepairInputActivity: () => void;
  onUpdateDraft: (updater: (draft: AppSettings) => AppSettings) => void;
  inputActivityRepairMessage: string | null;
  inputActivityRepairing: boolean;
  inputActivityStatus: InputActivityStatus | null;
  providerStatus: AiProviderStatus | null;
  transcriptionKeyDraft: string;
  visionKeyDraft: string;
}

export function AiSettingsTab({
  draft,
  limits,
  models,
  modelActionErrors,
  onTransferAction,
  sourceCount,
  onCancelModelDownload,
  onChangeTranscriptionKey,
  onChangeVisionKey,
  onDeleteModel,
  onDownloadModel,
  onRefreshInputActivityStatus,
  onRepairInputActivity,
  onUpdateDraft,
  inputActivityRepairMessage,
  inputActivityRepairing,
  inputActivityStatus,
  providerStatus,
  transcriptionKeyDraft,
  visionKeyDraft,
}: AiSettingsTabProps) {
  const whisperModels = models.filter(
    (model) => model.provider === 'local-whisper-cpp',
  );

  return (
    <div className="grid gap-3">
      <section id="ai-transcription" tabIndex={-1}>
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
      </section>

      <section id="ai-models" tabIndex={-1}>
        <AiModelCacheCard
          modelActionErrors={modelActionErrors}
          onTransferAction={onTransferAction}
          models={whisperModels}
          onCancelModelDownload={onCancelModelDownload}
          onDeleteModel={onDeleteModel}
          onDownloadModel={onDownloadModel}
        />
      </section>

      <section id="ai-vision" tabIndex={-1}>
        <div id="ai-ocr" tabIndex={-1}>
          <AiVisionCard
            draft={draft}
            models={models}
            sourceCount={sourceCount}
            onCancelModelDownload={onCancelModelDownload}
            onChangeVisionKey={onChangeVisionKey}
            onDeleteModel={onDeleteModel}
            onDownloadModel={onDownloadModel}
            onUpdateDraft={onUpdateDraft}
            providerStatus={providerStatus}
            visionKeyDraft={visionKeyDraft}
          />
        </div>
      </section>

      <section id="ai-processing" tabIndex={-1}>
        <AiActivityProcessingCard
          draft={draft}
          inputActivityRepairMessage={inputActivityRepairMessage}
          inputActivityRepairing={inputActivityRepairing}
          inputActivityStatus={inputActivityStatus}
          limits={limits}
          onRefreshInputActivityStatus={onRefreshInputActivityStatus}
          onRepairInputActivity={onRepairInputActivity}
          onUpdateDraft={onUpdateDraft}
        />
      </section>
    </div>
  );
}
