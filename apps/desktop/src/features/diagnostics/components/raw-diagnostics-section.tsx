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
    <details className="border-y py-2">
      <summary className="cursor-pointer text-xs font-medium">
        Raw diagnostics JSON
      </summary>
      <pre className="bg-muted mt-2 max-h-72 overflow-auto p-2 text-[10px] leading-4">
        {JSON.stringify(diagnostics ?? { status }, null, 2)}
      </pre>
    </details>
  );
}
