import { render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page } from 'vitest/browser';
import type * as NotificationsClient from '@/features/notifications/notifications-client';
import type * as RecorderClient from '@/features/recorder/recorder-client';
import '@/features/settings/testing/settings-workspace-harness';
import {
  setupWorkspace,
  cleanupWorkspace,
  configureAccounts,
  settings,
  saved,
  selectSetting,
} from '@/features/settings/testing/settings-workspace-harness';
import { SettingsScreen } from './SettingsScreen';

vi.mock(
  '@/features/settings/maintenance-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .maintenance,
);
vi.mock(
  '@/features/settings/settings-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .settings,
);
vi.mock('@/features/recorder/recorder-client', async (importOriginal) => ({
  ...(await importOriginal<typeof RecorderClient>()),
  ...(await import('@/features/settings/testing/settings-workspace-mocks'))
    .recorder,
}));
vi.mock(
  '@/features/ai/ai-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks')).ai,
);
vi.mock(
  '@/features/ai/models/model-transfer-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .transfer,
);
vi.mock(
  '@/features/recall/recall-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .recall,
);
vi.mock(
  '@/features/settings/models/model-client',
  async () =>
    (await import('@/features/settings/testing/settings-workspace-mocks'))
      .accounts,
);
vi.mock('@tanstack/react-router', async () => {
  const { blocker } =
    await import('@/features/settings/testing/settings-workspace-mocks');
  return { useBlocker: () => blocker };
});
vi.mock(
  '@/features/notifications/notifications-client',
  async (importOriginal) => ({
    ...(await importOriginal<typeof NotificationsClient>()),
    getNotifications: async () => ({ notifications: [] }),
  }),
);

beforeEach(setupWorkspace);
afterEach(cleanupWorkspace);
test.each([
  ['light', 1440],
  ['dark', 1440],
  ['light', 1024],
  ['dark', 1024],
  ['light', 390],
  ['dark', 390],
] as const)(
  'settings sections and editors stay reachable in %s at %i pixels',
  async (theme, width) => {
    await page.viewport(width, 960);
    document.documentElement.dataset.theme = theme;
    configureAccounts();
    const view = render(<SettingsScreen />);
    await screen.findByRole('switch', { name: 'Enable expert mode' });
    expect(screen.queryByRole('tablist')).toBeNull();
    expect(screen.queryByRole('button', { name: /^Save$/ })).toBeNull();
    for (const [id, label] of [
      ['ai', 'AI features'],
      ['models', 'Models & accounts'],
      ['privacy', 'Privacy'],
      ['recording', 'Recording'],
      ['storage', 'Storage'],
      ['general', 'General'],
    ] as const) {
      if (width >= 1024)
        await page
          .getByRole('navigation', { name: 'Settings sections' })
          .getByRole('button', { name: label, exact: true })
          .click();
      else await selectSetting('Settings section', label);
      await screen.findByRole('heading', { name: label });
      expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(width);
      for (const element of document.querySelectorAll(
        'button, input, [role="combobox"], [role="switch"]',
      )) {
        const bounds = element.getBoundingClientRect();
        if (!bounds.width) continue;
        expect(bounds.left).toBeGreaterThanOrEqual(0);
        expect(bounds.right).toBeLessThanOrEqual(width);
      }
      if (id === 'ai' || id === 'models' || id === 'privacy')
        await page.screenshot({
          path: `__screenshots__/settings-redesign/${id}-${theme}-${width}.png`,
        });
    }
    view.rerender(<SettingsScreen section="ai" />);
    await page
      .getByRole('button', { name: 'Configure Text recognition' })
      .click();
    const dialog = await screen.findByRole('dialog', {
      name: 'Text recognition',
    });
    const bounds = dialog.getBoundingClientRect();
    expect(Math.round(bounds.width)).toBe(width < 560 ? width : 560);
    await page.getByRole('combobox', { name: 'Recognition method' }).click();
    await page.getByRole('option', { name: 'Remote · Screen model' }).click();
    await page.screenshot({
      path: `__screenshots__/settings-redesign/editor-${theme}-${width}.png`,
    });
    await page.getByRole('button', { name: 'Cancel', exact: true }).click();
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(settings.saveAppSettings).not.toHaveBeenCalled();
    expect(saved.settings.ai.ocr.engine).toBe('tesseract');
  },
);
