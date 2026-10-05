import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface LayoutProps {
  children: ReactNode;
  className?: string;
}

export function PageLayout({ children, className }: LayoutProps) {
  return (
    <section
      className={cn(
        'mx-auto flex min-h-full w-full max-w-7xl flex-col gap-5 px-4 py-4 sm:px-5 lg:px-6',
        className,
      )}
    >
      {children}
    </section>
  );
}

export function NarrowPageLayout({ children, className }: LayoutProps) {
  return (
    <section
      className={cn(
        'mx-auto flex min-h-full w-full max-w-4xl flex-col gap-4 px-4 py-4 sm:px-5',
        className,
      )}
    >
      {children}
    </section>
  );
}

export function FullHeightLayout({ children, className }: LayoutProps) {
  return (
    <section className={cn('flex min-h-full min-w-0 flex-col', className)}>
      {children}
    </section>
  );
}

interface PageHeaderProps {
  actions?: ReactNode;
  children?: ReactNode;
  className?: string;
  description?: ReactNode;
  title: ReactNode;
}

export function PageHeader({
  actions,
  children,
  className,
  description,
  title,
}: PageHeaderProps) {
  return (
    <div
      className={cn(
        'flex flex-col gap-4 md:flex-row md:items-start md:justify-between',
        className,
      )}
    >
      <div className="min-w-0">
        <h1 className="text-xl font-semibold tracking-tight sm:text-2xl">
          {title}
        </h1>
        {description ? (
          <p className="text-muted-foreground mt-1.5 max-w-3xl text-[13px] leading-5">
            {description}
          </p>
        ) : null}
        {children}
      </div>
      {actions ? (
        <div className="flex flex-wrap gap-2 md:justify-end">{actions}</div>
      ) : null}
    </div>
  );
}
