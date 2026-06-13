import type {
  AiModelAsset,
  AiProviderStatus,
  AiVisionProviderKind,
  AppSettings,
} from '@daydream/shared';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
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
  updateAiVision,
} from '@/features/settings/settings-draft-updaters';
import { AiModelActionButton, ModelStatusIcon } from './ai-model-cache-card';
import { CheckboxField, RemoteProviderFields } from './ai-settings-fields';

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
  const { ocr, vision } = draft.ai;
  const remoteVision = vision.provider === 'openai-compatible';
  const tesseractRuntime = models.find(
    (model) => model.id === 'tesseract-runtime',
  );
  const tessdataEng = models.find((model) => model.id === 'tessdata-eng');
  const tessdataDeu = models.find((model) => model.id === 'tessdata-deu');
  const ocrStatus = providerStatus?.ocr;
  const systemRuntimeAvailable = ocrStatus?.runtimeSource === 'system';

  return (
    <Card>
      <CardHeader>
        <CardTitle>OCR and Vision</CardTitle>
        <CardDescription>
          OCR is local. VLM observations require an explicitly configured remote
          provider.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <FieldGroup>
          <CheckboxField
            checked={ocr.enabled}
            description="Tesseract is the only OCR engine in M2."
            label="Run OCR on retained frames"
            onCheckedChange={(enabled) => {
              updateAiOcr(onUpdateDraft, (current) => ({
                ...current,
                enabled,
              }));
            }}
          />
          <div className="grid gap-2">
            <p className="text-muted-foreground text-xs">
              {formatEffectiveOcrStatus(ocrStatus)}
            </p>
            <OcrAssetRow
              availableOverride={systemRuntimeAvailable}
              model={tesseractRuntime}
              name="Runtime"
              onCancelModelDownload={onCancelModelDownload}
              onDeleteModel={onDeleteModel}
              onDownloadModel={onDownloadModel}
              statusTextOverride={
                systemRuntimeAvailable
                  ? systemRuntimeStatusText(ocrStatus)
                  : undefined
              }
            />
            <OcrAssetRow
              model={tessdataEng}
              name="Best English OCR data"
              onCancelModelDownload={onCancelModelDownload}
              onDeleteModel={onDeleteModel}
              onDownloadModel={onDownloadModel}
            />
            <OcrAssetRow
              model={tessdataDeu}
              name="Best German OCR data"
              onCancelModelDownload={onCancelModelDownload}
              onDeleteModel={onDeleteModel}
              onDownloadModel={onDownloadModel}
            />
          </div>
          <Field>
            <FieldLabel htmlFor="ai-vision-provider">
              Vision provider
            </FieldLabel>
            <Select
              onValueChange={(value) => {
                const provider = value as AiVisionProviderKind;
                updateAiVision(onUpdateDraft, (current) => ({
                  ...current,
                  apiKeyRef:
                    provider === 'openai-compatible' ? current.apiKeyRef : null,
                  provider,
                }));
              }}
              value={vision.provider}
            >
              <SelectTrigger id="ai-vision-provider">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="disabled">Disabled</SelectItem>
                <SelectItem value="openai-compatible">
                  OpenAI-compatible
                </SelectItem>
              </SelectContent>
            </Select>
          </Field>
          {remoteVision ? (
            <>
              <RemoteProviderFields
                baseUrl={vision.baseUrl ?? ''}
                hasSavedKey={Boolean(vision.apiKeyRef)}
                keyDraft={visionKeyDraft}
                onBaseUrlChange={(value) => {
                  updateAiVision(onUpdateDraft, (current) => ({
                    ...current,
                    baseUrl: value || null,
                  }));
                }}
                onKeyChange={onChangeVisionKey}
                prefix="vision"
              />
              <Field>
                <FieldLabel htmlFor="ai-vision-model">Vision model</FieldLabel>
                <Input
                  id="ai-vision-model"
                  onChange={(event) => {
                    updateAiVision(onUpdateDraft, (current) => ({
                      ...current,
                      model: event.target.value || null,
                    }));
                  }}
                  placeholder="Model name"
                  value={vision.model ?? ''}
                />
              </Field>
            </>
          ) : null}
        </FieldGroup>
      </CardContent>
    </Card>
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
