import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  RouterProvider,
} from '@tanstack/react-router';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useAppearanceStore } from '@/features/appearance/appearance-store';
import { useNotificationsStore } from '@/features/notifications/notifications-store';
import { useSettingsStore } from '@/features/settings/settings-store';
import '@/index.css';
import { AppShell } from './AppShell';
import {
  emulate200PercentDesktopZoom,
  focusContrast,
  resetZoomEmulation,
  settleVisualTransitions,
  textContrast,
} from './design-accessibility-audit';

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn().mockResolvedValue(() => {}),
}));
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(async (command: string) => {
    if (command === 'listBackgroundTasks')
      return { tasks: [], nextCursor: null };
    if (command === 'listAiModels') return { models: [] };
    if (command === 'getRecorderPerformanceStats')
      return {
        state: 'idle',
        desiredFps: null,
        segmentStartedAtMs: null,
        timing: null,
        updatedAtMs: 0,
      };
    throw new Error('Synthetic native command unavailable');
  }),
}));

function LibraryFixture() {
  return <h1>Local library fixture</h1>;
}
function CaptureFixture() {
  return <h1>Capture fixture</h1>;
}

beforeEach(() => {
  useSettingsStore.setState({ loadStatus: 'ready' });
  useNotificationsStore.setState({
    status: 'ready',
    notifications: [],
    error: null,
  });
});

afterEach(async () => {
  cleanup();
  document.body.removeAttribute('tabindex');
  await resetZoomEmulation();
  await page.viewport(1440, 960);
});

async function renderShell() {
  const root = createRootRoute({ component: AppShell });
  const library = createRoute({
    getParentRoute: () => root,
    path: '/library',
    component: LibraryFixture,
  });
  const capture = createRoute({
    getParentRoute: () => root,
    path: '/capture',
    component: CaptureFixture,
  });
  const router = createRouter({
    routeTree: root.addChildren([library, capture]),
    history: createMemoryHistory({ initialEntries: ['/library'] }),
  });
  await router.load();
  render(
    <TooltipProvider>
      <RouterProvider router={router} />
    </TooltipProvider>,
  );
  await screen.findByRole('heading', { name: 'Local library fixture' });
}

test.each([390, 320] as const)(
  '200 percent zoom retains accessible shell controls at %i CSS pixels',
  async (width) => {
    await page.viewport(width * 2, 900);
    await emulate200PercentDesktopZoom(width);
    await renderShell();
    for (const mode of ['light', 'dark'] as const) {
      useAppearanceStore.getState().setMode(mode);
      const menu = screen.getByRole('button', { name: 'Open navigation' });
      const appearance = screen.getByRole('combobox', {
        name: 'Color appearance',
      });
      await settleVisualTransitions();
      const visited = new Set<Element>();
      document.body.tabIndex = -1;
      document.body.focus();
      for (let step = 0; step < 8; step++) {
        await userEvent.keyboard('{Tab}');
        if (document.activeElement) visited.add(document.activeElement);
      }
      expect(visited.has(menu)).toBe(true);
      expect(visited.has(appearance)).toBe(true);
      appearance.focus();
      expect(
        focusContrast(appearance),
        `${mode} shell appearance focus`,
      ).toBeGreaterThanOrEqual(3);
      expect(
        textContrast(appearance),
        `${mode} shell appearance text`,
      ).toBeGreaterThanOrEqual(4.5);
      expect(
        textContrast(screen.getByText('Recorder idle')),
        `${mode} recording status text`,
      ).toBeGreaterThanOrEqual(4.5);
      menu.focus();
      await userEvent.keyboard('{Enter}');
      expect(
        await screen.findByRole('dialog', { name: 'Daydream' }),
      ).toBeTruthy();
      await settleVisualTransitions();
      const link = screen.getByRole('link', { name: 'Library' });
      expect(
        textContrast(link),
        `${mode} navigation text`,
      ).toBeGreaterThanOrEqual(4.5);
      link.focus();
      expect(
        focusContrast(link),
        `${mode} navigation keyboard focus`,
      ).toBeGreaterThanOrEqual(3);
      await userEvent.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
      await page.screenshot({
        path: `__screenshots__/gallery/shell-${mode}-zoom-200-${width}.png`,
      });
    }
    document.body.removeAttribute('tabindex');
  },
);

test('mobile shell keeps appearance and recording status available across navigation', async () => {
  await page.viewport(320, 900);
  await renderShell();
  expect(
    screen.getByRole('region', { name: 'Recording controls' }),
  ).toBeTruthy();
  expect(
    screen.getByRole('combobox', { name: 'Color appearance' }),
  ).toBeTruthy();
  const menu = screen.getByRole('button', { name: 'Open navigation' });
  await userEvent.click(menu);
  expect(await screen.findByRole('dialog', { name: 'Daydream' })).toBeTruthy();
  await userEvent.click(screen.getByRole('link', { name: 'Capture' }));
  expect(
    await screen.findByRole('heading', { name: 'Capture fixture' }),
  ).toBeTruthy();
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(
    screen.getByRole('region', { name: 'Recording controls' }),
  ).toBeTruthy();
});

test('shell closes mobile navigation when the viewport switches to desktop', async () => {
  await page.viewport(390, 900);
  await renderShell();
  await userEvent.click(
    screen.getByRole('button', { name: 'Open navigation' }),
  );
  await screen.findByRole('dialog', { name: 'Daydream' });
  await page.viewport(1024, 900);
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  const expand = screen.getByRole('button', { name: 'Expand navigation' });
  await userEvent.click(expand);
  expect(
    screen
      .getByRole('button', { name: 'Collapse navigation' })
      .getAttribute('aria-expanded'),
  ).toBe('true');
  expect(
    screen.getByRole('region', { name: 'Recording controls' }),
  ).toBeTruthy();
});
