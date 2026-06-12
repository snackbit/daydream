import type { DaydreamDiagnostics } from '@daydream/shared';
import { CheckCircle2, XCircle } from 'lucide-react';
import { DiagnosticRow } from './diagnostic-primitives';

interface DatabaseDiagnosticsSectionProps {
  diagnostics: DaydreamDiagnostics | null;
}

export function DatabaseDiagnosticsSection({
  diagnostics,
}: DatabaseDiagnosticsSectionProps) {
  return (
    <div className="bg-card border p-4 md:col-span-2">
      <h2 className="text-sm font-medium">Database</h2>
      <div className="mt-3 flex items-center gap-2 text-sm">
        {diagnostics?.database.available ? (
          <CheckCircle2 className="text-primary size-4" />
        ) : (
          <XCircle className="text-destructive size-4" />
        )}
        <span>
          {diagnostics?.database.available ? 'Available' : 'Not available'}
        </span>
      </div>
      <dl className="mt-3 grid gap-2 text-sm">
        <DiagnosticRow
          label="Path"
          value={diagnostics?.database.path ?? null}
        />
        <DiagnosticRow
          label="Schema"
          value={diagnostics?.database.schemaVersion?.toString() ?? null}
        />
        <DiagnosticRow
          label="Error"
          emptyValue="None"
          value={diagnostics?.database.error ?? null}
        />
      </dl>
      <div className="mt-4 grid gap-2 text-sm md:grid-cols-2">
        {Object.entries(diagnostics?.database.tableCounts ?? {}).map(
          ([table, count]) => (
            <div
              className="bg-muted grid grid-cols-[1fr_4rem] gap-3 p-2"
              key={table}
            >
              <span className="font-mono text-xs">{table}</span>
              <span className="text-right font-mono text-xs">{count}</span>
            </div>
          ),
        )}
      </div>
    </div>
  );
}
