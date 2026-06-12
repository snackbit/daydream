import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { DatabaseDiagnosticsSection } from '@/features/diagnostics/components/database-diagnostics-section';
import { DiagnosticsHeaderActions } from '@/features/diagnostics/components/diagnostics-header-actions';
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
        <div className="grid gap-3 border p-4 text-sm">
          <p className="text-muted-foreground">
            Enable Debug in Settings to view runtime diagnostics.
          </p>
          <Button asChild className="w-fit" variant="outline">
            <a href="/settings">Open Settings</a>
          </Button>
        </div>
      </PageLayout>
    );
  }

  return (
    <PageLayout>
      <PageHeader
        title="Diagnostics"
        description={
          <>
            Runtime facts, media writer health, storage paths, and native
            recorder capability probes. This screen is intentionally verbose for
            early development.
          </>
        }
        actions={
          <DiagnosticsHeaderActions
            actions={diagnosticsController.actions}
            model={diagnosticsController.model}
          />
        }
      />

      {error ? (
        <div className="border-destructive/40 bg-destructive/10 text-destructive border p-4 text-sm">
          {error}
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-2">
        <DiagnosticsSummarySections diagnostics={diagnostics} />
        <RecorderBackendDiagnosticsSection diagnostics={diagnostics} />
        <DatabaseDiagnosticsSection diagnostics={diagnostics} />
        <PortalRestoreTokenDiagnosticsSection diagnostics={diagnostics} />
      </div>

      <RawDiagnosticsSection
        diagnostics={diagnostics}
        status={diagnosticsController.model.status}
      />
    </PageLayout>
  );
}
