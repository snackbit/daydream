import type { ProtectableWindow } from '@daydream/shared';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { settleVisualTransitions } from '@/app/design-accessibility-audit';
import '@/index.css';
import { ProtectedWindowPicker } from './protected-window-picker';

const native = vi.hoisted(() => ({
  listProtectableWindows: vi.fn(),
  getWindowProtectionStatus: vi.fn(),
}));
vi.mock('@/features/recorder/recorder-client', () => native);
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi
    .fn()
    .mockResolvedValue({ iconKey: 'window-icon:synthetic', pngBase64: null }),
}));
const longTitle =
  'Synthetic document for a long window title that wraps across two lines and retains its application identity';
const windows: ProtectableWindow[] = [
  {
    id: 'one',
    appIdentity: 'org.synthetic.Editor',
    processName: 'editor',
    processId: 100,
    title: longTitle,
    platform: 'linux',
    iconKey: 'window-icon:synthetic',
  },
  {
    id: 'two',
    appIdentity: 'org.synthetic.Terminal',
    processName: 'terminal',
    processId: 200,
    title: 'Synthetic terminal',
    platform: 'linux',
  },
  {
    id: 'three',
    appIdentity: null,
    processName: null,
    processId: null,
    title: null,
    platform: 'linux',
  },
];
beforeEach(() => {
  native.listProtectableWindows
    .mockReset()
    .mockResolvedValue({ provider: 'synthetic', status: 'available', windows });
  native.getWindowProtectionStatus.mockReset().mockResolvedValue({
    provider: 'synthetic',
    platform: 'linux',
    status: 'available',
    message: 'Available',
  });
});
afterEach(async () => {
  cleanup();
  await page.viewport(1440, 900);
});

test('window search keeps whole-row selection and legacy metadata usable', async () => {
  const onPick = vi.fn();
  render(<ProtectedWindowPicker onPick={onPick} onStatusLoaded={vi.fn()} />);
  await userEvent.click(screen.getByRole('button', { name: 'Pick window' }));
  await screen.findByRole('button', { name: /Synthetic terminal/ });
  expect(screen.getByRole('button', { name: /Unknown app/ })).toBeTruthy();
  await userEvent.fill(
    screen.getByRole('textbox', { name: 'Search windows' }),
    '200',
  );
  expect(
    screen.queryByRole('button', { name: new RegExp(longTitle) }),
  ).toBeNull();
  const row = screen.getByRole('button', { name: /Synthetic terminal/ });
  row.focus();
  await userEvent.keyboard('{Enter}');
  expect(onPick).toHaveBeenCalledWith(windows[1]);
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('picker keeps failures visible and refresh recovers without dropping selection', async () => {
  native.listProtectableWindows.mockRejectedValueOnce(
    new Error('synthetic private backend detail'),
  );
  render(<ProtectedWindowPicker onPick={vi.fn()} onStatusLoaded={vi.fn()} />);
  await userEvent.click(screen.getByRole('button', { name: 'Pick window' }));
  const alert = await screen.findByRole('alert');
  expect(alert.textContent).toContain('Try refreshing');
  expect(alert.textContent).not.toContain('backend detail');
  await userEvent.click(
    screen.getByRole('button', { name: 'Refresh windows' }),
  );
  await screen.findByRole('button', { name: /Synthetic terminal/ });
  await userEvent.fill(screen.getByRole('textbox'), 'not-present');
  expect(screen.getByRole('status').textContent).toContain('No windows match');
});

test.each(
  ['light', 'dark'].flatMap((mode) =>
    [1440, 1024, 768, 390, 320].map((width) => ({ mode, width })),
  ),
)(
  'picker long titles and controls remain reachable in $mode at $width pixels',
  async ({ mode, width }) => {
    document.documentElement.dataset.theme = mode;
    await page.viewport(width, 900);
    render(<ProtectedWindowPicker onPick={vi.fn()} onStatusLoaded={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Pick window' }));
    await userEvent.click(
      await screen.findByRole('button', { name: new RegExp(longTitle) }),
    );
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    await userEvent.click(screen.getByRole('button', { name: 'Pick window' }));
    await screen.findByRole('button', { name: /Synthetic terminal/ });
    await settleVisualTransitions();
    // Intentional visual regression for the approved two-line window title;
    // accessible row text still retains the complete selection metadata.
    const title = screen.getByText(longTitle);
    expect(title.getBoundingClientRect().height).toBeLessThanOrEqual(
      Number.parseFloat(getComputedStyle(title).lineHeight) * 2 + 1,
    );
    await page.screenshot({
      path: `__screenshots__/picker-${mode}-${width}.png`,
    });
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  },
);
