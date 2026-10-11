import type {
  AiModelAsset,
  AiProviderStatus,
  AppSettings,
} from '@daydream/shared';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { FieldGroup } from '@/components/ui/field';
import { Separator } from '@/components/ui/separator';
import type { UpdateSettingsDraft } from '@/features/settings/settings-draft-updaters';
import { VisionModelSettings } from '../models/vision-model-settings';
import { OcrEngineSettingsSection } from './ocr/ocr-engine-settings-section';
import { VisionEndpointSettingsSection } from './vision/vision-endpoint-settings-section';
import {
  joinRequirements,
  missingVisionRequirements,
} from './vision/vision-settings-format';

interface AiVisionCardProps {
  draft: AppSettings;
  models: AiModelAsset[];
  sourceCount: number;
  onCancelModelDownload: (modelId: string) => void;
  onChangeVisionKey: (value: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  onUpdateDraft: UpdateSettingsDraft;
  providerStatus: AiProviderStatus | null;
  visionKeyDraft: string;
}

export function AiVisionCard({
  draft,
  models,
  sourceCount,
  onCancelModelDownload,
  onChangeVisionKey,
  onDeleteModel,
  onDownloadModel,
  onUpdateDraft,
  providerStatus,
  visionKeyDraft,
}: AiVisionCardProps) {
  const { ocr, processing, vision } = draft.ai;
  const effectiveFrameRecognitionFps =
    processing.frameRecognitionSyncWithRecording
      ? draft.record.fps
      : processing.frameRecognitionFps;
  const missingRequirements = missingVisionRequirements({
    vision,
    visionKeyDraft,
  });
  const hasVisionConsumer =
    (ocr.enabled && ocr.engine === 'vision') || vision.observationsEnabled;

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>OCR and Vision</CardTitle>
        <CardDescription>
          Read text and describe screen activity.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup className="gap-3">
          <OcrEngineSettingsSection
            model={{
              enabled: ocr.enabled,
              engine: ocr.engine,
              effectiveFrameRecognitionFps,
              observationsEnabled: vision.observationsEnabled,
              status: providerStatus?.ocr,
              tesseractAssets: {
                runtime: models.find(
                  (model) => model.id === 'tesseract-runtime',
                ),
                englishData: models.find(
                  (model) => model.id === 'tessdata-eng',
                ),
                germanData: models.find((model) => model.id === 'tessdata-deu'),
              },
            }}
            actions={{
              onCancelModelDownload,
              onDeleteModel,
              onDownloadModel,
              onUpdateDraft,
            }}
          />
          {!draft.ai.modelRegistry &&
          hasVisionConsumer &&
          missingRequirements.length > 0 ? (
            <VisionConfigurationAlert
              missingRequirements={missingRequirements}
            />
          ) : null}
          <Separator />
          {draft.ai.modelRegistry ? (
            <VisionModelSettings draft={draft} onUpdateDraft={onUpdateDraft} />
          ) : (
            <VisionEndpointSettingsSection
              model={{
                ocrEnabled: ocr.enabled,
                ocrEngine: ocr.engine,
                settings: vision,
                visionKeyDraft,
                sourceCount,
                samplesPerMinute: effectiveFrameRecognitionFps * 60,
              }}
              actions={{
                onChangeVisionKey,
                onUpdateDraft,
              }}
            />
          )}
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

function VisionConfigurationAlert({
  missingRequirements,
}: {
  missingRequirements: string[];
}) {
  const message = `Add ${joinRequirements(missingRequirements)}. You can save this setup now. Vision tasks cannot run until it is complete.`;

  return (
    <Alert variant="destructive">
      <AlertTitle>Vision processing is not ready</AlertTitle>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}
