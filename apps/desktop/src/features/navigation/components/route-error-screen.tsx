import { useCallback } from 'react';
import { Link } from '@tanstack/react-router';
import { AlertCircle, RotateCcw } from 'lucide-react';
import { NarrowPageLayout, PageHeader } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { unknownErrorMessage } from '@/lib/async-store-action';

export interface RouteErrorScreenProps {
  error: unknown;
  reset: () => void;
}

export function RouteErrorScreen({ error, reset }: RouteErrorScreenProps) {
  const handleRetry = useCallback(() => {
    reset();
  }, [reset]);

  return (
    <NarrowPageLayout>
      <PageHeader
        title="This screen hit a problem"
        description="The last screen could not finish loading. You can try again or move to another part of Daydream."
      />

      <Alert className="p-4" variant="destructive">
        <AlertCircle aria-hidden="true" className="size-4" />
        <AlertTitle>Daydream could not open this screen</AlertTitle>
        <AlertDescription>
          Retry this screen, or choose another destination below.
        </AlertDescription>
      </Alert>

      <div className="flex flex-wrap gap-2">
        <Button className="gap-2" onClick={handleRetry}>
          <RotateCcw aria-hidden="true" className="size-3.5" />
          Try again
        </Button>
        <Button asChild variant="outline">
          <Link to="/library">Open Library</Link>
        </Button>
        <Button asChild variant="ghost">
          <Link to="/">Go home</Link>
        </Button>
      </div>

      <details className="text-muted-foreground bg-card rounded-lg border px-4 py-3 text-xs">
        <summary className="text-foreground cursor-pointer font-medium">
          Technical details
        </summary>
        <pre className="bg-muted mt-3 overflow-auto rounded-md p-3 text-xs leading-5">
          {unknownErrorMessage(error)}
        </pre>
      </details>
    </NarrowPageLayout>
  );
}
