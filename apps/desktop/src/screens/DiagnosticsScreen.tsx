import { Link } from '@tanstack/react-router';
import { AlertCircle, LoaderCircle, RotateCcw } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { DatabaseDiagnosticsSection } from '@/features/diagnostics/components/database-diagnostics-section';
import { DiagnosticsHeaderActions } from '@/features/diagnostics/components/diagnostics-header-actions';
import { DiagnosticsHealthSummary } from '@/features/diagnostics/components/diagnostics-health-summary';
import { DiagnosticsSummarySections } from '@/features/diagnostics/components/diagnostics-summary-sections';
import { PortalRestoreTokenDiagnosticsSection } from '@/features/diagnostics/components/portal-restore-token-diagnostics-section';
import { RawDiagnosticsSection } from '@/features/diagnostics/components/raw-diagnostics-section';
import { RecorderBackendDiagnosticsSection } from '@/features/diagnostics/components/recorder-backend-diagnostics-section';
import { useDiagnosticsScreenController } from '@/features/diagnostics/use-diagnostics-screen-controller';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';

export function DiagnosticsScreen() {
  const debugEnabled = useDebugEnabled();
  const diagnosticsController = useDiagnosticsScreenController(debugEnabled);
  const { diagnostics, error } = diagnosticsController.model;

  if (!debugEnabled) {
    return (
      <PageLayout>
        <PageHeader
          title="Diagnostics"
          description="Diagnostics are hidden until Debug is enabled."
        />
        <div className="bg-muted rounded-panel grid gap-4 p-6 text-sm">
          <p className="text-muted-foreground">
            Enable Debug in Settings to view runtime diagnostics.
          </p>
          <Button asChild className="w-fit" variant="outline">
            <Link to="/settings">Open Settings</Link>
          </Button>
        </div>
      </PageLayout>
    );
  }

  return (
    <PageLayout>
      <PageHeader
        title="Diagnostics"
        description="Runtime health and native recorder capability details for this device."
        actions={
          <DiagnosticsHeaderActions
            actions={diagnosticsController.actions}
            model={diagnosticsController.model}
          />
        }
      />

      {error ? (
        <Alert className="p-3" variant="destructive">
          <AlertCircle aria-hidden="true" className="size-3.5" />
          <AlertTitle>Could not refresh diagnostics</AlertTitle>
          <div className="flex flex-wrap items-start justify-between gap-2">
            <AlertDescription>{error}</AlertDescription>
            <Button
              onClick={diagnosticsController.actions.refresh}
              size="sm"
              variant="outline"
            >
              <RotateCcw aria-hidden="true" className="size-3" />
              Try again
            </Button>
          </div>
        </Alert>
      ) : null}

      {diagnosticsController.model.status === 'loading' && !diagnostics ? (
        <div
          aria-busy="true"
          className="text-muted-foreground bg-muted rounded-panel flex min-h-24 items-center gap-3 p-6 text-sm"
          role="status"
        >
          <LoaderCircle aria-hidden="true" className="size-3.5 animate-spin" />
          Loading device diagnostics…
        </div>
      ) : null}

      {diagnostics ? (
        <>
          <DiagnosticsHealthSummary diagnostics={diagnostics} />
          <div className="grid gap-6 xl:grid-cols-2">
            <DiagnosticsSummarySections diagnostics={diagnostics} />
            <RecorderBackendDiagnosticsSection diagnostics={diagnostics} />
            <DatabaseDiagnosticsSection diagnostics={diagnostics} />
            <PortalRestoreTokenDiagnosticsSection diagnostics={diagnostics} />
          </div>
        </>
      ) : null}

      {diagnostics ? (
        <RawDiagnosticsSection
          diagnostics={diagnostics}
          status={diagnosticsController.model.status}
        />
      ) : null}
    </PageLayout>
  );
}
