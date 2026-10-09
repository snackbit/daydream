import { useCallback, useEffect, useState } from 'react';
import type { AiModelAsset, BackgroundTask } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { listAiModels, listenToAiModelsChanged } from '../ai-client';
import { ModelTransferRow } from './model-transfer-row';

export interface DownloadTaskRowProps {
  task: BackgroundTask;
  onChanged?: (() => Promise<void>) | undefined;
}
export function DownloadTaskRow({ task, onChanged }: DownloadTaskRowProps) {
  const [models, setModels] = useState<AiModelAsset[]>([]);
  const [readStatus, setReadStatus] = useState<'loading' | 'ready' | 'error'>(
    'loading',
  );
  const [subscriptionError, setSubscriptionError] = useState(false);
  const [revision, setRevision] = useState(0);
  const refresh = useCallback(() => {
    setReadStatus('loading');
    setSubscriptionError(false);
    setRevision((value) => value + 1);
  }, []);
  useEffect(() => {
    let active = true;
    let version = 0;
    let unlisten: (() => void) | undefined;
    const request = version;
    void listAiModels()
      .then((result) => {
        if (active && version === request) {
          setModels(result.models);
          setReadStatus('ready');
        }
      })
      .catch(() => {
        if (active && version === request) setReadStatus('error');
      });
    void listenToAiModelsChanged((result) => {
      version += 1;
      if (active) {
        setModels(result.models);
        setReadStatus('ready');
      }
    })
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => {
        if (active) setSubscriptionError(true);
      });
    return () => {
      active = false;
      unlisten?.();
    };
  }, [revision]);
  const model = models.find((model) => model.id === task.modelId);
  return (
    <li className="px-4">
      {model ? (
        <ModelTransferRow model={model} onChanged={onChanged} />
      ) : (
        <p className="text-muted-foreground py-3 text-sm">
          {task.title} ·{' '}
          {readStatus === 'loading'
            ? 'Loading download status…'
            : readStatus === 'error'
              ? 'Download status could not be loaded.'
              : 'This download is no longer available.'}
        </p>
      )}
      {readStatus === 'error' || subscriptionError ? (
        <div className="flex flex-wrap items-center justify-between gap-2 pb-3">
          <p className="text-muted-foreground text-xs" role="alert">
            {readStatus === 'error'
              ? 'Could not refresh download status. Try again.'
              : 'Live download updates are unavailable. Refresh to check progress.'}
          </p>
          <Button size="sm" variant="outline" onClick={refresh}>
            Refresh download status
          </Button>
        </div>
      ) : null}
    </li>
  );
}
