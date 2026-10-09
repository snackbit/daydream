import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import type * as NotificationsClientModule from '@/features/notifications/notifications-client';
import { SettingsDangerZone } from '../components/settings-danger-zone';
import { useSettingsStore } from '../settings-store';

const native = vi.hoisted(() => ({
  previewMaintenance: vi.fn(),
  executeMaintenance: vi.fn(),
  getMaintenanceStatus: vi.fn(),
  acknowledgeMaintenanceThemeReset: vi.fn(),
}));
const notifications = vi.hoisted(() => ({ getNotifications: vi.fn() }));
vi.mock(
  '@/features/notifications/notifications-client',
  async (importOriginal) => ({
    ...(await importOriginal<typeof NotificationsClientModule>()),
    ...notifications,
  }),
);
vi.mock('../maintenance-client', () => native);
beforeEach(() => {
  vi.resetAllMocks();
  notifications.getNotifications.mockResolvedValue({ notifications: [] });
  native.getMaintenanceStatus.mockResolvedValue(null);
  native.previewMaintenance.mockImplementation(async ({ action }) => ({
    action,
    token: 'synthetic-impact',
    recordingCount: 2,
    libraryRowCount: 10,
    modelCount: 1,
    fileCount: 3,
    sizeBytes: 1024,
  }));
  native.executeMaintenance.mockImplementation(async ({ action }) => ({
    action,
    operationId: 'synthetic-operation',
    phase: 'completed',
    error: null,
  }));
  vi.spyOn(useSettingsStore.getState(), 'load').mockResolvedValue();
});
afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});
test('clear library requires review and exact typed confirmation before native execution', async () => {
  render(<SettingsDangerZone />);
  await page
    .getByRole('button', { name: 'Clear library', exact: true })
    .click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  expect(native.executeMaintenance).not.toHaveBeenCalled();
  await expect
    .element(page.getByRole('button', { name: 'Confirm', exact: true }))
    .toBeDisabled();
  await page
    .getByRole('textbox', { name: 'Type CLEAR LIBRARY' })
    .fill('CLEAR LIBRARY');
  await page.getByRole('button', { name: 'Confirm', exact: true }).click();
  await waitFor(() =>
    expect(native.executeMaintenance).toHaveBeenCalledExactlyOnceWith({
      action: 'clear-library',
      previewToken: 'synthetic-impact',
      confirmation: 'CLEAR LIBRARY',
    }),
  );
  expect(native.acknowledgeMaintenanceThemeReset).not.toHaveBeenCalled();
  await screen.findByText('Clear library completed.');
});
test('failed cleanup stays visible and retries with a fresh reviewed impact', async () => {
  native.executeMaintenance.mockRejectedValue(
    'Synthetic cleanup failure. Retry cleanup.',
  );
  native.getMaintenanceStatus.mockResolvedValue({
    action: 'clear-library',
    operationId: 'synthetic-operation',
    phase: 'failed',
    error: 'synthetic',
  });
  render(<SettingsDangerZone />);
  await page.getByRole('button', { name: 'Retry clear library' }).click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page
    .getByRole('textbox', { name: 'Type CLEAR LIBRARY' })
    .fill('CLEAR LIBRARY');
  await page.getByRole('button', { name: 'Confirm', exact: true }).click();
  await waitFor(() =>
    expect(
      screen
        .getAllByRole('alert')
        .some((alert) =>
          alert.textContent?.includes('Synthetic cleanup failure'),
        ),
    ).toBe(true),
  );
  expect(native.previewMaintenance).toHaveBeenCalledOnce();
  expect(native.executeMaintenance).toHaveBeenCalledOnce();
});
test('factory reset acknowledges removal of persisted theme before native restart', async () => {
  window.localStorage.setItem('daydream.appearance', 'dark');
  native.executeMaintenance.mockResolvedValue({
    action: 'factory-reset',
    operationId: 'synthetic-operation',
    phase: 'awaiting-theme-reset',
    error: null,
  });
  native.acknowledgeMaintenanceThemeReset.mockImplementation(async () => {
    expect(window.localStorage.getItem('daydream.appearance')).toBeNull();
    return {
      action: 'factory-reset',
      operationId: 'synthetic-operation',
      phase: 'completed',
      error: null,
    };
  });
  render(<SettingsDangerZone />);
  await page
    .getByRole('button', { name: 'Factory reset', exact: true })
    .click();
  await page.getByRole('button', { name: 'Continue', exact: true }).click();
  await page
    .getByRole('textbox', { name: 'Type FACTORY RESET' })
    .fill('FACTORY RESET');
  await page.getByRole('button', { name: 'Confirm', exact: true }).click();
  await waitFor(() =>
    expect(
      native.acknowledgeMaintenanceThemeReset,
    ).toHaveBeenCalledExactlyOnceWith({ operationId: 'synthetic-operation' }),
  );
});
