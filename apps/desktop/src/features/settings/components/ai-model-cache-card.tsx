import type { AiModelAsset } from '@daydream/shared';
import {
  CheckCircle2,
  Download,
  HardDriveDownload,
  LoaderCircle,
  Trash2,
  TriangleAlert,
  XCircle,
} from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';

interface AiModelCacheCardProps {
  models: AiModelAsset[];
  onCancelModelDownload: (modelId: string) => void;
  onDeleteModel: (modelId: string) => void;
  onDownloadModel: (modelId: string) => void;
}

export function AiModelCacheCard({
  models,
  onCancelModelDownload,
  onDeleteModel,
  onDownloadModel,
}: AiModelCacheCardProps) {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Model Cache</CardTitle>
        <CardDescription>
          Downloads use pinned Hugging Face GGML files and SHA-1 verification.
        </CardDescription>
      </CardHeader>
      <CardContent>
        <div className="grid gap-2">
          {models.map((model) => (
            <ModelCacheRow
              key={model.id}
              model={model}
              onCancelModelDownload={onCancelModelDownload}
              onDeleteModel={onDeleteModel}
              onDownloadModel={onDownloadModel}
            />
          ))}
        </div>
      </CardContent>
    </Card>
  );
}

interface ModelCacheRowProps extends Omit<AiModelCacheCardProps, 'models'> {
  model: AiModelAsset;
}

interface AiModelActionButtonProps extends Omit<
  AiModelCacheCardProps,
  'models'
> {
  model: AiModelAsset | undefined;
}

export function AiModelActionButton({
  model,
  onCancelModelDownload,
  onDeleteModel,
  onDownloadModel,
}: AiModelActionButtonProps) {
  if (!model) {
    return null;
  }

  const isActive =
    model.downloadStatus === 'downloading' || model.downloadStatus === 'queued';
  const isAvailable = model.downloadStatus === 'available';

  if (isAvailable) {
    return (
      <Button
        onClick={() => onDeleteModel(model.id)}
        type="button"
        variant="destructive"
      >
        <Trash2 className="size-3.5" />
        Delete
      </Button>
    );
  }

  if (isActive) {
    return (
      <Button
        onClick={() => onCancelModelDownload(model.id)}
        type="button"
        variant="outline"
      >
        <XCircle className="size-3.5" />
        Cancel
      </Button>
    );
  }

  return (
    <Button
      onClick={() => onDownloadModel(model.id)}
      type="button"
      variant="outline"
    >
      <Download className="size-3.5" />
      Download
    </Button>
  );
}

function ModelCacheRow({
  model,
  onCancelModelDownload,
  onDeleteModel,
  onDownloadModel,
}: ModelCacheRowProps) {
  const isActive =
    model.downloadStatus === 'downloading' || model.downloadStatus === 'queued';

  return (
    <div className="flex flex-col gap-2 border p-3 md:flex-row md:items-center md:justify-between">
      <div className="min-w-0">
        <div className="flex min-w-0 items-center gap-2">
          <ModelStatusIcon model={model} />
          <div className="min-w-0 text-sm font-medium">{model.label}</div>
        </div>
        <div className="text-muted-foreground mt-1 flex flex-wrap items-center gap-x-2 gap-y-1 pl-6 text-xs">
          <span>{modelStatusText(model)}</span>
          <span>{modelSizeLabel(model)}</span>
        </div>
        {isActive ? <ModelDownloadProgress model={model} /> : null}
        {model.errorCode && model.downloadStatus !== 'cancelled' ? (
          <div className="text-destructive mt-1 pl-6 text-xs">
            {modelErrorText(model.errorCode)}
          </div>
        ) : null}
      </div>
      <div className="flex flex-wrap gap-2">
        <AiModelActionButton
          model={model}
          onCancelModelDownload={onCancelModelDownload}
          onDeleteModel={onDeleteModel}
          onDownloadModel={onDownloadModel}
        />
      </div>
    </div>
  );
}

function ModelDownloadProgress({ model }: { model: AiModelAsset }) {
  const progress = downloadProgressPercent(model);

  return (
    <div className="mt-2 grid gap-1 pl-6">
      <Progress
        aria-label={`${model.label} download progress`}
        className="h-1.5"
        value={progress}
      />
      <div className="text-muted-foreground text-xs">
        {downloadProgressLabel(model)}
      </div>
    </div>
  );
}

function ModelStatusIcon({ model }: { model: AiModelAsset }) {
  const className =
    model.downloadStatus === 'available'
      ? 'text-primary size-4 shrink-0'
      : model.downloadStatus === 'failed'
        ? 'text-destructive size-4 shrink-0'
        : 'text-muted-foreground size-4 shrink-0';

  if (model.downloadStatus === 'available') {
    return <CheckCircle2 aria-hidden="true" className={className} />;
  }
  if (
    model.downloadStatus === 'downloading' ||
    model.downloadStatus === 'queued'
  ) {
    return <LoaderCircle aria-hidden="true" className={className} />;
  }
  if (model.downloadStatus === 'failed') {
    return <TriangleAlert aria-hidden="true" className={className} />;
  }
  if (model.downloadStatus === 'cancelled') {
    return <XCircle aria-hidden="true" className={className} />;
  }

  return <HardDriveDownload aria-hidden="true" className={className} />;
}

function downloadProgressLabel(model: AiModelAsset): string {
  if (model.downloadStatus === 'queued') {
    return 'Waiting to download';
  }

  if (model.totalBytes && model.totalBytes > 0) {
    const percent = downloadProgressPercent(model);

    return `${formatBytes(model.downloadedBytes)} of ${formatBytes(
      model.totalBytes,
    )} (${percent}%)`;
  }

  return `${formatBytes(model.downloadedBytes)} downloaded`;
}

function downloadProgressPercent(model: AiModelAsset): number {
  if (!model.totalBytes || model.totalBytes <= 0) {
    return 0;
  }

  return Math.min(
    100,
    Math.max(0, Math.round((model.downloadedBytes / model.totalBytes) * 100)),
  );
}

function modelSizeLabel(model: AiModelAsset): string {
  return model.sizeBytes ? formatBytes(model.sizeBytes) : 'Size unknown';
}

function modelStatusText(model: AiModelAsset): string {
  if (model.downloadStatus === 'available') {
    return 'Downloaded';
  }
  if (model.downloadStatus === 'downloading') {
    return 'Downloading';
  }
  if (model.downloadStatus === 'queued') {
    return 'Queued';
  }
  if (model.downloadStatus === 'failed') {
    return 'Failed';
  }
  if (model.downloadStatus === 'cancelled') {
    return 'Cancelled';
  }

  return 'Not downloaded';
}

function modelErrorText(errorCode: string): string {
  if (errorCode === 'ai_model_download_cancelled') {
    return 'Cancelled';
  }

  return errorCode;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024 * 1024)).toFixed(1)} GiB`;
  }
  if (bytes >= 1024 * 1024) {
    return `${Math.round(bytes / (1024 * 1024))} MiB`;
  }

  return `${bytes} B`;
}
