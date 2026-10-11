import policy from './failure-policy.json' with { type: 'json' };

/** Closed metadata policy: never accept arbitrary error messages or payload fields. */
export const frontendFailureOperations: readonly string[] = policy.operations;
export const frontendFailureErrorCodes: readonly string[] = policy.errorCodes;
export type FrontendFailureStage =
  | 'invoke'
  | 'decode'
  | 'subscribe'
  | 'callback'
  | 'execute'
  | 'render'
  | 'unhandled-rejection'
  | 'resource';
export type FrontendErrorKind =
  | 'Error'
  | 'TypeError'
  | 'RangeError'
  | 'SyntaxError'
  | 'ReferenceError'
  | 'native-code'
  | 'other';
export interface FrontendFailureInput {
  requestId: string;
  operation: string;
  stage: FrontendFailureStage;
  errorCode: string | null;
  errorKind: FrontendErrorKind;
  elapsedMs: number | null;
}
