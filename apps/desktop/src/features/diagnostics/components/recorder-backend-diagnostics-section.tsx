import type { DaydreamDiagnostics } from '@daydream/shared';
import { DiagnosticRow, HealthBlock } from './diagnostic-primitives';

interface RecorderBackendDiagnosticsSectionProps {
  diagnostics: DaydreamDiagnostics | null;
}

export function RecorderBackendDiagnosticsSection({
  diagnostics,
}: RecorderBackendDiagnosticsSectionProps) {
  return (
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
              diagnostics?.recorder.portal.screenCastVersion?.toString() ?? null
            }
          />
          <DiagnosticRow
            label="Sources"
            value={diagnostics?.recorder.portal.sourceTypes.join(', ') ?? null}
          />
          <DiagnosticRow
            label="Cursor"
            value={diagnostics?.recorder.portal.cursorModes.join(', ') ?? null}
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
                ? formatBoolean(diagnostics.recorder.gpuCapture.eglAvailable)
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
              diagnostics?.recorder.gpuCapture.eglClientApis.join(', ') ?? null
            }
          />
          <DiagnosticRow
            label="OpenGL"
            value={
              diagnostics
                ? formatBoolean(diagnostics.recorder.gpuCapture.openglAvailable)
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
            value={diagnostics?.recorder.desktopAudio.defaultSourceId ?? null}
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
  );
}

function formatBoolean(value: boolean): string {
  return value ? 'Yes' : 'No';
}

function formatOptionalBoolean(value: boolean | null): string | null {
  return value === null ? null : formatBoolean(value);
}
