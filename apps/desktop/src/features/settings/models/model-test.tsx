import { useEffect, useRef, useState } from 'react';
import type {
  AiConfiguredModel,
  AiModelConnection,
  AiModelTestResult,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  cancelModelRequest,
  modelActionError,
  testConfiguredModel,
} from './model-client';

export function ModelTest({
  connection,
  model,
  keyDraft,
  onDimensions,
}: {
  connection: AiModelConnection;
  model: AiConfiguredModel;
  keyDraft: string;
  onDimensions: (dimensions: number) => void;
}) {
  const fingerprint = JSON.stringify({
    connection,
    model: { ...model, embeddingDimensions: null },
    keyDraft,
  });
  const pending = useRef<string | null>(null);
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<{
    fingerprint: string;
    result: AiModelTestResult;
  } | null>(null);
  const [failure, setFailure] = useState<{
    fingerprint: string;
    message: string;
  } | null>(null);
  useEffect(
    () => () => {
      const request = pending.current;
      pending.current = null;
      if (request) void cancelModelRequest(request).catch(() => undefined);
    },
    [fingerprint],
  );
  async function run() {
    if (pending.current) return;
    const requestId = crypto.randomUUID();
    pending.current = requestId;
    setRunning(true);
    setFailure(null);
    try {
      const result = await testConfiguredModel({
        connection,
        model,
        keyDraft: keyDraft || null,
        requestId,
      });
      if (pending.current !== requestId) return;
      setResult({ fingerprint, result });
      const dimensions = result.checks.find(
        (check) =>
          check.capability === 'embedding' && check.status === 'succeeded',
      )?.dimensions;
      if (dimensions) onDimensions(dimensions);
    } catch (error) {
      if (pending.current === requestId)
        setFailure({ fingerprint, message: modelActionError(error) });
    } finally {
      if (pending.current === requestId) pending.current = null;
      setRunning(false);
    }
  }
  function cancel() {
    if (pending.current)
      void cancelModelRequest(pending.current).catch(() => undefined);
  }
  return (
    <div className="grid gap-2">
      <div className="flex flex-wrap items-center gap-3">
        <Button
          type="button"
          variant="outline"
          size="sm"
          disabled={
            running || !model.model.trim() || !model.capabilities.length
          }
          onClick={() => void run()}
        >
          {running ? 'Testing…' : 'Test model'}
        </Button>
        {running ? (
          <Button type="button" variant="ghost" size="sm" onClick={cancel}>
            Cancel test
          </Button>
        ) : null}
        <span className="text-muted-foreground text-xs">
          Synthetic text and images only. No recordings are sent.
        </span>
      </div>
      <div aria-live="polite">
        {result?.fingerprint === fingerprint
          ? result.result.checks.map((check) => (
              <p
                key={check.capability}
                className={
                  check.status === 'failed'
                    ? 'text-destructive text-sm'
                    : 'text-muted-foreground text-sm'
                }
              >
                {check.capability}: {check.message}
                {check.dimensions ? ` ${check.dimensions} dimensions.` : ''}
              </p>
            ))
          : null}
        {failure?.fingerprint === fingerprint ? (
          <p role="alert" className="text-destructive text-sm">
            {failure.message}
          </p>
        ) : null}
      </div>
    </div>
  );
}
