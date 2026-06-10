import { useEffect, useState } from 'react';
import { CheckCircle2, Clipboard, RefreshCw, XCircle } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import {
  DiagnosticRow,
  HealthBlock,
  MediaWriterCapabilityList,
} from '@/features/diagnostics/components/diagnostic-primitives';
import { getDebugLogBundle } from '@/features/diagnostics/diagnostics-client';
import { useDiagnosticsStore } from '@/features/diagnostics/diagnostics-store';
import { useAsyncErrorBoundary } from '@/lib/use-async-error-boundary';

export function DiagnosticsScreen() {
  const runAsyncAction = useAsyncErrorBoundary();
  const diagnostics = useDiagnosticsStore((state) => state.diagnostics);
  const error = useDiagnosticsStore((state) => state.error);
  const load = useDiagnosticsStore((state) => state.load);
  const status = useDiagnosticsStore((state) => state.status);
  const [copyStatus, setCopyStatus] = useState<'copied' | 'failed' | 'idle'>(
    'idle',
  );
  const [bundleCopyStatus, setBundleCopyStatus] = useState<
    'copied' | 'failed' | 'idle'
  >('idle');

  useEffect(() => {
    if (status === 'idle') {
      runAsyncAction(load);
    }
  }, [load, runAsyncAction, status]);

  async function copyRawDiagnosticsJson() {
    if (!diagnostics) {
      return;
    }

    try {
      await navigator.clipboard.writeText(JSON.stringify(diagnostics, null, 2));
      setCopyStatus('copied');
    } catch (_error) {
      setCopyStatus('failed');
    }
  }

  async function copyDebugLogBundle() {
    try {
      const bundle = await getDebugLogBundle();
      await navigator.clipboard.writeText(JSON.stringify(bundle, null, 2));
      setBundleCopyStatus('copied');
    } catch (_error) {
      setBundleCopyStatus('failed');
    }
  }

  return (
    <PageLayout>
      <PageHeader
        title="Diagnostics"
        description={
          <>
            Runtime facts, media writer health, storage paths, and native
            recorder capability probes. This screen is intentionally verbose for
            early development.
          </>
        }
        actions={
          <div className="flex flex-col items-end gap-2">
            <div className="flex gap-2">
              <Button
                className="gap-2"
                disabled={!diagnostics}
                onClick={() => void copyDebugLogBundle()}
                variant="outline"
              >
                <Clipboard className="size-3.5" />
                Copy Debug Bundle
              </Button>
              <Button
                className="gap-2"
                disabled={!diagnostics}
                onClick={() => void copyRawDiagnosticsJson()}
                variant="outline"
              >
                <Clipboard className="size-3.5" />
                Copy JSON
              </Button>
              <Button
                className="gap-2"
                onClick={() => runAsyncAction(load)}
                variant="outline"
              >
                <RefreshCw className="size-3.5" />
                Refresh
              </Button>
            </div>
            <p
              aria-live="polite"
              className="text-muted-foreground min-h-5 text-xs"
            >
              {copyStatus === 'copied' ? 'Diagnostics JSON copied.' : null}
              {copyStatus === 'failed'
                ? 'Unable to copy diagnostics JSON.'
                : null}
              {bundleCopyStatus === 'copied' ? 'Debug bundle copied.' : null}
              {bundleCopyStatus === 'failed'
                ? 'Unable to copy debug bundle.'
                : null}
            </p>
          </div>
        }
      />

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
              available={diagnostics?.recorder.gpuCapture.available ?? false}
              label="GPU capture"
            >
              <DiagnosticRow
                label="EGL"
                value={
                  diagnostics
                    ? formatBoolean(
                        diagnostics.recorder.gpuCapture.eglAvailable,
                      )
                    : null
                }
              />
              <DiagnosticRow
                label="EGL platform"
                value={diagnostics?.recorder.gpuCapture.eglPlatform ?? null}
              />
              <DiagnosticRow
                label="EGL vendor"
                value={diagnostics?.recorder.gpuCapture.eglVendor ?? null}
              />
              <DiagnosticRow
                label="EGL version"
                value={diagnostics?.recorder.gpuCapture.eglVersion ?? null}
              />
              <DiagnosticRow
                label="EGL APIs"
                value={
                  diagnostics?.recorder.gpuCapture.eglClientApis.join(', ') ??
                  null
                }
              />
              <DiagnosticRow
                label="OpenGL"
                value={
                  diagnostics
                    ? formatBoolean(
                        diagnostics.recorder.gpuCapture.openglAvailable,
                      )
                    : null
                }
              />
              <DiagnosticRow
                label="DMA-BUF"
                value={
                  diagnostics
                    ? formatBoolean(
                        diagnostics.recorder.gpuCapture.dmabufImportAvailable,
                      )
                    : null
                }
              />
              <DiagnosticRow
                label="Modifiers"
                value={
                  diagnostics
                    ? formatOptionalBoolean(
                        diagnostics.recorder.gpuCapture.modifierImportAvailable,
                      )
                    : null
                }
              />
              <DiagnosticRow
                label="Renderer"
                value={diagnostics?.recorder.gpuCapture.renderer ?? null}
              />
              <DiagnosticRow
                label="Vendor"
                value={diagnostics?.recorder.gpuCapture.vendor ?? null}
              />
              <DiagnosticRow
                label="Version"
                value={diagnostics?.recorder.gpuCapture.version ?? null}
              />
              <DiagnosticRow
                emptyValue="None"
                label="Cleanup warning"
                value={diagnostics?.recorder.gpuCapture.cleanupError ?? null}
              />
              <DiagnosticRow
                emptyValue="None"
                label="Error"
                value={diagnostics?.recorder.gpuCapture.error ?? null}
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
      </div>

      <div className="bg-card border p-4">
        <h2 className="text-sm font-medium">Raw diagnostics</h2>
        <pre className="bg-muted mt-3 max-h-96 overflow-auto p-3 text-xs leading-5">
          {JSON.stringify(diagnostics ?? { status }, null, 2)}
        </pre>
      </div>
    </PageLayout>
  );
}

function formatBoolean(value: boolean): string {
  return value ? 'Yes' : 'No';
}

function formatOptionalBoolean(value: boolean | null): string | null {
  return value === null ? null : formatBoolean(value);
}
