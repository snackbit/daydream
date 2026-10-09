import type { DaydreamDiagnostics } from '@daydream/shared';
import { CheckCircle2, CircleAlert } from 'lucide-react';

interface DiagnosticsHealthSummaryProps {
  diagnostics: DaydreamDiagnostics;
}

export function DiagnosticsHealthSummary({
  diagnostics,
}: DiagnosticsHealthSummaryProps) {
  const checks = [
    {
      available: diagnostics.recorder.portal.available,
      label: 'Screen capture',
    },
    { available: diagnostics.mediaWriter.available, label: 'Media writer' },
    { available: diagnostics.database.available, label: 'Database' },
    {
      available: diagnostics.recorder.desktopAudio.available,
      label: 'System audio',
    },
    {
      available: diagnostics.recorder.microphone.available,
      label: 'Microphone',
    },
  ];
  const availableCount = checks.filter((check) => check.available).length;
  const unavailableCount = checks.length - availableCount;

  return (
    <section
      aria-labelledby="runtime-overview-heading"
      className="bg-muted rounded-panel grid gap-4 p-6"
    >
      <div className="flex flex-wrap items-baseline justify-between gap-3">
        <h2 className="text-sm font-medium" id="runtime-overview-heading">
          Runtime overview
        </h2>
        <p aria-live="polite" className="text-muted-foreground text-xs">
          {unavailableCount === 0
            ? `${availableCount} checks available`
            : `${unavailableCount} of ${checks.length} checks unavailable`}
        </p>
      </div>
      <ul className="grid gap-3 sm:grid-cols-2 xl:grid-cols-5">
        {checks.map((check) => (
          <li
            className="flex min-w-0 items-center gap-2 py-2"
            key={check.label}
          >
            {check.available ? (
              <CheckCircle2
                aria-hidden="true"
                className="text-muted-foreground size-3 shrink-0"
              />
            ) : (
              <CircleAlert
                aria-hidden="true"
                className="text-destructive size-3 shrink-0"
              />
            )}
            <span className="text-xs">{check.label}</span>
            <span className="sr-only">
              {check.available ? 'Available' : 'Unavailable'}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}
