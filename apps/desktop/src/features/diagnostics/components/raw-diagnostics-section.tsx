import type { DaydreamDiagnostics } from '@daydream/shared';
import type { DiagnosticsScreenModel } from '../use-diagnostics-screen-controller';

interface RawDiagnosticsSectionProps {
  diagnostics: DaydreamDiagnostics | null;
  status: DiagnosticsScreenModel['status'];
}

export function RawDiagnosticsSection({
  diagnostics,
  status,
}: RawDiagnosticsSectionProps) {
  return (
    <div className="bg-card border p-4">
      <h2 className="text-sm font-medium">Raw diagnostics</h2>
      <pre className="bg-muted mt-3 max-h-96 overflow-auto p-3 text-xs leading-5">
        {JSON.stringify(diagnostics ?? { status }, null, 2)}
      </pre>
    </div>
  );
}
