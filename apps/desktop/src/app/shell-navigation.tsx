import { Link } from '@tanstack/react-router';
import { Button } from '@/components/ui/button';
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip';
import type { NavigationItem } from './navigation';

export interface ShellNavigationProps {
  compact?: boolean;
  items: NavigationItem[];
  pathname: string;
  onNavigate?: () => void;
}

export function navigationItemIsActive(
  href: string,
  pathname: string,
): boolean {
  return (
    href === pathname ||
    (href !== '/' && pathname.startsWith(`${href}/`)) ||
    (href === '/library' && pathname.startsWith('/day/'))
  );
}

export function ShellNavigation({
  compact = false,
  items,
  pathname,
  onNavigate,
}: ShellNavigationProps) {
  return (
    <nav
      aria-label="Primary navigation"
      data-compact={compact}
      className="group/navigation flex min-h-0 flex-col gap-2 overflow-y-auto px-4 py-5 data-[compact=true]:items-center data-[compact=true]:px-2"
    >
      {items.map((item) => {
        const Icon = item.icon;
        const active = navigationItemIsActive(item.href, pathname);
        return (
          <Tooltip key={item.href}>
            <TooltipTrigger asChild>
              <Button
                asChild
                className="h-10 w-full justify-start gap-3 px-4 group-data-[compact=true]/navigation:size-10 group-data-[compact=true]/navigation:justify-center group-data-[compact=true]/navigation:px-0"
                variant="ghost"
              >
                <Link
                  aria-current={active ? 'page' : undefined}
                  aria-label={item.label}
                  className="aria-[current=page]:bg-sidebar-accent aria-[current=page]:text-sidebar-accent-foreground"
                  onClick={onNavigate}
                  to={item.href}
                >
                  <Icon aria-hidden="true" className="size-4" />
                  <span className="truncate group-data-[compact=true]/navigation:sr-only">
                    {item.label}
                  </span>
                </Link>
              </Button>
            </TooltipTrigger>
            {compact ? (
              <TooltipContent side="right">{item.label}</TooltipContent>
            ) : null}
          </Tooltip>
        );
      })}
    </nav>
  );
}
