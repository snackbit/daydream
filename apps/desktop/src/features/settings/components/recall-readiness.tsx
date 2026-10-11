import { useEffect } from 'react';
import type { AiModelAsset, RecallSettings } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { getRecallStatus } from '@/features/recall/recall-client';
import { useRecallRead } from '@/features/recall/use-recall-read';

export interface RecallReadinessProps {
  settings?: RecallSettings | undefined;
  models?: AiModelAsset[] | undefined;
  onDownloadModel?: ((modelId: string) => void) | undefined;
  onChooseCpu?: (() => void) | undefined;
}
export function RecallReadiness({
  settings,
  models,
  onDownloadModel,
  onChooseCpu,
}: RecallReadinessProps) {
  const { state, refresh, isRefreshing, checkedAt } =
    useRecallRead(getRecallStatus);
  const selectedModel = models?.find(
    (model) => model.id === settings?.embedding.localModelId,
  );
  const terminal = selectedModel?.downloadStatus;
  const selfTest = selectedModel?.selfTestStatus;
  useEffect(() => {
    refresh();
  }, [terminal, selfTest, selectedModel?.id, refresh]);
  function downloadSelectedModel() {
    if (settings) onDownloadModel?.(settings.embedding.localModelId);
  }
  const missingLocalModel =
    state.status === 'ready' &&
    state.data.embeddingUnavailableReason ===
      'recall_embedding_model_missing' &&
    settings?.embedding.provider === 'local';
  return (
    <section
      aria-label="Recall readiness"
      className="bg-muted rounded-panel p-6"
    >
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-sm font-medium">Saved Recall configuration</h2>
        <Button
          onClick={refresh}
          disabled={isRefreshing}
          variant="outline"
          size="sm"
        >
          {isRefreshing ? 'Checking…' : 'Check readiness'}
        </Button>
      </div>
      {state.status === 'ready' ? (
        <div className="mt-4 grid gap-3 text-sm sm:grid-cols-2">
          <p>
            Embeddings:{' '}
            {state.data.embeddingReady
              ? 'Ready'
              : readinessReason(
                  state.data.embeddingUnavailableReason,
                  'embedding',
                  settings?.embedding.provider,
                )}
            {missingLocalModel ? (
              <Button
                className="ml-2"
                size="sm"
                variant="outline"
                disabled={
                  selectedModel?.downloadStatus === 'downloading' ||
                  selectedModel?.downloadStatus === 'queued'
                }
                onClick={downloadSelectedModel}
              >
                Download selected model
              </Button>
            ) : null}
            {state.data.embeddingUnavailableReason ===
              'recall_embedding_vulkan_unavailable' ||
            (state.data.embeddingUnavailableReason ===
              'recall_embedding_model_test_failed' &&
              settings?.embedding.localBackend !== 'cpu') ? (
              <Button
                className="ml-2"
                size="sm"
                variant="outline"
                onClick={onChooseCpu}
              >
                Choose CPU
              </Button>
            ) : null}
          </p>
          <p>
            Generation:{' '}
            {state.data.generationReady
              ? 'Ready'
              : readinessReason(
                  state.data.generationUnavailableReason,
                  'generation',
                )}
          </p>
          <p className="text-muted-foreground text-xs">
            {state.data.indexedEvidenceCount} text and transcript entries
            indexed · {state.data.pendingEvidenceCount} pending
          </p>
          {state.data.jobs.some(
            (job) => job.kind === 'index' && job.status === 'failed',
          ) ? (
            <p className="text-muted-foreground text-xs">
              An indexing job failed. Model readiness is checked separately;
              review processing for recovery.
            </p>
          ) : null}
        </div>
      ) : (
        <p
          className="text-muted-foreground mt-4 text-sm"
          role={state.status === 'error' ? 'alert' : 'status'}
        >
          {state.status === 'error'
            ? 'Could not check readiness. Try again.'
            : 'Checking Recall readiness…'}
        </p>
      )}
      {state.status === 'ready' && checkedAt !== null ? (
        <p
          role="status"
          aria-label="Readiness check"
          className="text-muted-foreground mt-3 text-xs"
        >
          {isRefreshing
            ? 'Checking saved configuration…'
            : `Readiness checked at ${new Date(checkedAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit' })}.`}
        </p>
      ) : null}
      <p className="text-muted-foreground mt-3 text-xs">
        Readiness uses saved settings. Save draft changes before checking them.
      </p>
    </section>
  );
}

function readinessReason(
  reason: string | null,
  kind: 'embedding' | 'generation',
  provider?: 'local' | 'openai-compatible',
): string {
  if (reason?.includes('subscription'))
    return 'Sign in with ChatGPT again in Models';
  if (reason === 'recall_generation_disabled')
    return 'Optional generation is turned off';
  if (reason === 'recall_embedding_remote_disabled')
    return 'Enable and configure a remote embedding provider below';
  if (reason === 'recall_embedding_model_missing')
    return provider === 'local'
      ? 'Download the selected model to enable semantic search'
      : 'Choose a remote embedding model below';
  if (reason === 'recall_embedding_vulkan_unavailable')
    return 'Vulkan is unavailable on this device. Choose CPU and save your settings';
  if (reason === 'recall_embedding_model_test_failed')
    return 'The model was downloaded, but its load test failed. Check the local runtime or choose CPU';
  if (reason?.includes('model_missing'))
    return 'Choose a model in the provider settings below';
  if (
    reason?.includes('key') ||
    reason?.includes('secret') ||
    reason?.includes('credential')
  )
    return 'Configure the provider credentials below';
  if (
    reason?.includes('url') ||
    reason?.includes('configuration') ||
    reason?.includes('endpoint')
  )
    return 'Complete the provider configuration below';
  return kind === 'embedding'
    ? 'Semantic search is not ready. Check the embedding settings below'
    : 'Generation is not ready. Check the provider settings below';
}
