import type { ReactNode } from 'react';
import { cn } from '@/lib/utils';

export interface LayoutProps {
  children: ReactNode;
  className?: string;
}

export function PageLayout({ children, className }: LayoutProps) {
  return (
    <section
      className={cn(
        'mx-auto flex min-h-full w-full max-w-[1280px] flex-col gap-8 px-5 py-8 md:px-7 xl:px-10',
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
        'mx-auto flex min-h-full w-full max-w-[1280px] flex-col gap-8 px-5 py-8 md:px-7 xl:px-10',
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

export interface PageHeaderProps {
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
        <h1 className="text-heading-sm sm:text-heading font-light tracking-[-0.02em]">
          {title}
        </h1>
        {description ? (
          <p className="text-muted-foreground text-body mt-3 max-w-3xl leading-relaxed">
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
