import type {
  AiModelTestInput,
  AiModelTestResult,
  ChatGptConnectionState,
} from '@daydream/shared';
import { reportFrontendFailure } from '@/lib/diagnostics/failure-logging';
import { invoke, decodeResponse } from '@/lib/diagnostics/invoke';

function record(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
const invalid = () =>
  Object.assign(new Error('The model service returned an invalid response.'), {
    code: 'model_response_invalid',
  });
export async function testConfiguredModel(
  input: AiModelTestInput,
): Promise<AiModelTestResult> {
  const result: unknown = await invoke('test_configured_model', { input });
  return decodeResponse(
    'test_configured_model',
    result,
    (result) => decodeModelTest(result, input.requestId),
    { requestId: input.requestId },
  );
}
function decodeModelTest(
  result: unknown,
  requestId: string,
): AiModelTestResult {
  if (
    !record(result) ||
    typeof result.fingerprint !== 'string' ||
    !Array.isArray(result.checks)
  )
    throw invalid();
  const checks: AiModelTestResult['checks'] = [];
  for (const check of result.checks) {
    if (
      !record(check) ||
      (check.capability !== 'text' &&
        check.capability !== 'vision' &&
        check.capability !== 'embedding') ||
      (check.status !== 'succeeded' && check.status !== 'failed') ||
      typeof check.message !== 'string' ||
      (check.errorCode !== undefined &&
        check.errorCode !== null &&
        typeof check.errorCode !== 'string') ||
      typeof check.elapsedMs !== 'number' ||
      !Number.isSafeInteger(check.elapsedMs) ||
      check.elapsedMs < 0 ||
      (check.dimensions !== null &&
        (typeof check.dimensions !== 'number' ||
          !Number.isSafeInteger(check.dimensions) ||
          check.dimensions < 1 ||
          check.dimensions > 8192))
    )
      throw invalid();
    checks.push({
      capability: check.capability,
      status: check.status,
      message: check.message,
      elapsedMs: check.elapsedMs,
      dimensions: check.dimensions,
      ...(check.errorCode === undefined ? {} : { errorCode: check.errorCode }),
    });
    if (check.status === 'failed')
      reportFrontendFailure(
        'test_configured_model',
        'execute',
        check.errorCode ?? null,
        { requestId, elapsedMs: check.elapsedMs, deduplicate: false },
      );
  }
  return { fingerprint: result.fingerprint, checks };
}
function account(value: unknown): ChatGptConnectionState {
  if (
    !record(value) ||
    typeof value.id !== 'string' ||
    typeof value.label !== 'string' ||
    (value.status !== 'connected' &&
      value.status !== 'signed-out' &&
      value.status !== 'permission-required' &&
      value.status !== 'reconnect-required') ||
    !Array.isArray(value.models)
  )
    throw invalid();
  const models: ChatGptConnectionState['models'] = [];
  for (const model of value.models) {
    if (
      !record(model) ||
      typeof model.slug !== 'string' ||
      typeof model.displayName !== 'string'
    )
      throw invalid();
    models.push({ slug: model.slug, displayName: model.displayName });
  }
  return { id: value.id, label: value.label, status: value.status, models };
}
export async function signInChatGpt(
  subscriptionId: string | null,
  requestId: string,
): Promise<ChatGptConnectionState> {
  return decodeResponse(
    'sign_in_chatgpt',
    await invoke('sign_in_chatgpt', { subscriptionId, requestId }),
    account,
    { requestId },
  );
}
export async function prepareChatGptConnection(): Promise<string> {
  return decodeResponse(
    'prepare_chatgpt_connection',
    await invoke('prepare_chatgpt_connection'),
    (id) => {
      if (typeof id !== 'string' || !/^[0-9a-f-]{36}$/i.test(id))
        throw invalid();
      return id;
    },
  );
}
export async function getChatGptConnection(
  subscriptionId: string,
): Promise<ChatGptConnectionState> {
  return decodeResponse(
    'get_chatgpt_connection',
    await invoke('get_chatgpt_connection', { subscriptionId }),
    account,
  );
}
export async function signOutChatGpt(subscriptionId: string): Promise<boolean> {
  return decodeResponse(
    'sign_out_chatgpt',
    await invoke('sign_out_chatgpt', { subscriptionId }),
    (result) => {
      if (typeof result !== 'boolean') throw invalid();
      if (!result)
        reportFrontendFailure(
          'sign_out_chatgpt',
          'execute',
          'model_subscription_request_failed',
          { deduplicate: false },
        );
      return result;
    },
  );
}
export async function cancelModelRequest(requestId: string): Promise<void> {
  await invoke('cancel_model_request', { requestId });
}
export function modelActionError(error: unknown): string {
  const code =
    typeof error === 'string'
      ? error
      : record(error) && typeof error.code === 'string'
        ? error.code
        : '';
  if (code === 'model_cancelled') return 'Cancelled.';
  if (code === 'model_response_invalid')
    return 'The model service returned an invalid response. Check Diagnostics and try again.';
  if (code === 'model_subscription_registration_missing')
    return 'ChatGPT did not complete registration. Start sign-in again.';
  if (code === 'model_subscription_registration_mismatch')
    return 'ChatGPT returned a different connection. Sign in with the account used for this connection.';
  if (code === 'model_subscription_identity_invalid')
    return 'ChatGPT identity could not be verified. Try signing in again.';
  if (code === 'model_subscription_credential_unavailable')
    return 'Could not access local credentials. Unlock your system keyring and try again.';
  if (code === 'model_subscription_storage_failed')
    return 'Could not save or read the ChatGPT connection locally. Check Diagnostics and try again.';
  if (code.includes('permission'))
    return 'Sign in again and allow direct model access.';
  if (code === 'model_subscription_usage_unavailable')
    return 'Subscription model access is unavailable. Check your ChatGPT plan and connected-app access.';
  if (code.includes('usage') || code === 'model_rate_limited')
    return 'The account’s usage limit was reached. Check ChatGPT Settings → Usage and try again later.';
  if (code.includes('reconnect') || code.includes('missing'))
    return 'Sign in again for this connection.';
  if (code.includes('timeout')) return 'The request timed out. Try again.';
  if (code.includes('denied'))
    return 'Sign-in was declined. You can try again.';
  return 'The model action failed. Check the connection and try again.';
}
