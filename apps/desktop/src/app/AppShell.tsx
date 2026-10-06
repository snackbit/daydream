import { useCallback, useEffect, useState } from 'react';
import { Link, Outlet, useRouterState } from '@tanstack/react-router';
import { PanelLeftClose, PanelLeftOpen, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import { AppearanceControl } from '@/features/appearance/components/appearance-control';
import { NotificationCenter } from '@/features/notifications/components/notification-center';
import { useBackgroundTaskSync } from '@/features/processing/use-background-task-sync';
import { RecorderStatusBar } from '@/features/recorder/components/recorder-status-bar';
import { useRecorderSession } from '@/features/recorder/use-recorder-session';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';
import { StartupWarningBanner } from '@/features/startup/components/startup-warning-banner';
import { navigationItems } from './navigation';

export function AppShell() {
  useBackgroundTaskSync();
  const recorderSession = useRecorderSession();
  const [compactViewport, setCompactViewport] = useState(
    () =>
      typeof window !== 'undefined' &&
      window.matchMedia('(max-width: 1199px)').matches,
  );
  const [navigationOverride, setNavigationOverride] = useState<
    'collapsed' | 'expanded' | null
  >(null);
  const debugEnabled = useDebugEnabled();
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });
  const visibleNavigationItems = navigationItems.filter(
    (item) => item.href !== '/diagnostics' || debugEnabled,
  );
  const compactNavigation =
    navigationOverride === null
      ? compactViewport
      : navigationOverride === 'collapsed';

  useEffect(() => {
    const mediaQuery = window.matchMedia('(max-width: 1199px)');
    const handleViewportChange = (event: MediaQueryListEvent) => {
      setCompactViewport(event.matches);
    };

    mediaQuery.addEventListener('change', handleViewportChange);

    return () => mediaQuery.removeEventListener('change', handleViewportChange);
  }, []);

  const handleToggleNavigation = useCallback(() => {
    setNavigationOverride(compactNavigation ? 'expanded' : 'collapsed');
  }, [compactNavigation]);

  return (
    <div
      className={
        compactNavigation
          ? 'bg-background grid h-dvh min-h-0 grid-cols-1 grid-rows-[minmax(0,1fr)_auto] md:grid-cols-[3rem_minmax(0,1fr)] md:grid-rows-1'
          : 'bg-background grid h-dvh min-h-0 grid-cols-1 grid-rows-[minmax(0,1fr)_auto] md:grid-cols-[13rem_minmax(0,1fr)] md:grid-rows-1'
      }
    >
      <div className="row-start-1 flex min-h-0 min-w-0 flex-col overflow-hidden md:col-start-2">
        <header className="bg-card/50 flex h-11 shrink-0 items-center justify-between gap-3 border-b px-3 sm:px-4">
          <div className="flex min-w-0 items-center gap-2">
            <Sparkles
              aria-hidden="true"
              className="text-muted-foreground size-3.5"
            />
            <span className="text-[13px] font-semibold tracking-tight">
              Daydream
            </span>
          </div>
          <div className="flex shrink-0 items-center gap-1.5">
            <AppearanceControl />
            <NotificationCenter />
          </div>
        </header>

        <StartupWarningBanner />

        <main className="min-h-0 flex-1 overflow-y-auto">
          <Outlet />
        </main>

        <footer className="text-muted-foreground flex min-h-10 shrink-0 items-center border-t px-3 text-[11px] sm:px-4">
          <RecorderStatusBar session={recorderSession} />
        </footer>
      </div>

      <aside className="bg-sidebar text-sidebar-foreground row-start-2 flex min-h-0 flex-col border-t md:col-start-1 md:row-start-1 md:border-t-0 md:border-r">
        <div
          className={
            compactNavigation
              ? 'hidden h-11 shrink-0 items-center justify-center border-b md:flex'
              : 'hidden h-11 shrink-0 items-center justify-between gap-2 border-b px-3 md:flex'
          }
        >
          {compactNavigation ? null : (
            <Link
              aria-label="Daydream home"
              className="flex min-w-0 items-center gap-2 text-[13px] font-semibold tracking-tight"
              to="/"
            >
              <Sparkles
                aria-hidden="true"
                className="text-muted-foreground size-3.5 shrink-0"
              />
              <span className="truncate">Daydream</span>
            </Link>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                aria-expanded={!compactNavigation}
                aria-label={
                  compactNavigation
                    ? 'Expand navigation'
                    : 'Collapse navigation'
                }
                className="shrink-0"
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
            </TooltipTrigger>
            <TooltipContent side="right">
              {compactNavigation ? 'Expand navigation' : 'Collapse navigation'}
            </TooltipContent>
          </Tooltip>
        </div>
        <nav
          aria-label="Primary navigation"
          className={
            compactNavigation
              ? 'flex min-w-0 items-stretch gap-0.5 overflow-x-auto p-1 md:flex-1 md:flex-col md:items-center md:gap-0.5 md:overflow-y-auto md:p-1'
              : 'flex min-w-0 items-stretch gap-0.5 overflow-x-auto p-1 md:flex-1 md:flex-col md:gap-0.5 md:overflow-y-auto md:p-2 md:pt-2'
          }
        >
          {visibleNavigationItems.map((item) => {
            const Icon = item.icon;
            const isActive =
              item.href === pathname || pathname.startsWith(`${item.href}/`);

            return (
              <Tooltip key={item.href}>
                <TooltipTrigger asChild>
                  <Button
                    asChild
                    className={
                      compactNavigation
                        ? 'h-10 min-w-0 flex-1 flex-col gap-0.5 px-1 text-[9px] aria-[current=page]:font-medium sm:text-[10px] md:size-8 md:flex-none md:justify-center md:px-0 md:text-xs'
                        : 'h-10 min-w-0 flex-1 flex-col gap-0.5 px-1 text-[9px] aria-[current=page]:font-medium sm:text-[10px] md:h-8 md:flex-none md:flex-row md:justify-start md:gap-2.5 md:px-2.5 md:text-xs'
                    }
                    variant="ghost"
                  >
                    <Link
                      aria-current={isActive ? 'page' : undefined}
                      aria-label={compactNavigation ? item.label : undefined}
                      className="aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground"
                      to={item.href}
                    >
                      <Icon aria-hidden="true" className="size-3.5 md:size-4" />
                      <span
                        className={
                          compactNavigation ? 'truncate md:sr-only' : 'truncate'
                        }
                      >
                        {item.label}
                      </span>
                    </Link>
                  </Button>
                </TooltipTrigger>
                {compactNavigation ? (
                  <TooltipContent side="right">{item.label}</TooltipContent>
                ) : null}
              </Tooltip>
            );
          })}
        </nav>

        <div className="hidden shrink-0 border-t px-4 py-2.5 text-[10px] leading-4 md:block">
          {compactNavigation ? null : (
            <>
              <div className="font-medium">Local storage</div>
              <p className="text-muted-foreground mt-0.5">
                Remote providers are identified in Settings.
              </p>
            </>
          )}
        </div>
      </aside>
    </div>
  );
}
