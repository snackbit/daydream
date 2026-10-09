import { useCallback, useEffect } from 'react';
import type { ReactNode } from 'react';
import type { AppStartupStage } from '@daydream/shared';
import {
  AlertCircle,
  CheckCircle2,
  Loader2,
  RotateCcw,
  Sparkles,
} from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { SettingsDangerZone } from '@/features/settings/components/settings-danger-zone';
import { cn } from '@/lib/utils';
import { useStartupStore } from '../startup-store';

interface AppStartupGateProps {
  children: ReactNode;
}

export function AppStartupGate({ children }: AppStartupGateProps) {
  const error = useStartupStore((state) => state.error);
  const initialize = useStartupStore((state) => state.initialize);
  const retry = useStartupStore((state) => state.retry);
  const status = useStartupStore((state) => state.status);

  const handleRetry = useCallback(() => {
    void retry();
  }, [retry]);

  useEffect(() => {
    void initialize();
  }, [initialize]);

  if (status?.phase === 'ready') {
    return <>{children}</>;
  }

  if (status?.phase === 'maintenance') {
    return (
      <main className="bg-background text-foreground min-h-screen p-6">
        <div className="mx-auto max-w-2xl">
          <h1 className="text-xl font-medium">Finish interrupted cleanup</h1>
          <p className="text-muted-foreground mt-2 text-sm">
            Daydream stopped normal startup because a previous cleanup is
            incomplete. Review its remaining impact and retry the same action.
            Recordings and background work stay blocked until it finishes.
          </p>
          <SettingsDangerZone />
        </div>
      </main>
    );
  }

  const stages = status?.stages ?? [];
  const completedStages = stages.filter((stage) =>
    ['ready', 'warning'].includes(stage.status),
  ).length;
  const progress = stages.length
    ? Math.round((completedStages / stages.length) * 100)
    : 15;
  const blockingError = status?.phase === 'error' ? status.error : null;

  return (
    <main className="bg-background text-foreground flex min-h-screen items-center justify-center p-4 sm:p-8">
      <Card className="shadow-primary/5 w-full max-w-xl shadow-lg">
        <CardHeader className="gap-2">
          <div className="mb-2 flex items-center gap-3">
            <span className="bg-primary text-primary-foreground flex size-10 items-center justify-center rounded-xl shadow-sm">
              <Sparkles aria-hidden="true" className="size-4" />
            </span>
            <div className="min-w-0">
              <div className="text-sm font-semibold tracking-tight">
                Daydream
              </div>
              <Badge className="mt-1" variant="outline">
                LOCAL WORKSPACE
              </Badge>
            </div>
          </div>
          <div aria-live="polite" className="flex items-center gap-2">
            {blockingError ? (
              <AlertCircle className="text-destructive size-5" />
            ) : (
              <Loader2 className="text-muted-foreground size-5 animate-spin" />
            )}
            <CardTitle aria-level={1} className="text-lg" role="heading">
              {blockingError
                ? 'Could not finish startup'
                : 'Preparing your workspace'}
            </CardTitle>
          </div>
          <p className="text-muted-foreground text-sm">
            Preparing local storage, capture services, and local processing
            tools.
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          {blockingError ? null : (
            <Progress
              aria-label="Workspace startup progress"
              value={progress}
            />
          )}

          {error ? (
            <Alert variant="destructive">
              <AlertTitle>Startup status unavailable</AlertTitle>
              <AlertDescription>{error}</AlertDescription>
            </Alert>
          ) : null}

          {blockingError ? (
            <Alert variant="destructive">
              <AlertTitle>{blockingError.code}</AlertTitle>
              <AlertDescription>{blockingError.message}</AlertDescription>
            </Alert>
          ) : null}

          <div className="space-y-2">
            {stages.map((stage) => (
              <StartupStageRow key={stage.id} stage={stage} />
            ))}
          </div>

          {blockingError ? (
            <div className="flex justify-end">
              <Button className="gap-2" onClick={handleRetry}>
                <RotateCcw className="size-4" />
                Retry
              </Button>
            </div>
          ) : null}
        </CardContent>
      </Card>
    </main>
  );
}

function StartupStageRow({ stage }: { stage: AppStartupStage }) {
  const loading = stage.status === 'loading';
  const warning = stage.status === 'warning';
  const failed = stage.status === 'error';

  return (
    <div className="flex items-start gap-3 rounded-md border p-3">
      <div className="mt-0.5">
        {loading ? (
          <Loader2 className="text-muted-foreground size-4 animate-spin" />
        ) : failed ? (
          <AlertCircle className="text-destructive size-4" />
        ) : (
          <CheckCircle2
            className={cn(
              'size-4',
              warning ? 'text-amber-600' : 'text-muted-foreground',
            )}
          />
        )}
      </div>
      <div className="min-w-0">
        <div className="text-sm font-medium">{stage.label}</div>
        <div className="text-muted-foreground text-xs leading-5">
          {stage.description}
        </div>
      </div>
    </div>
  );
}
