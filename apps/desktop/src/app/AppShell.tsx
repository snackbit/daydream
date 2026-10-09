import { useCallback, useEffect, useState } from 'react';
import { Link, Outlet, useRouterState } from '@tanstack/react-router';
import { Menu, PanelLeftClose, PanelLeftOpen, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { AppearanceControl } from '@/features/appearance/components/appearance-control';
import { NotificationCenter } from '@/features/notifications/components/notification-center';
import { useBackgroundTaskSync } from '@/features/processing/use-background-task-sync';
import { RecorderStatusBar } from '@/features/recorder/components/recorder-status-bar';
import { RecordingSavedNavigation } from '@/features/recorder/components/recording-saved-navigation';
import { useRecorderSession } from '@/features/recorder/use-recorder-session';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';
import { StartupWarningBanner } from '@/features/startup/components/startup-warning-banner';
import { navigationItems } from './navigation';
import { ShellNavigation, navigationItemIsActive } from './shell-navigation';

export function AppShell() {
  useBackgroundTaskSync();
  const recorderSession = useRecorderSession();
  const [compactViewport, setCompactViewport] = useState(
    () => window.matchMedia('(max-width: 1199px)').matches,
  );
  const [navigationOverride, setNavigationOverride] = useState<boolean | null>(
    null,
  );
  const [mobileNavigationOpen, setMobileNavigationOpen] = useState(false);
  const debugEnabled = useDebugEnabled();
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const items = navigationItems.filter(
    (item) => item.href !== '/diagnostics' || debugEnabled,
  );
  const compactNavigation = navigationOverride ?? compactViewport;
  const context = items.find((item) =>
    navigationItemIsActive(item.href, pathname),
  );
  const contextLabel = pathname.startsWith('/library/')
    ? 'Studio'
    : pathname.startsWith('/day/')
      ? 'Day review'
      : pathname === '/notifications'
        ? 'Notifications'
        : (context?.label ?? 'Library');

  useEffect(() => {
    const query = window.matchMedia('(max-width: 1199px)');
    const desktopQuery = window.matchMedia('(min-width: 768px)');
    const handleViewportChange = (event: MediaQueryListEvent) =>
      setCompactViewport(event.matches);
    const handleDesktopViewport = (event: MediaQueryListEvent) => {
      if (event.matches) setMobileNavigationOpen(false);
    };
    query.addEventListener('change', handleViewportChange);
    desktopQuery.addEventListener('change', handleDesktopViewport);
    return () => {
      query.removeEventListener('change', handleViewportChange);
      desktopQuery.removeEventListener('change', handleDesktopViewport);
    };
  }, []);

  const handleToggleNavigation = useCallback(
    () => setNavigationOverride(!compactNavigation),
    [compactNavigation],
  );
  const handleCloseMobileNavigation = useCallback(
    () => setMobileNavigationOpen(false),
    [],
  );

  return (
    <div
      data-navigation={compactNavigation ? 'compact' : 'expanded'}
      className="group/shell bg-background grid h-dvh min-h-0 grid-cols-1 md:grid-cols-[216px_minmax(0,1fr)] md:data-[navigation=compact]:grid-cols-[72px_minmax(0,1fr)]"
    >
      <a
        href="#main-content"
        className="bg-background text-control fixed top-2 left-2 z-50 -translate-y-20 rounded-full border px-4 py-2 focus:translate-y-0"
      >
        Skip to content
      </a>
      <aside className="bg-sidebar text-sidebar-foreground hidden min-h-0 flex-col border-r md:flex">
        <div className="flex h-16 shrink-0 items-center justify-between gap-2 px-4 group-data-[navigation=compact]/shell:justify-center group-data-[navigation=compact]/shell:px-2">
          <Link
            aria-label="Daydream home"
            className="flex items-center gap-2 text-base font-medium group-data-[navigation=compact]/shell:hidden"
            to="/"
          >
            <Sparkles aria-hidden="true" className="size-5" />
            Daydream
          </Link>
          <Button
            aria-expanded={!compactNavigation}
            aria-label={
              compactNavigation ? 'Expand navigation' : 'Collapse navigation'
            }
            onClick={handleToggleNavigation}
            size="icon-sm"
            variant="ghost"
          >
            {compactNavigation ? (
              <PanelLeftOpen aria-hidden="true" />
            ) : (
              <PanelLeftClose aria-hidden="true" />
            )}
          </Button>
        </div>
        <ShellNavigation
          compact={compactNavigation}
          items={items}
          pathname={pathname}
        />
        <div className="text-muted-foreground text-metadata mt-auto px-6 py-6 leading-relaxed group-data-[navigation=compact]/shell:hidden">
          <p className="text-secondary-foreground font-medium">Local storage</p>
          <p className="mt-1">Remote providers are identified in Settings.</p>
        </div>
      </aside>
      <div className="flex min-h-0 min-w-0 flex-col overflow-hidden">
        <header className="flex h-16 shrink-0 items-center justify-between gap-3 border-b px-5 md:px-7 xl:px-10">
          <div className="flex min-w-0 items-center gap-3">
            <Sheet
              open={mobileNavigationOpen}
              onOpenChange={setMobileNavigationOpen}
            >
              <SheetTrigger asChild>
                <Button
                  aria-label="Open navigation"
                  className="md:hidden"
                  size="icon"
                  variant="ghost"
                >
                  <Menu aria-hidden="true" />
                </Button>
              </SheetTrigger>
              <SheetContent side="left" className="w-[min(320px,90vw)]">
                <SheetHeader>
                  <SheetTitle>Daydream</SheetTitle>
                  <SheetDescription>
                    Your local recall workspace
                  </SheetDescription>
                </SheetHeader>
                <ShellNavigation
                  items={items}
                  pathname={pathname}
                  onNavigate={handleCloseMobileNavigation}
                />
              </SheetContent>
            </Sheet>
            <span className="text-control truncate font-medium">
              {contextLabel}
            </span>
          </div>
          <div className="flex shrink-0 items-center gap-2">
            <AppearanceControl />
            <NotificationCenter />
          </div>
        </header>
        <div
          aria-label="Recording controls"
          role="region"
          className="bg-card/50 text-metadata flex min-h-14 shrink-0 items-center border-b px-5 py-3 md:px-7 xl:px-10"
        >
          <RecorderStatusBar session={recorderSession} />
        </div>
        <StartupWarningBanner />
        <RecordingSavedNavigation />
        <main
          id="main-content"
          tabIndex={-1}
          className="min-h-0 flex-1 overflow-y-auto outline-none"
        >
          <Outlet />
        </main>
      </div>
    </div>
  );
}
