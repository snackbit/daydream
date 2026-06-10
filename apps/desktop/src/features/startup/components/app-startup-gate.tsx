import { useEffect } from 'react';
import type { ReactNode } from 'react';
import type { AppStartupStage } from '@daydream/shared';
import { AlertCircle, CheckCircle2, Loader2, RotateCcw } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
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

  useEffect(() => {
    void initialize();
  }, [initialize]);

  if (status?.phase === 'ready') {
    return <>{children}</>;
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
    <main className="bg-background text-foreground flex min-h-screen items-center justify-center p-6">
      <Card className="w-full max-w-xl">
        <CardHeader className="gap-2">
          <div className="flex items-center gap-2">
            {blockingError ? (
              <AlertCircle className="text-destructive size-5" />
            ) : (
              <Loader2 className="text-muted-foreground size-5 animate-spin" />
            )}
            <CardTitle className="text-lg">
              {blockingError ? 'Startup failed' : 'Starting Daydream'}
            </CardTitle>
          </div>
          <p className="text-muted-foreground text-sm">
            Preparing local storage, app services, and model-dependent tools.
          </p>
        </CardHeader>
        <CardContent className="space-y-4">
          {blockingError ? null : <Progress value={progress} />}

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
              <Button className="gap-2" onClick={() => void retry()}>
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
