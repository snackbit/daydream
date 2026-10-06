import { describeProcessingError } from '@/features/processing/processing-errors';

export function aiProcessingErrorSummary(errorCode: string): string {
  return describeProcessingError(errorCode).summary;
}

export function aiProcessingErrorDetails(errorCode: string): string {
  const summary = aiProcessingErrorSummary(errorCode);

  return summary === errorCode ? errorCode : `${summary} (${errorCode})`;
}
