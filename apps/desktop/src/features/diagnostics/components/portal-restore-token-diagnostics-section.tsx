import type { DaydreamDiagnostics } from '@daydream/shared';
import { CheckCircle2, XCircle } from 'lucide-react';
import { DiagnosticRow } from './diagnostic-primitives';

interface PortalRestoreTokenDiagnosticsSectionProps {
  diagnostics: DaydreamDiagnostics | null;
}

export function PortalRestoreTokenDiagnosticsSection({
  diagnostics,
}: PortalRestoreTokenDiagnosticsSectionProps) {
  return (
    <div className="bg-card border p-4 md:col-span-2">
      <h2 className="text-sm font-medium">Portal restore token</h2>
      <div className="mt-3 flex items-center gap-2 text-sm">
        {diagnostics?.portalRestoreToken.present ? (
          <CheckCircle2 className="text-primary size-4" />
        ) : (
          <XCircle className="text-destructive size-4" />
        )}
        <span>
          {diagnostics?.portalRestoreToken.present ? 'Saved' : 'Missing'}
        </span>
      </div>
      <dl className="mt-3 grid gap-2 text-sm">
        <DiagnosticRow
          label="Expected ID"
          value={diagnostics?.portalRestoreToken.expectedId ?? null}
        />
        <DiagnosticRow
          label="Portal"
          value={diagnostics?.portalRestoreToken.portal ?? null}
        />
        <DiagnosticRow
          label="Token length"
          value={
            diagnostics?.portalRestoreToken.tokenLength?.toString() ?? null
          }
        />
        <DiagnosticRow
          label="Created"
          value={
            diagnostics?.portalRestoreToken.createdAtMs?.toString() ?? null
          }
        />
        <DiagnosticRow
          label="Updated"
          value={
            diagnostics?.portalRestoreToken.updatedAtMs?.toString() ?? null
          }
        />
        <DiagnosticRow
          label="Last used"
          value={
            diagnostics?.portalRestoreToken.lastUsedAtMs?.toString() ?? null
          }
        />
        <DiagnosticRow
          emptyValue="None"
          label="Error"
          value={diagnostics?.portalRestoreToken.error ?? null}
        />
      </dl>
    </div>
  );
}
