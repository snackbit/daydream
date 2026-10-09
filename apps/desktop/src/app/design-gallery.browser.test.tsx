import {
  useCallback,
  useState,
  type AnchorHTMLAttributes,
  type ReactNode,
} from 'react';
import { cleanup, render, screen, waitFor } from '@testing-library/react';
import { Database, Video } from 'lucide-react';
import { afterEach, beforeEach, expect, test, vi } from 'vitest';
import { page, userEvent } from 'vitest/browser';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { Switch } from '@/components/ui/switch';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import { TooltipProvider } from '@/components/ui/tooltip';
import { useAppearanceStore } from '@/features/appearance/appearance-store';
import { AppearanceControl } from '@/features/appearance/components/appearance-control';
import '@/index.css';
import { PageHeader, PageLayout } from './PageLayout';
import {
  controlContrast,
  emulate200PercentDesktopZoom,
  focusContrast,
  resetZoomEmulation,
  settleVisualTransitions,
  textContrast,
} from './design-accessibility-audit';
import { ShellNavigation } from './shell-navigation';

vi.mock('@tanstack/react-router', () => ({
  Link: ({
    children,
    to,
    ...props
  }: AnchorHTMLAttributes<HTMLAnchorElement> & {
    children: ReactNode;
    to: string;
  }) => (
    <a href={to} {...props}>
      {children}
    </a>
  ),
}));

const items = [
  {
    href: '/library',
    label: 'Library',
    description: 'Review local recordings',
    icon: Database,
  },
  {
    href: '/capture',
    label: 'Capture',
    description: 'Record locally',
    icon: Video,
  },
];

function DesignGallery() {
  const [open, setOpen] = useState(false);
  const closeNavigation = useCallback(() => setOpen(false), []);
  return (
    <TooltipProvider>
      <PageLayout>
        <PageHeader
          title="Your day, remembered"
          description="A synthetic gallery for the shared desktop components."
          actions={<AppearanceControl />}
        />
        <Card>
          <CardHeader>
            <CardTitle>Recall workspace</CardTitle>
          </CardHeader>
          <CardContent className="flex flex-col gap-6">
            <div className="flex flex-wrap items-center gap-3">
              <Button>Primary action</Button>
              <Button variant="outline">Secondary action</Button>
              <Button disabled>Unavailable action</Button>
              <Badge variant="secondary">Local</Badge>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="gallery-search">Search your day</Label>
              <Input
                id="gallery-search"
                placeholder="Search OCR and transcripts"
              />
            </div>
            <div className="flex items-center gap-3">
              <Switch id="gallery-switch" />
              <Label htmlFor="gallery-switch">Include transcripts</Label>
            </div>
            <Tabs defaultValue="days">
              <TabsList>
                <TabsTrigger value="days">Days</TabsTrigger>
                <TabsTrigger value="recordings">Recordings</TabsTrigger>
              </TabsList>
              <TabsContent value="days">Day review</TabsContent>
              <TabsContent value="recordings">Recording review</TabsContent>
            </Tabs>
            <Sheet open={open} onOpenChange={setOpen}>
              <SheetTrigger asChild>
                <Button variant="outline">Open navigation</Button>
              </SheetTrigger>
              <SheetContent side="left">
                <SheetHeader>
                  <SheetTitle>Daydream navigation</SheetTitle>
                  <SheetDescription>
                    Navigate your local workspace
                  </SheetDescription>
                </SheetHeader>
                <ShellNavigation
                  items={items}
                  pathname="/library"
                  onNavigate={closeNavigation}
                />
              </SheetContent>
            </Sheet>
          </CardContent>
        </Card>
      </PageLayout>
    </TooltipProvider>
  );
}

beforeEach(() => {
  useAppearanceStore.setState({ mode: 'light', persistence: 'saved' });
  document.documentElement.dataset.theme = 'light';
});

afterEach(async () => {
  cleanup();
  document.body.removeAttribute('tabindex');
  localStorage.removeItem('daydream.appearance');
  await resetZoomEmulation();
  await page.viewport(1440, 960);
});

test.each([390, 320] as const)(
  '200 percent desktop zoom preserves keyboard access at %i CSS pixels',
  async (width) => {
    await page.viewport(width * 2, 900);
    await emulate200PercentDesktopZoom(width);
    render(<DesignGallery />);
    expect(window.devicePixelRatio).toBe(2);
    expect(window.innerWidth).toBe(width);
    for (const mode of ['light', 'dark'] as const) {
      useAppearanceStore.getState().setMode(mode);
      const targets = [
        screen.getByRole('combobox'),
        screen.getByRole('button', { name: 'Primary action' }),
        screen.getByRole('textbox'),
        screen.getByRole('switch'),
        screen.getByRole('button', { name: 'Open navigation' }),
      ];
      const visited = new Set<Element>();
      document.body.tabIndex = -1;
      document.body.focus();
      for (let step = 0; step < 12; step++) {
        await userEvent.keyboard('{Tab}');
        if (document.activeElement) visited.add(document.activeElement);
      }
      for (const target of targets)
        expect(
          visited.has(target),
          `${mode} zoom keyboard access to ${target.getAttribute('data-slot')}`,
        ).toBe(true);
      await userEvent.fill(
        screen.getByRole('textbox'),
        'Zoomed synthetic note',
      );
      await userEvent.click(
        screen.getByRole('button', { name: 'Open navigation' }),
      );
      expect(await screen.findByRole('dialog')).toBeTruthy();
      await userEvent.keyboard('{Escape}');
      await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
      window.scrollTo({ top: 0, left: 0 });
      await settleVisualTransitions();
      await page.screenshot({
        path: `__screenshots__/gallery/${mode}-zoom-200-${width}.png`,
      });
    }
    document.body.removeAttribute('tabindex');
  },
);

test('shared controls stay reachable in both themes and narrow viewports', async () => {
  render(<DesignGallery />);
  for (const mode of ['light', 'dark'] as const) {
    useAppearanceStore.getState().setMode(mode);
    for (const width of [1440, 1024, 768, 390, 320]) {
      await page.viewport(width, 900);
      await userEvent.click(screen.getByRole('tab', { name: 'Recordings' }));
      expect(screen.getByRole('tabpanel').textContent).toContain(
        'Recording review',
      );
      await userEvent.click(screen.getByRole('tab', { name: 'Days' }));
      const input = screen.getByRole('textbox', { name: 'Search your day' });
      await userEvent.fill(input, 'Synthetic note');
      expect(input).toHaveProperty('value', 'Synthetic note');
      expect(
        screen.getByRole('button', { name: 'Unavailable action' }),
      ).toHaveProperty('disabled', true);
      await page.screenshot({
        path: `__screenshots__/gallery/${mode}-${width}.png`,
      });
    }
  }
});

test('mobile navigation supports Escape, focus return, and closes on selection', async () => {
  await page.viewport(390, 900);
  render(<DesignGallery />);
  const trigger = screen.getByRole('button', { name: 'Open navigation' });
  await userEvent.click(trigger);
  expect(
    await screen.findByRole('dialog', { name: 'Daydream navigation' }),
  ).toBeTruthy();
  expect(
    screen.getByRole('link', { name: 'Library' }).getAttribute('aria-current'),
  ).toBe('page');
  await userEvent.keyboard('{Escape}');
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
  expect(document.activeElement).toBe(trigger);
  await userEvent.click(trigger);
  // Prevent the browser fixture from leaving the test runner; the real Link
  // receives the route and the shell's close action before navigation.
  screen
    .getByRole('link', { name: 'Capture' })
    .addEventListener('click', (event) => event.preventDefault(), {
      once: true,
    });
  await userEvent.click(screen.getByRole('link', { name: 'Capture' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
});

test('switch and appearance are operable by keyboard', async () => {
  render(<DesignGallery />);
  const toggle = screen.getByRole('switch', { name: 'Include transcripts' });
  toggle.focus();
  await userEvent.keyboard(' ');
  expect(toggle.getAttribute('aria-checked')).toBe('true');
  const appearance = screen.getByRole('combobox', { name: 'Color appearance' });
  appearance.focus();
  await userEvent.keyboard('{Enter}');
  await userEvent.click(await screen.findByRole('option', { name: 'Dark' }));
  expect(document.documentElement.dataset.theme).toBe('dark');
  expect(localStorage.getItem('daydream.appearance')).toBe('dark');
});

test('essential text and focused controls meet contrast requirements in both themes', async () => {
  render(<DesignGallery />);
  for (const mode of ['light', 'dark'] as const) {
    useAppearanceStore.getState().setMode(mode);
    const essentialText = [
      screen.getByRole('heading', { level: 1 }),
      screen.getByText(
        'A synthetic gallery for the shared desktop components.',
      ),
      screen.getByRole('button', { name: 'Primary action' }),
      screen.getByRole('button', { name: 'Secondary action' }),
      screen.getByRole('textbox'),
      screen.getByRole('tab', { name: 'Recordings' }),
      screen.getByRole('combobox'),
    ];
    await settleVisualTransitions();
    for (const element of essentialText)
      expect
        .soft(
          textContrast(element),
          `${mode}: ${element.textContent ?? element.getAttribute('aria-label')}`,
        )
        .toBeGreaterThanOrEqual(4.5);
    expect
      .soft(textContrast(screen.getByRole('textbox'), '::placeholder'))
      .toBeGreaterThanOrEqual(4.5);
    expect
      .soft(
        controlContrast(screen.getByRole('textbox'), 'borderColor'),
        `${mode} input boundary`,
      )
      .toBeGreaterThanOrEqual(3);
    expect
      .soft(
        controlContrast(screen.getByRole('switch'), 'backgroundColor'),
        `${mode} unchecked switch`,
      )
      .toBeGreaterThanOrEqual(3);
    expect
      .soft(
        controlContrast(
          screen.getByRole('button', { name: 'Secondary action' }),
          'borderColor',
        ),
        `${mode} secondary action boundary`,
      )
      .toBeGreaterThanOrEqual(3);
    expect
      .soft(
        controlContrast(
          screen.getByRole('tab', { name: 'Days' }),
          'borderColor',
        ),
        `${mode} selected tab boundary`,
      )
      .toBeGreaterThanOrEqual(3);
    await userEvent.keyboard('{Tab}');
    for (const element of [
      screen.getByRole('button', { name: 'Primary action' }),
      screen.getByRole('textbox'),
      screen.getByRole('switch'),
      screen.getByRole('combobox'),
    ]) {
      element.focus();
      expect
        .soft(
          focusContrast(element),
          `${mode} keyboard focus on ${element.getAttribute('data-slot')}`,
        )
        .toBeGreaterThanOrEqual(3);
    }
  }
});
