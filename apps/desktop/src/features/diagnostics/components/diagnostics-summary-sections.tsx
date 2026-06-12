import type { DaydreamDiagnostics } from '@daydream/shared';
import { CheckCircle2, XCircle } from 'lucide-react';
import {
  DiagnosticRow,
  MediaWriterCapabilityList,
} from './diagnostic-primitives';

interface DiagnosticsSummarySectionsProps {
  diagnostics: DaydreamDiagnostics | null;
}

export function DiagnosticsSummarySections({
  diagnostics,
}: DiagnosticsSummarySectionsProps) {
  return (
    <>
      <div className="bg-card border p-4">
        <h2 className="text-sm font-medium">Application</h2>
        <dl className="mt-3 grid gap-2 text-sm">
          <DiagnosticRow label="Name" value={diagnostics?.app.name ?? null} />
          <DiagnosticRow
            label="Identifier"
            value={diagnostics?.app.identifier ?? null}
          />
          <DiagnosticRow
            label="Version"
            value={diagnostics?.app.version ?? null}
          />
        </dl>
      </div>

      <div className="bg-card border p-4">
        <h2 className="text-sm font-medium">System</h2>
        <dl className="mt-3 grid gap-2 text-sm">
          <DiagnosticRow
            label="Platform"
            value={diagnostics?.system.platform ?? null}
          />
          <DiagnosticRow
            label="Arch"
            value={diagnostics?.system.arch ?? null}
          />
          <DiagnosticRow
            label="Family"
            value={diagnostics?.system.family ?? null}
          />
        </dl>
      </div>

      <div className="bg-card border p-4">
        <h2 className="text-sm font-medium">Media writer</h2>
        <div className="mt-3 flex items-center gap-2 text-sm">
          {diagnostics?.mediaWriter.available ? (
            <CheckCircle2 className="text-primary size-4" />
          ) : (
            <XCircle className="text-destructive size-4" />
          )}
          <span>
            {diagnostics?.mediaWriter.available
              ? 'Available'
              : 'Not available yet'}
          </span>
        </div>
        <dl className="mt-3 grid gap-2 text-sm">
          <DiagnosticRow
            label="Writer"
            value={diagnostics?.mediaWriter.writer ?? null}
          />
          <DiagnosticRow
            label="Version"
            value={diagnostics?.mediaWriter.version ?? null}
          />
          <DiagnosticRow
            label="Error"
            emptyValue="None"
            value={diagnostics?.mediaWriter.error ?? null}
          />
        </dl>
        <MediaWriterCapabilityList
          capabilities={diagnostics?.mediaWriter.capabilities ?? null}
        />
      </div>

      <div className="bg-card border p-4">
        <h2 className="text-sm font-medium">Storage</h2>
        <dl className="mt-3 grid gap-2 text-sm">
          <DiagnosticRow
            label="App data"
            value={diagnostics?.storage.appDataDir ?? null}
          />
        </dl>
      </div>

      <div className="bg-card border p-4 md:col-span-2">
        <h2 className="text-sm font-medium">Debug logging</h2>
        <div className="mt-3 flex items-center gap-2 text-sm">
          {diagnostics?.debugLogging.enabled ? (
            <CheckCircle2 className="text-primary size-4" />
          ) : (
            <XCircle className="text-destructive size-4" />
          )}
          <span>
            {diagnostics?.debugLogging.enabled ? 'Enabled' : 'Not enabled'}
          </span>
        </div>
        <dl className="mt-3 grid gap-2 text-sm">
          <DiagnosticRow
            label="File"
            value={diagnostics?.debugLogging.filePath ?? null}
          />
          <DiagnosticRow
            label="Filter"
            value={diagnostics?.debugLogging.filter ?? null}
          />
          <DiagnosticRow
            label="Format"
            value={diagnostics?.debugLogging.format ?? null}
          />
          <DiagnosticRow
            label="Latest bytes"
            value={
              diagnostics?.debugLogging.latestFileBytes?.toString() ?? null
            }
          />
          <DiagnosticRow
            label="Retention days"
            value={diagnostics?.debugLogging.retentionDays.toString() ?? null}
          />
        </dl>
      </div>
    </>
  );
}
