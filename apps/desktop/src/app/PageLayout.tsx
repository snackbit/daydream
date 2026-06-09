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
        'mx-auto flex w-full max-w-5xl flex-col gap-6 p-6',
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
        'mx-auto flex w-full max-w-3xl flex-col gap-3 p-6',
        className,
      )}
    >
      {children}
    </section>
  );
}

export function FullHeightLayout({ children, className }: LayoutProps) {
  return (
    <section className={cn('flex min-h-full flex-col', className)}>
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
        <h1 className="text-2xl font-semibold tracking-normal">{title}</h1>
        {description ? (
          <p className="text-muted-foreground mt-2 max-w-3xl text-sm leading-6">
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
