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
import { OcrEngineSettingsSection } from './ocr/ocr-engine-settings-section';
import { VisionEndpointSettingsSection } from './vision/vision-endpoint-settings-section';
import {
  joinRequirements,
  missingVisionRequirements,
} from './vision/vision-settings-format';

interface AiVisionCardProps {
  draft: AppSettings;
  models: AiModelAsset[];
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
    ocr.engine === 'vision' || vision.observationsEnabled;

  return (
    <Card size="sm">
      <CardHeader>
        <CardTitle>OCR and Vision</CardTitle>
        <CardDescription>
          Choose how text is recognized. Screen observations use the shared
          Vision endpoint independently.
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
          {hasVisionConsumer && missingRequirements.length > 0 ? (
            <VisionConfigurationAlert
              engine={ocr.engine}
              missingRequirements={missingRequirements}
              observationsEnabled={vision.observationsEnabled}
            />
          ) : null}
          <Separator />
          <VisionEndpointSettingsSection
            model={{
              ocrEnabled: ocr.enabled,
              ocrEngine: ocr.engine,
              settings: vision,
              visionKeyDraft,
            }}
            actions={{
              onChangeVisionKey,
              onUpdateDraft,
            }}
          />
        </FieldGroup>
      </CardContent>
    </Card>
  );
}

function VisionConfigurationAlert({
  engine,
  missingRequirements,
  observationsEnabled,
}: {
  engine: AppSettings['ai']['ocr']['engine'];
  missingRequirements: string[];
  observationsEnabled: boolean;
}) {
  const message =
    engine === 'vision'
      ? `Vision OCR is not ready until you configure ${joinRequirements(missingRequirements)}. It will not fall back to Tesseract.${observationsEnabled ? ' Screen observations are also unavailable until configured.' : ''}`
      : `Screen observations are not ready until you configure ${joinRequirements(missingRequirements)}.`;

  return (
    <Alert variant="destructive">
      <AlertTitle>Vision processing is not ready</AlertTitle>
      <AlertDescription>{message}</AlertDescription>
    </Alert>
  );
}
