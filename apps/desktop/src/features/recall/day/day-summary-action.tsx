import type { ReactNode } from 'react';
import { Link } from '@tanstack/react-router';
import type { RecallStatus } from '@daydream/shared';
import { RefreshCw } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useDaySummaryGeneration } from './use-day-summary-generation';

export interface DaySummaryActionProps {
  date: string;
  status: RecallStatus;
  evidenceCount: number;
  refresh: () => void;
  children?: ReactNode;
}
export function DaySummaryAction({
  date,
  status,
  evidenceCount,
  refresh,
  children,
}: DaySummaryActionProps) {
  const generation = useDaySummaryGeneration(date, status.jobs, refresh);
  return (
    <div className="mt-5 space-y-3">
      <div className="flex flex-wrap gap-3">
        {status.generationReady && evidenceCount > 0 ? (
          <Button
            aria-busy={generation.working}
            disabled={generation.working}
            onClick={
              generation.paused ? generation.resume : generation.generate
            }
            variant="outline"
          >
            <RefreshCw
              aria-hidden="true"
              className={
                generation.working
                  ? 'size-4 motion-safe:animate-spin'
                  : 'size-4'
              }
            />
            {generation.paused ? 'Resume summary' : 'Generate summary'}
          </Button>
        ) : null}
        {children}
      </div>
      <div className="min-h-10 text-sm">
        {generation.error ? (
          <p role="alert" className="text-destructive">
            {generation.error}
          </p>
        ) : (
          <p role="status" aria-live="polite" className="text-muted-foreground">
            {generation.message}
          </p>
        )}
        {!status.generationReady || generation.error ? (
          <Link
            to="/settings"
            search={{ tab: 'ai', focus: 'recall-generation' }}
            className="text-muted-foreground text-xs underline"
          >
            Summary settings
          </Link>
        ) : null}
      </div>
    </div>
  );
}
