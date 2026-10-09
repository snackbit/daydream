import type { ReactNode } from 'react';
import { CheckCircle2, XCircle } from 'lucide-react';
import { cn } from '@/lib/utils';

export interface DiagnosticCardProps {
  children: ReactNode;
  className?: string;
  title: string;
}

export function DiagnosticCard({
  children,
  className,
  title,
}: DiagnosticCardProps) {
  return (
    <section className={cn('bg-muted rounded-panel min-w-0 p-6', className)}>
      <h2 className="text-sm font-medium">{title}</h2>
      <div className="mt-4">{children}</div>
    </section>
  );
}

export interface HealthBlockProps {
  available: boolean;
  children: ReactNode;
  label: string;
}

export function HealthBlock({ available, children, label }: HealthBlockProps) {
  return (
    <section
      className="bg-muted rounded-panel min-w-0 p-6"
      data-health={available ? 'available' : 'unavailable'}
    >
      <div className="flex items-center justify-between gap-2">
        <h3 className="flex min-w-0 items-center gap-2 text-sm font-medium">
          {available ? (
            <CheckCircle2
              aria-hidden="true"
              className="text-muted-foreground size-3.5 shrink-0"
            />
          ) : (
            <XCircle
              aria-hidden="true"
              className="text-destructive size-4 shrink-0"
            />
          )}
          {label}
        </h3>
        <span
          className={cn(
            'shrink-0 text-xs',
            available ? 'text-muted-foreground' : 'text-destructive',
          )}
        >
          {available ? 'Available' : 'Unavailable'}
        </span>
      </div>
      <dl className="mt-4 grid gap-3 text-sm">{children}</dl>
    </section>
  );
}

export interface DiagnosticRowProps {
  emptyValue?: string;
  label: string;
  value: string | null;
}

export function DiagnosticRow({
  emptyValue = 'Unavailable',
  label,
  value,
}: DiagnosticRowProps) {
  return (
    <div className="grid min-h-5 grid-cols-[minmax(6rem,8rem)_minmax(0,1fr)] items-baseline gap-2 sm:gap-3">
      <dt className="text-muted-foreground text-xs leading-5">{label}</dt>
      <dd className="min-w-0 font-mono text-xs leading-5 wrap-break-word">
        {value && value.length > 0 ? value : emptyValue}
      </dd>
    </div>
  );
}
