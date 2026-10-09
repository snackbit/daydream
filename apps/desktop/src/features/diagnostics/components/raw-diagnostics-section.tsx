import type { DaydreamDiagnostics } from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
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
    <SharedDisclosure label="Raw diagnostics JSON" className="bg-muted">
      <pre className="bg-background mt-4 max-h-72 overflow-auto rounded-lg p-4 text-xs leading-5">
        {JSON.stringify(diagnostics ?? { status }, null, 2)}
      </pre>
    </SharedDisclosure>
  );
}
