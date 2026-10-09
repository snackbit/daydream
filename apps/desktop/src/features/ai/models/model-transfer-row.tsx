import { useCallback, useRef, useState } from 'react';
import type { AiModelAsset } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  cancelAiModelDownload,
  deleteAiModel,
  downloadAiModel,
} from '../ai-client';
import {
  pauseAiModelDownload,
  resumeAiModelDownload,
  removeAiModelPartial,
} from './model-transfer-client';
import type { ModelTransferAction } from './model-transfer-client';
import {
  modelTransferError,
  modelTransferState,
} from './model-transfer-format';
import { ModelTransferProgress } from './model-transfer-progress';

export interface ModelTransferRowProps {
  model: AiModelAsset;
  actionError?: string | undefined;
  onDownloadModel?: ((modelId: string) => void) | undefined;
  onCancelModelDownload?: ((modelId: string) => void) | undefined;
  onDeleteModel?: ((modelId: string) => void) | undefined;
  onChanged?: (() => Promise<void>) | undefined;
  onTransferAction?:
    | ((modelId: string, action: ModelTransferAction) => Promise<void>)
    | undefined;
}
export function ModelTransferRow({
  model,
  actionError,
  onDownloadModel,
  onCancelModelDownload,
  onDeleteModel,
  onChanged,
  onTransferAction,
}: ModelTransferRowProps) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const admission = useRef(false);
  const perform = useCallback(
    async (action: ModelTransferAction) => {
      if (admission.current) return;
      admission.current = true;
      setPending(true);
      setError(null);
      const input = { modelId: model.id };
      try {
        if (onTransferAction) {
          await onTransferAction(model.id, action);
          return;
        }
        switch (action) {
          case 'download':
            if (onDownloadModel) await onDownloadModel(model.id);
            else await downloadAiModel(input);
            break;
          case 'cancel':
            if (onCancelModelDownload) await onCancelModelDownload(model.id);
            else await cancelAiModelDownload(input);
            break;
          case 'delete':
            if (onDeleteModel) await onDeleteModel(model.id);
            else await deleteAiModel(input);
            break;
          case 'pause':
            await pauseAiModelDownload(input);
            break;
          case 'resume':
            await resumeAiModelDownload(input);
            break;
          case 'remove':
            await removeAiModelPartial(input);
            break;
        }
        await onChanged?.();
      } catch (error) {
        setError(modelTransferError(error));
      } finally {
        admission.current = false;
        setPending(false);
      }
    },
    [
      model.id,
      onChanged,
      onDownloadModel,
      onCancelModelDownload,
      onDeleteModel,
      onTransferAction,
    ],
  );
  const download = useCallback(() => void perform('download'), [perform]);
  const pause = useCallback(() => void perform('pause'), [perform]);
  const resume = useCallback(() => void perform('resume'), [perform]);
  const cancel = useCallback(() => void perform('cancel'), [perform]);
  const remove = useCallback(() => void perform('remove'), [perform]);
  const deleteModel = useCallback(() => void perform('delete'), [perform]);
  const active =
    model.downloadStatus === 'downloading' || model.downloadStatus === 'queued';
  const partial =
    model.downloadedBytes > 0 && model.downloadStatus !== 'available';
  const failure =
    actionError ??
    error ??
    (model.errorCode && model.downloadStatus !== 'cancelled'
      ? modelTransferError({ code: model.errorCode })
      : null);
  return (
    <div
      role="group"
      className="grid min-w-0 gap-2 py-3"
      aria-label={model.label}
      aria-busy={pending}
    >
      <div className="flex flex-wrap items-start justify-between gap-2">
        <div className="min-w-0">
          <p className="text-sm font-medium wrap-break-word">{model.label}</p>
          <p className="text-muted-foreground text-xs" role="status">
            {modelTransferState(model)}
          </p>
        </div>
        <div className="flex flex-wrap gap-1">
          {active ? (
            <>
              <Button
                size="sm"
                variant="outline"
                disabled={pending}
                onClick={pause}
              >
                Pause
              </Button>
              <Button
                size="sm"
                variant="ghost"
                disabled={pending}
                onClick={cancel}
              >
                Cancel
              </Button>
            </>
          ) : model.downloadStatus === 'available' ? (
            <Button
              size="sm"
              variant="ghost"
              disabled={pending}
              onClick={deleteModel}
            >
              Delete
            </Button>
          ) : (
            <Button
              size="sm"
              variant="outline"
              disabled={pending || !model.sourceUrl}
              onClick={model.downloadStatus === 'paused' ? resume : download}
            >
              {model.downloadStatus === 'paused'
                ? 'Resume'
                : model.downloadStatus === 'failed' ||
                    model.downloadStatus === 'cancelled'
                  ? 'Retry'
                  : 'Download'}
            </Button>
          )}
          {model.downloadStatus === 'paused' ? (
            <Button
              size="sm"
              variant="ghost"
              disabled={pending}
              onClick={cancel}
            >
              Cancel
            </Button>
          ) : null}
          {partial && !active ? (
            <Button
              size="sm"
              variant="ghost"
              disabled={pending}
              onClick={remove}
            >
              Remove partial data
            </Button>
          ) : null}
        </div>
      </div>
      {model.downloadStatus !== 'missing' ? (
        <ModelTransferProgress model={model} />
      ) : null}
      {failure ? (
        <p className="text-destructive text-xs" role="alert">
          {failure}
        </p>
      ) : null}
    </div>
  );
}
