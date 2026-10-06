import { useCallback } from 'react';
import type {
  AiModelAsset,
  AiOcrEngine,
  AiProviderStatus,
} from '@daydream/shared';
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field';
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
import { AiModelActionButton, ModelStatusIcon } from '../ai-model-cache-card';

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
  const handleOcrEngineChange = useCallback(
    (value: string) => {
      if (value === 'off') {
        updateAiOcr(actions.onUpdateDraft, (current) => ({
          ...current,
          enabled: false,
        }));
        return;
      }
      const engine = parseOcrEngine(value);
      if (!engine) {
        return;
      }
      updateAiOcr(actions.onUpdateDraft, (current) => ({
        ...current,
        engine,
        enabled: true,
      }));
    },
    [actions.onUpdateDraft],
  );

  return (
    <div className="grid gap-3">
      <FieldGroup>
        <Field>
          <FieldLabel htmlFor="ai-ocr-engine">
            Text recognition engine
          </FieldLabel>
          <Select
            onValueChange={handleOcrEngineChange}
            value={model.enabled ? model.engine : 'off'}
          >
            <SelectTrigger id="ai-ocr-engine">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              <SelectItem value="off">Off</SelectItem>
              <SelectItem value="tesseract">Tesseract (local)</SelectItem>
              <SelectItem value="vision">Vision model</SelectItem>
            </SelectContent>
          </Select>
        </Field>
      </FieldGroup>
      {model.enabled && model.engine === 'tesseract' ? (
        <TesseractAssets actions={actions} model={model} />
      ) : null}
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
