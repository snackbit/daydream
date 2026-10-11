import { invoke as nativeInvoke } from '@tauri-apps/api/core';
import {
  frontendFailureErrorCodes,
  frontendFailureOperations,
  type FrontendFailureInput,
  type FrontendFailureStage,
  type FrontendErrorKind,
} from '@daydream/shared';

const operations = new Set(frontendFailureOperations);
const codes = new Set(frontendFailureErrorCodes);
const reported = new Map<unknown, number>();
const errorKinds = new Set<FrontendErrorKind>([
  'Error',
  'TypeError',
  'RangeError',
  'SyntaxError',
  'ReferenceError',
]);
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

function errorMetadata(
  error: unknown,
): Pick<FrontendFailureInput, 'errorCode' | 'errorKind'> {
  const code =
    typeof error === 'string'
      ? error
      : typeof error === 'object' &&
          error !== null &&
          'code' in error &&
          typeof error.code === 'string'
        ? error.code
        : null;
  const errorCode = code !== null && codes.has(code) ? code : null;
  let errorKind: FrontendErrorKind = errorCode ? 'native-code' : 'other';
  if (error instanceof Error) {
    for (const kind of errorKinds) if (error.name === kind) errorKind = kind;
  }
  return { errorCode, errorKind };
}

export function reportFrontendFailure(
  operation: string,
  stage: FrontendFailureStage,
  error: unknown,
  options: {
    requestId?: string;
    elapsedMs?: number;
    deduplicate?: boolean;
  } = {},
): void {
  try {
    const now = Date.now();
    const previous =
      reported.get(error) ??
      (error instanceof Error ? reported.get(error.cause) : undefined);
    if (
      options.deduplicate !== false &&
      previous !== undefined &&
      now - previous < 2000
    )
      return;
    if (reported.size >= 128) reported.clear();
    reported.set(error, now);
    const input: FrontendFailureInput = {
      operation: operations.has(operation) ? operation : 'frontend_action',
      stage,
      requestId:
        options.requestId && uuid.test(options.requestId)
          ? options.requestId
          : crypto.randomUUID(),
      ...errorMetadata(error),
      elapsedMs:
        options.elapsedMs === undefined || !Number.isFinite(options.elapsedMs)
          ? null
          : Math.max(0, Math.min(86400000, Math.round(options.elapsedMs))),
    };
    // Report through the raw transport: logging must never recurse or replace the failure.
    void Promise.resolve()
      .then(() => nativeInvoke('log_frontend_failure', { input }))
      .catch(() => {
        console.warn('frontend_failure_logging_unavailable', input);
      });
  } catch {
    console.warn('frontend_failure_metadata_unavailable');
  }
}

export function installFrontendFailureLogging(): () => void {
  function onError(event: Event) {
    if (event instanceof ErrorEvent)
      reportFrontendFailure('window_error', 'render', event.error);
    else
      reportFrontendFailure('resource_load', 'resource', null, {
        deduplicate: false,
      });
  }
  function onRejection(event: PromiseRejectionEvent) {
    reportFrontendFailure(
      'unhandled_rejection',
      'unhandled-rejection',
      event.reason,
    );
  }
  window.addEventListener('error', onError, true);
  window.addEventListener('unhandledrejection', onRejection);
  return () => {
    window.removeEventListener('error', onError, true);
    window.removeEventListener('unhandledrejection', onRejection);
  };
}
