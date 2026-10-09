import type { AiModelAsset } from '@daydream/shared';
import { Progress } from '@/components/ui/progress';
import {
  formatModelBytes,
  modelTransferPercent,
  modelTransferState,
} from './model-transfer-format';

export interface ModelTransferProgressProps {
  model: AiModelAsset;
}
export function ModelTransferProgress({ model }: ModelTransferProgressProps) {
  const speed = model.transfer?.bytesPerSecond;
  const eta = model.transfer?.etaMs;
  return (
    <div className="grid gap-1.5">
      <Progress
        className="h-1.5"
        aria-label={`${model.label} download progress`}
        value={modelTransferPercent(model)}
      />
      <div className="text-muted-foreground flex flex-wrap justify-between gap-x-3 text-xs tabular-nums">
        <span>
          {modelTransferState(model)} ·{' '}
          {formatModelBytes(model.downloadedBytes)}
          {model.totalBytes
            ? ` of ${formatModelBytes(model.totalBytes)}`
            : ' downloaded'}
        </span>
        <span>
          {speed && speed > 0 ? `${formatModelBytes(speed)}/s` : ''}
          {eta !== null && eta !== undefined
            ? ` · ${Math.max(1, Math.ceil(eta / 60000))} min remaining`
            : ''}
        </span>
      </div>
    </div>
  );
}
