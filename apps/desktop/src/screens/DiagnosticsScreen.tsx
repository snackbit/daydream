import { useEffect } from 'react';
import { CheckCircle2, RefreshCw, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useDiagnosticsStore } from '@/features/diagnostics/diagnostics-store';

function formatValue(value: string | null): string {
  return value && value.length > 0 ? value : 'Unavailable';
}

export function DiagnosticsScreen() {
  const diagnostics = useDiagnosticsStore((state) => state.diagnostics);
  const error = useDiagnosticsStore((state) => state.error);
  const load = useDiagnosticsStore((state) => state.load);
  const status = useDiagnosticsStore((state) => state.status);

  useEffect(() => {
    if (status === 'idle') {
      void load();
    }
  }, [load, status]);

  return (
    <section className="mx-auto flex max-w-6xl flex-col gap-5 p-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-normal">
            Diagnostics
          </h1>
          <p className="text-muted-foreground mt-2 max-w-3xl text-sm leading-6">
            Runtime facts, FFmpeg location, storage paths, and future native
            capability probes. This screen is intentionally verbose for early
            development.
          </p>
        </div>

        <Button className="gap-2" onClick={() => void load()} variant="outline">
          <RefreshCw className="size-3.5" />
          Refresh
        </Button>
      </div>

      {error ? (
        <div className="border-destructive/40 bg-destructive/10 text-destructive border p-4 text-sm">
          {error}
        </div>
      ) : null}

      <div className="grid gap-3 md:grid-cols-2">
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
          <h2 className="text-sm font-medium">FFmpeg</h2>
          <div className="mt-3 flex items-center gap-2 text-sm">
            {diagnostics?.ffmpeg.available ? (
              <CheckCircle2 className="text-primary size-4" />
            ) : (
              <XCircle className="text-destructive size-4" />
            )}
            <span>
              {diagnostics?.ffmpeg.available
                ? 'Available'
                : 'Not available yet'}
            </span>
          </div>
          <dl className="mt-3 grid gap-2 text-sm">
            <DiagnosticRow
              label="Path"
              value={diagnostics?.ffmpeg.path ?? null}
            />
            <DiagnosticRow
              label="Source"
              value={diagnostics?.ffmpeg.source ?? null}
            />
            <DiagnosticRow
              label="Version"
              value={diagnostics?.ffmpeg.versionLine ?? null}
            />
            <DiagnosticRow
              label="Error"
              value={diagnostics?.ffmpeg.error ?? null}
            />
          </dl>
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
      </div>

      <div className="bg-card border p-4">
        <h2 className="text-sm font-medium">Raw diagnostics</h2>
        <pre className="bg-muted mt-3 max-h-96 overflow-auto p-3 text-xs leading-5">
          {JSON.stringify(diagnostics ?? { status }, null, 2)}
        </pre>
      </div>
    </section>
  );
}

interface DiagnosticRowProps {
  label: string;
  value: string | null;
}

function DiagnosticRow({ label, value }: DiagnosticRowProps) {
  return (
    <div className="grid grid-cols-[8rem_1fr] gap-3">
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="min-w-0 font-mono text-xs break-words">
        {formatValue(value)}
      </dd>
    </div>
  );
}
