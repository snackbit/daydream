import { useCallback } from 'react';
import type {
  AiModelAsset,
  AiOcrEngine,
  AiProviderStatus,
} from '@daydream/shared';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import {
  Field,
  FieldDescription,
  FieldGroup,
  FieldLabel,
} from '@/components/ui/field';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  type UpdateSettingsDraft,
  updateAiOcr,
} from '@/features/settings/settings-draft-updaters';
import { formatFps } from '@/features/settings/settings-format';
import { AiModelActionButton, ModelStatusIcon } from '../ai-model-cache-card';
import { CheckboxField } from '../ai-settings-fields';

export interface OcrEngineSettingsSectionModel {
  enabled: boolean;
  engine: AiOcrEngine;
  effectiveFrameRecognitionFps: number;
  observationsEnabled: boolean;
  status: AiProviderStatus['ocr'] | undefined;
  tesseractAssets: {
    runtime: AiModelAsset | undefined;
    englishData: AiModelAsset | undefined;
    germanData: AiModelAsset | undefined;
  };
}

export interface OcrEngineSettingsSectionActions {
  onCancelModelDownload: (modelId: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  onUpdateDraft: UpdateSettingsDraft;
}

export interface OcrEngineSettingsSectionProps {
  actions: OcrEngineSettingsSectionActions;
  model: OcrEngineSettingsSectionModel;
}

export function OcrEngineSettingsSection({
  actions,
  model,
}: OcrEngineSettingsSectionProps) {
  const requestCeilingPerMinute = 60 * model.effectiveFrameRecognitionFps;
  const handleOcrEnabledChange = useCallback(
    (enabled: boolean) => {
      updateAiOcr(actions.onUpdateDraft, (current) => ({
        ...current,
        enabled,
      }));
    },
    [actions.onUpdateDraft],
  );
  const handleOcrEngineChange = useCallback(
    (value: string) => {
      const engine = parseOcrEngine(value);
      if (!engine) {
        return;
      }
      updateAiOcr(actions.onUpdateDraft, (current) => ({ ...current, engine }));
    },
    [actions.onUpdateDraft],
  );

  return (
    <div className="grid gap-3">
      <FieldGroup>
        <CheckboxField
          checked={model.enabled}
          description="Recognize retained frames selected by the existing sampling policy."
          label="Run OCR on retained frames"
          onCheckedChange={handleOcrEnabledChange}
        />
        <Field>
          <FieldLabel htmlFor="ai-ocr-engine">
            Text recognition engine
          </FieldLabel>
          <Select onValueChange={handleOcrEngineChange} value={model.engine}>
            <SelectTrigger id="ai-ocr-engine">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="tesseract">Tesseract (local)</SelectItem>
              <SelectItem value="vision">Vision model</SelectItem>
            </SelectContent>
          </Select>
          <FieldDescription>
            New jobs use this setting. Retry keeps a job&apos;s original engine;
            Reprocess OCR uses the current setting.
          </FieldDescription>
        </Field>
      </FieldGroup>
      {model.engine === 'tesseract' ? (
        <TesseractAssets actions={actions} model={model} />
      ) : (
        <VisionOcrDisclosure
          enabled={model.enabled}
          observationsEnabled={model.observationsEnabled}
          recognitionFps={model.effectiveFrameRecognitionFps}
          requestCeilingPerMinute={requestCeilingPerMinute}
        />
      )}
    </div>
  );
}

function TesseractAssets({ actions, model }: OcrEngineSettingsSectionProps) {
  const status = model.status;
  const systemRuntimeAvailable = status?.runtimeSource === 'system';

  return (
    <div className="grid gap-2">
      <p className="text-muted-foreground text-xs">
        {formatEffectiveOcrStatus(status)}
      </p>
      <OcrAssetRow
        availableOverride={systemRuntimeAvailable}
        model={model.tesseractAssets.runtime}
        name="Runtime"
        onCancelModelDownload={actions.onCancelModelDownload}
        onDeleteModel={actions.onDeleteModel}
        onDownloadModel={actions.onDownloadModel}
        statusTextOverride={
          systemRuntimeAvailable && status
            ? systemRuntimeStatusText(status)
            : undefined
        }
      />
      <OcrAssetRow
        model={model.tesseractAssets.englishData}
        name="Best English OCR data"
        onCancelModelDownload={actions.onCancelModelDownload}
        onDeleteModel={actions.onDeleteModel}
        onDownloadModel={actions.onDownloadModel}
      />
      <OcrAssetRow
        model={model.tesseractAssets.germanData}
        name="Best German OCR data"
        onCancelModelDownload={actions.onCancelModelDownload}
        onDeleteModel={actions.onDeleteModel}
        onDownloadModel={actions.onDownloadModel}
      />
    </div>
  );
}

function VisionOcrDisclosure({
  enabled,
  observationsEnabled,
  recognitionFps,
  requestCeilingPerMinute,
}: {
  enabled: boolean;
  observationsEnabled: boolean;
  recognitionFps: number;
  requestCeilingPerMinute: number;
}) {
  const fpsLabel = recognitionFps.toLocaleString(undefined, {
    maximumFractionDigits: 2,
  });
  const requestCeilingLabel = requestCeilingPerMinute.toLocaleString(
    undefined,
    { maximumFractionDigits: 2 },
  );

  return (
    <Alert>
      <AlertTitle>Vision OCR uses the shared remote endpoint</AlertTitle>
      <AlertDescription>
        <div className="grid gap-1">
          <p>
            {enabled
              ? 'Original-size retained sampled frames will be sent for OCR when you process a recording.'
              : 'OCR is off; no frames will be sent for OCR until you enable it.'}
          </p>
          <p>
            Current cadence: {formatFps(recognitionFps)}. OCR request ceiling:{' '}
            {requestCeilingLabel} sampled frames per minute (60 × {fpsLabel}{' '}
            FPS), before exact-image cache reuse.
          </p>
          <p>
            Remote image calls may incur provider charges. Pricing varies; this
            screen does not estimate charges.{' '}
            {observationsEnabled
              ? 'Enabled screen observations can add separate requests.'
              : 'Screen observations use a separate control and can add requests if enabled.'}
          </p>
        </div>
      </AlertDescription>
    </Alert>
  );
}

function OcrAssetRow({
  availableOverride = false,
  model,
  name,
  onCancelModelDownload,
  onDeleteModel,
  onDownloadModel,
  statusTextOverride,
}: {
  availableOverride?: boolean;
  model: AiModelAsset | undefined;
  name: string;
  onCancelModelDownload: (modelId: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
  statusTextOverride?: string | undefined;
}) {
  const displayModel =
    model && availableOverride
      ? ({ ...model, downloadStatus: 'available' } satisfies AiModelAsset)
      : model;

  return (
    <div className="flex flex-col gap-2 border p-3 md:flex-row md:items-center md:justify-between">
      <div className="min-w-0">
        <div className="flex min-w-0 items-center gap-2">
          {displayModel ? <ModelStatusIcon model={displayModel} /> : null}
          <div className="min-w-0 text-sm font-medium">{name}</div>
        </div>
        <div className="text-muted-foreground mt-1 pl-6 text-xs">
          {statusTextOverride ?? (model ? ocrAssetStatus(model) : 'Missing')}
        </div>
      </div>
      {availableOverride ? null : (
        <AiModelActionButton
          model={model}
          onCancelModelDownload={onCancelModelDownload}
          onDeleteModel={onDeleteModel}
          onDownloadModel={onDownloadModel}
        />
      )}
    </div>
  );
}

function formatEffectiveOcrStatus(
  ocr: AiProviderStatus['ocr'] | undefined,
): string {
  if (!ocr) {
    return 'Runtime: missing. Data: missing.';
  }

  const languages = ocr.languages ?? [];
  const languageSuffix = languages.length ? ` ${languages.join('+')}` : '';
  const runtimeSource = ocr.runtimeSource ?? ocr.runtimeDownloadStatus;
  const dataSource = ocr.dataSource ?? ocr.dataDownloadStatus;

  return `Runtime: ${runtimeSource}. Data: ${dataSource}${languageSuffix}.`;
}

function systemRuntimeStatusText(ocr: AiProviderStatus['ocr']): string {
  const version = ocr.runtimeVersion ? ` ${ocr.runtimeVersion}` : '';
  const path = ocr.runtimePath ? ` at ${ocr.runtimePath}` : '';

  return `Available from system Tesseract${version}${path}`;
}

function ocrAssetStatus(model: AiModelAsset): string {
  if (model.downloadStatus === 'available') {
    return 'Available';
  }
  if (!model.sourceUrl) {
    return 'Uses system Tesseract until a pinned runtime artifact exists';
  }
  if (model.downloadStatus === 'downloading') {
    return 'Downloading';
  }
  if (model.downloadStatus === 'queued') {
    return 'Queued';
  }
  if (model.downloadStatus === 'failed') {
    return model.errorCode ?? 'Failed';
  }
  if (model.downloadStatus === 'cancelled') {
    return 'Cancelled';
  }

  return 'Not downloaded';
}

function parseOcrEngine(value: string): AiOcrEngine | null {
  return value === 'tesseract' || value === 'vision' ? value : null;
}
