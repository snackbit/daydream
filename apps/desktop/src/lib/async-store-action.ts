import { reportFrontendFailure } from './diagnostics/failure-logging';

type AsyncStoreActionOptions<Result> = {
  before?: () => void;
  cleanup?: () => void | Promise<void>;
  failure?: (error: unknown) => boolean | void | Promise<boolean | void>;
  rethrow?: boolean;
  run: () => Promise<Result>;
  success?: (result: Result) => void | Promise<void>;
};

export async function runAsyncStoreAction<Result>({
  before,
  cleanup,
  failure,
  rethrow = true,
  run,
  success,
}: AsyncStoreActionOptions<Result>): Promise<Result | undefined> {
  try {
    before?.();
  } catch (error) {
    reportFrontendFailure('frontend_action', 'execute', error);
    throw error;
  }

  try {
    const result = await run();
    await success?.(result);

    return result;
  } catch (error) {
    reportFrontendFailure('frontend_action', 'execute', error);
    const failureResult = await failure?.(error);
    if (rethrow && failureResult !== false) {
      throw error;
    }

    return undefined;
  } finally {
    await (async () => {
      try {
        await cleanup?.();
      } catch (error) {
        reportFrontendFailure('frontend_action', 'execute', error);
        throw error;
      }
    })();
  }
}

export function unknownErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : JSON.stringify(error);
}

export function sleep(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
