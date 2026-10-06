import { useRef, useState } from 'react';
import type {
  AiVisionProviderSettings,
  VisionProviderTestResult,
} from '@daydream/shared';
import { Button } from '@/components/ui/button';
import {
  processingFailureSummary,
  describeProcessingError,
} from '@/features/processing/processing-errors';
import { testVisionProvider } from '../../vision-provider-client';
import { missingVisionRequirements } from './vision-settings-format';

export function VisionProviderTest({
  vision,
  keyDraft,
  ocr,
  observations,
}: {
  vision: AiVisionProviderSettings;
  keyDraft: string;
  ocr: boolean;
  observations: boolean;
}) {
  const [outcome, setOutcome] = useState<{
    fingerprint: string;
    testedKey: string;
    result: VisionProviderTestResult | null;
    error: string | null;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const request = useRef(0);
  // The fingerprint includes only public configuration; the key is compared by
  // identity through a request snapshot and never rendered or logged.
  const fingerprint = JSON.stringify([vision, ocr, observations]);
  const current =
    outcome?.fingerprint === fingerprint && outcome.testedKey === keyDraft
      ? outcome
      : null;
  const missing = missingVisionRequirements({
    vision,
    visionKeyDraft: keyDraft,
  });
  async function run() {
    if (busy) return;
    const id = ++request.current;
    const testedKey = keyDraft;
    setBusy(true);
    try {
      const result = await testVisionProvider({
        vision,
        keyDraft: vision.authMode === 'none' ? null : keyDraft.trim() || null,
        ocr,
        observations,
      });
      if (request.current === id) {
        setOutcome({ fingerprint, testedKey, result, error: null });
      }
    } catch (error) {
      if (request.current === id) {
        setOutcome({
          fingerprint,
          testedKey,
          result: null,
          error: processingFailureSummary(error),
        });
      }
    } finally {
      if (request.current === id) setBusy(false);
    }
  }
  return (
    <div className="space-y-1">
      <Button
        disabled={busy || missing.length > 0}
        onClick={run}
        size="sm"
        variant="outline"
      >
        {busy ? 'Testing Vision…' : 'Test Vision'}
      </Button>
      <p className="text-muted-foreground text-[11px]">
        Sends a generated test image using this form. Does not save changes.
      </p>
      {current?.error ? (
        <p role="alert" className="text-destructive text-xs">
          {current.error}
        </p>
      ) : null}
      {current?.result?.checks.map((check) => (
        <p
          key={check.kind}
          role={check.status === 'failed' ? 'alert' : 'status'}
          className="text-xs"
        >
          {check.kind === 'ocr' ? 'Text recognition' : 'Screen observations'}:{' '}
          {check.status === 'succeeded'
            ? `Connected (${check.elapsedMs} ms)`
            : describeProcessingError(
                check.errorCode ?? 'ai_vision_response_invalid',
              ).summary}
        </p>
      ))}
    </div>
  );
}
