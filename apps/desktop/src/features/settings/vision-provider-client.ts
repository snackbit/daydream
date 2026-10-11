import type {
  TestVisionProviderInput,
  VisionProviderTestResult,
} from '@daydream/shared';
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
export async function testVisionProvider(
  input: TestVisionProviderInput,
): Promise<VisionProviderTestResult> {
  const result: unknown = await invoke('testVisionProvider', { input });
  return decodeResponse('testVisionProvider', result, decodeVisionTest);
}
function decodeVisionTest(result: unknown): VisionProviderTestResult {
  if (!record(result) || !Array.isArray(result.checks))
    throw new Error('The provider test returned an invalid response.');
  const checks: VisionProviderTestResult['checks'] = [];
  for (const check of result.checks) {
    if (
      !record(check) ||
      (check.kind !== 'ocr' && check.kind !== 'observations') ||
      (check.status !== 'succeeded' && check.status !== 'failed') ||
      (check.errorCode !== null && typeof check.errorCode !== 'string') ||
      typeof check.elapsedMs !== 'number' ||
      !Number.isSafeInteger(check.elapsedMs) ||
      check.elapsedMs < 0
    )
      throw new Error('The provider test returned an invalid response.');
    checks.push({
      kind: check.kind,
      status: check.status,
      errorCode: check.errorCode,
      elapsedMs: check.elapsedMs,
    });
    if (check.status === 'failed')
      reportFrontendFailure('testVisionProvider', 'execute', check.errorCode, {
        elapsedMs: check.elapsedMs,
        deduplicate: false,
      });
  }
  return { checks };
}
