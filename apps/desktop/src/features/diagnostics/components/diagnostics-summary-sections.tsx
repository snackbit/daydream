import type { DaydreamDiagnostics } from '@daydream/shared';
import { CheckCircle2, XCircle } from 'lucide-react';
import { DiagnosticCard, DiagnosticRow } from './diagnostic-primitives';
import { MediaWriterCapabilityList } from './media-writer-capability-list';

interface DiagnosticsSummarySectionsProps {
  diagnostics: DaydreamDiagnostics | null;
}

export function DiagnosticsSummarySections({
  diagnostics,
}: DiagnosticsSummarySectionsProps) {
  return (
    <>
      <DiagnosticCard title="Application">
        <dl className="grid gap-2 text-sm">
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
      </DiagnosticCard>

      <DiagnosticCard title="System">
        <dl className="grid gap-2 text-sm">
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
      </DiagnosticCard>

      <DiagnosticCard title="Media writer">
        <div className="flex items-center gap-2 text-sm">
          {diagnostics?.mediaWriter.available ? (
            <CheckCircle2 aria-hidden="true" className="text-primary size-4" />
          ) : (
            <XCircle aria-hidden="true" className="text-destructive size-4" />
          )}
          <span>
            {diagnostics?.mediaWriter.available
              ? 'Available'
              : 'Not available on this system'}
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
            emptyValue="None"
            label="Error"
            value={diagnostics?.mediaWriter.error ?? null}
          />
        </dl>
        <MediaWriterCapabilityList
          capabilities={diagnostics?.mediaWriter.capabilities ?? null}
        />
      </DiagnosticCard>

      <DiagnosticCard title="Storage">
        <dl className="grid gap-2 text-sm">
          <DiagnosticRow
            label="App data"
            value={diagnostics?.storage.appDataDir ?? null}
          />
        </dl>
      </DiagnosticCard>

      <DiagnosticCard className="md:col-span-2" title="Debug logging">
        <div className="flex items-center gap-2 text-sm">
          {diagnostics?.debugLogging.enabled ? (
            <CheckCircle2 aria-hidden="true" className="text-primary size-4" />
          ) : (
            <XCircle aria-hidden="true" className="text-destructive size-4" />
          )}
          <span>
            {diagnostics?.debugLogging.enabled ? 'Enabled' : 'Not enabled'}
          </span>
        </div>
        <dl className="mt-3 grid gap-2 text-sm md:grid-cols-2">
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
      </DiagnosticCard>
    </>
  );
}
