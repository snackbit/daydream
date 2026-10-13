import { expect, test } from 'vitest';
import { summaryFailureMessage } from './use-day-summary-generation';

test.each([
  ['recall_provider_auth_failed', 'authenticate'],
  ['recall_credential_missing', 'authenticate'],
  ['recall_credential_unavailable', 'authenticate'],
  ['recall_provider_subscription_reconnect', 'authenticate'],
  ['recall_provider_network', 'unavailable'],
  ['recall_provider_timeout', 'unavailable'],
  ['recall_provider_server_failed', 'unavailable'],
  ['recall_provider_rate_limited', 'unavailable'],
  ['recall_input_changed', 'current sources'],
  ['recall_profile_changed', 'current sources'],
  ['recall_generation_disabled', 'setup'],
  ['recall_profile_invalid', 'setup'],
])(
  'summary failure %s provides recovery without exposing provider content',
  (code, guidance) => {
    expect(summaryFailureMessage(code)).toContain(guidance);
    expect(
      summaryFailureMessage({
        code,
        message: 'Synthetic private provider body',
      }),
    ).toContain(guidance);
    expect(
      summaryFailureMessage({
        code,
        message: 'Synthetic private provider body',
      }),
    ).not.toContain('Synthetic private');
  },
);

test.each([
  null,
  undefined,
  12,
  {},
  { code: null },
  new Error('Synthetic private provider body'),
  'Synthetic private provider body',
])('unknown summary failures remain safe: %s', (error) => {
  expect(summaryFailureMessage(error)).toContain('Try again');
  expect(summaryFailureMessage(error)).not.toContain('Synthetic private');
});
