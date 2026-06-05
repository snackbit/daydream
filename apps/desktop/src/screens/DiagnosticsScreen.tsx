import { useEffect, type ReactNode } from 'react';
import { CheckCircle2, RefreshCw, XCircle } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useDiagnosticsStore } from '@/features/diagnostics/diagnostics-store';

function formatValue(value: string | null, emptyValue: string): string {
  return value && value.length > 0 ? value : emptyValue;
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
    <section className="mx-auto flex max-w-5xl flex-col gap-6 p-6">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-semibold tracking-normal">
            Diagnostics
          </h1>
          <p className="text-muted-foreground mt-2 max-w-3xl text-sm leading-6">
            Runtime facts, FFmpeg location, storage paths, and native recorder
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
              emptyValue="None"
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

        <div className="bg-card border p-4 md:col-span-2">
          <h2 className="text-sm font-medium">Recorder Backend</h2>
          <div className="mt-3 grid gap-3 md:grid-cols-2">
            <HealthBlock
              available={diagnostics?.recorder.portal.available ?? false}
              label="ScreenCast portal"
            >
              <DiagnosticRow
                label="Version"
                value={
                  diagnostics?.recorder.portal.screenCastVersion?.toString() ??
                  null
                }
              />
              <DiagnosticRow
                label="Sources"
                value={
                  diagnostics?.recorder.portal.sourceTypes.join(', ') ?? null
                }
              />
              <DiagnosticRow
                label="Cursor"
                value={
                  diagnostics?.recorder.portal.cursorModes.join(', ') ?? null
                }
              />
              <DiagnosticRow
                emptyValue="None"
                label="Error"
                value={diagnostics?.recorder.portal.error ?? null}
              />
            </HealthBlock>

            <HealthBlock
              available={diagnostics?.recorder.pipewire.available ?? false}
              label="PipeWire"
            >
              <DiagnosticRow
                emptyValue="None"
                label="Error"
                value={diagnostics?.recorder.pipewire.error ?? null}
              />
            </HealthBlock>

            <HealthBlock
              available={diagnostics?.recorder.desktopAudio.available ?? false}
              label="Desktop audio"
            >
              <DiagnosticRow
                label="Default"
                value={
                  diagnostics?.recorder.desktopAudio.defaultSourceId ?? null
                }
              />
              <DiagnosticRow
                emptyValue="None"
                label="Error"
                value={diagnostics?.recorder.desktopAudio.error ?? null}
              />
            </HealthBlock>

            <HealthBlock
              available={diagnostics?.recorder.microphone.available ?? false}
              label="Microphone"
            >
              <DiagnosticRow
                label="Default"
                value={diagnostics?.recorder.microphone.defaultSourceId ?? null}
              />
              <DiagnosticRow
                emptyValue="None"
                label="Error"
                value={diagnostics?.recorder.microphone.error ?? null}
              />
            </HealthBlock>
          </div>
        </div>

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

interface HealthBlockProps {
  available: boolean;
  children: ReactNode;
  label: string;
}

function HealthBlock({ available, children, label }: HealthBlockProps) {
  return (
    <div className="bg-muted/40 border p-3">
      <div className="flex items-center gap-2 text-sm">
        {available ? (
          <CheckCircle2 className="text-primary size-4" />
        ) : (
          <XCircle className="text-destructive size-4" />
        )}
        <span>{label}</span>
      </div>
      <dl className="mt-3 grid gap-2 text-sm">{children}</dl>
    </div>
  );
}

interface DiagnosticRowProps {
  emptyValue?: string;
  label: string;
  value: string | null;
}

function DiagnosticRow({
  emptyValue = 'Unavailable',
  label,
  value,
}: DiagnosticRowProps) {
  return (
    <div className="grid min-h-5 grid-cols-[8rem_1fr] items-baseline gap-3">
      <dt className="text-muted-foreground text-sm leading-5">{label}</dt>
      <dd className="min-w-0 font-mono text-xs leading-5 wrap-break-word">
        {formatValue(value, emptyValue)}
      </dd>
    </div>
  );
}
