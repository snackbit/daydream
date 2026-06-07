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
  before?.();

  try {
    const result = await run();
    await success?.(result);

    return result;
  } catch (error) {
    const failureResult = await failure?.(error);
    if (rethrow && failureResult !== false) {
      throw error;
    }

    return undefined;
  } finally {
    await cleanup?.();
  }
}

export function unknownErrorMessage(error: unknown): string {
  return error instanceof Error ? error.message : JSON.stringify(error);
}

export function sleep(durationMs: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
