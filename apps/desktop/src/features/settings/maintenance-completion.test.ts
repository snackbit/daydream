import { expect, test } from 'vitest';
import { maintenanceErrorMessage } from './maintenance-completion';

test('maintenance errors preserve native messages and withhold arbitrary object payloads', () => {
  const privateContent = 'SYNTHETIC-PRIVATE-PROVIDER-CONTENT';
  for (const error of [
    { message: privateContent },
    new Error(privateContent),
    [privateContent],
    null,
  ]) {
    const message = maintenanceErrorMessage(error);
    expect(message).not.toContain(privateContent);
    expect(message.length).toBeGreaterThan(0);
  }
  const native = 'Stop recording and wait for the writer to finish.';
  expect(maintenanceErrorMessage(native)).toBe(native);
});
