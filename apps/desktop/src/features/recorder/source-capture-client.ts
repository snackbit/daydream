import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import type {
  CaptureSetup,
  RecorderScene,
  RecorderSceneSource,
  SaveCaptureSetupInput,
} from '@daydream/shared';

function invalid(): never {
  throw new Error(
    'Capture setup returned an invalid response. Reload the workspace.',
  );
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
function record(value: unknown): Record<string, unknown> {
  if (!isRecord(value)) return invalid();
  return value;
}
function text(value: unknown): string {
  return typeof value === 'string' ? value : invalid();
}
function number(value: unknown): number {
  return typeof value === 'number' && Number.isFinite(value)
    ? value
    : invalid();
}
function nullableText(value: unknown): string | null {
  return value === null ? null : text(value);
}
function nullableNumber(value: unknown): number | null {
  return value === null ? null : number(value);
}
function source(value: unknown): RecorderSceneSource {
  const r = record(value),
    crop = record(r.crop),
    rect = record(r.rect);
  if (r.sourceKind !== 'monitor' || typeof r.preserveAspectRatio !== 'boolean')
    return invalid();
  return {
    id: text(r.id),
    sourceId: text(r.sourceId),
    displayName: text(r.displayName),
    sourceKind: r.sourceKind,
    preserveAspectRatio: r.preserveAspectRatio,
    sourceWidth: nullableNumber(r.sourceWidth),
    sourceHeight: nullableNumber(r.sourceHeight),
    zIndex: number(r.zIndex),
    crop: {
      left: number(crop.left),
      top: number(crop.top),
      right: number(crop.right),
      bottom: number(crop.bottom),
    },
    rect: {
      x: number(rect.x),
      y: number(rect.y),
      width: number(rect.width),
      height: number(rect.height),
    },
  };
}
function scene(value: unknown): RecorderScene {
  const r = record(value);
  if (
    !Array.isArray(r.sources) ||
    typeof r.independentVideo !== 'boolean' ||
    !['auto-16-9', 'custom', 'horizontal', 'vertical'].includes(
      text(r.layoutMode),
    )
  )
    return invalid();
  const mode = r.layoutMode;
  if (
    mode !== 'auto-16-9' &&
    mode !== 'custom' &&
    mode !== 'horizontal' &&
    mode !== 'vertical'
  )
    return invalid();
  return {
    id: text(r.id),
    name: text(r.name),
    canvasWidth: number(r.canvasWidth),
    canvasHeight: number(r.canvasHeight),
    desktopAudioSourceId: nullableText(r.desktopAudioSourceId),
    microphoneAudioSourceId: nullableText(r.microphoneAudioSourceId),
    layoutMode: mode,
    sources: r.sources.map(source),
    independentVideo: r.independentVideo,
  };
}
function setup(value: unknown): CaptureSetup {
  const r = record(value);
  return {
    scene: r.scene === null ? null : scene(r.scene),
    fps: number(r.fps),
    videoBitrateKbps: number(r.videoBitrateKbps),
  };
}
export async function getCaptureSetup(): Promise<CaptureSetup> {
  return setup(await invoke<unknown>('getCaptureSetup'));
}
export function listenToCaptureSetupSaved(
  onSaved: (saved: CaptureSetup) => void,
) {
  return listen<unknown>('capture-setup-saved', (event) => {
    onSaved(setup(event.payload));
  });
}
export async function saveCaptureSetup(
  input: SaveCaptureSetupInput,
): Promise<CaptureSetup> {
  return setup(await invoke<unknown>('saveCaptureSetup', { input }));
}
