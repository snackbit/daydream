import { Link, Outlet, useRouterState } from '@tanstack/react-router';
import { Search } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { NotificationCenter } from '@/features/notifications/components/notification-center';
import { RecorderStatusBar } from '@/features/recorder/components/recorder-status-bar';
import { navigationItems } from './navigation';

const standaloneRouteHeaders = {
  '/notifications': {
    description: 'Persistent local status updates and action history.',
    label: 'Notifications',
  },
} as const;

export function AppShell() {
  const pathname = useRouterState({
    select: (state) => state.location.pathname,
  });

  const activeItem =
    navigationItems.find(
      (item) => item.href === pathname || pathname.startsWith(`${item.href}/`),
    ) ??
    standaloneRouteHeaders[pathname as keyof typeof standaloneRouteHeaders] ??
    navigationItems[0]!;

  return (
    <div className="bg-background text-foreground grid h-screen min-h-0 grid-cols-[14rem_1fr]">
      <aside className="bg-sidebar text-sidebar-foreground flex min-h-0 flex-col border-r">
        <div className="border-b px-4 py-4">
          <div className="text-sm font-semibold tracking-normal">Daydream</div>
          <div className="text-muted-foreground mt-1 text-xs">
            Local recall workspace
          </div>
        </div>

        <nav className="flex flex-1 flex-col gap-1 overflow-y-auto p-2">
          {navigationItems.map((item) => {
            const Icon = item.icon;
            const isActive = item.href === pathname;

            return (
              <Button
                asChild
                className="h-9 justify-start gap-2 px-2"
                key={item.href}
                variant={isActive ? 'secondary' : 'ghost'}
              >
                <Link to={item.href}>
                  <Icon className="size-4" />
                  <span>{item.label}</span>
                </Link>
              </Button>
            );
          })}
        </nav>
      </aside>

      <div className="flex min-h-0 flex-col overflow-auto">
        <header className="flex h-12 shrink-0 items-center justify-between border-b px-4">
          <div>
            <div className="text-sm font-medium">{activeItem.label}</div>
            <div className="text-muted-foreground text-xs">
              {activeItem.description}
            </div>
          </div>

          <div className="flex items-center gap-2">
            <NotificationCenter />
            <Button className="gap-2" disabled variant="outline">
              <Search className="size-3.5" />
              Search later
            </Button>
          </div>
        </header>

        <main className="min-h-0 flex-1 overflow-y-auto">
          <Outlet />
        </main>

        <footer className="text-muted-foreground flex h-8 shrink-0 items-center border-t px-4 text-xs">
          <RecorderStatusBar />
        </footer>
      </div>
    </div>
  );
}
