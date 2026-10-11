import {
  invoke as nativeInvoke,
  type InvokeArgs,
  type InvokeOptions,
} from '@tauri-apps/api/core';
import { reportFrontendFailure } from './failure-logging';

export async function invoke<T>(
  command: string,
  ...args: [args?: InvokeArgs, options?: InvokeOptions]
): Promise<T> {
  const started = performance.now();
  try {
    return await nativeInvoke<T>(command, ...args);
  } catch (error) {
    const requestId = requestIdentifier(args[0]);
    reportFrontendFailure(command, 'invoke', error, {
      ...(requestId === undefined ? {} : { requestId }),
      elapsedMs: performance.now() - started,
      deduplicate: false,
    });
    throw error;
  }
}

export function decodeResponse<T, Value>(
  command: string,
  value: Value,
  decode: (value: Value) => T,
  options: { requestId?: string } = {},
): T {
  try {
    return decode(value);
  } catch (error) {
    reportFrontendFailure(command, 'decode', error, {
      ...options,
      deduplicate: false,
    });
    throw error;
  }
}

function requestIdentifier(
  argument: InvokeArgs | undefined,
): string | undefined {
  try {
    const input =
      typeof argument === 'object' && argument !== null && 'input' in argument
        ? argument.input
        : argument;
    return typeof input === 'object' &&
      input !== null &&
      'requestId' in input &&
      typeof input.requestId === 'string'
      ? input.requestId
      : undefined;
  } catch {
    return undefined;
  }
}
