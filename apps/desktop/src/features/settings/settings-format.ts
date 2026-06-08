import type { AppSettings, RecorderScene } from '@daydream/shared';

export const GIB_BYTES = 1024 ** 3;
const DESKTOP_AUDIO_BITRATE_KBPS = 96;
const MICROPHONE_AUDIO_BITRATE_KBPS = 64;
const HOUR_SECONDS = 60 * 60;

export function formatFps(fps: number): string {
  if (fps < 1) {
    const seconds = 1 / fps;
    const formattedSeconds = Number.isInteger(seconds)
      ? seconds.toString()
      : seconds.toFixed(1);

    return `1 frame every ${formattedSeconds} seconds`;
  }

  return `${formatNumber(fps)} ${fps === 1 ? 'frame' : 'frames'} per second`;
}

export function formatGiB(bytes: number): string {
  return formatNumber(bytes / GIB_BYTES);
}

export function gibToBytes(gib: number): number {
  return Math.round(gib * GIB_BYTES);
}

export function estimateOneHourRecordingBytes(
  settings: AppSettings,
  scene: RecorderScene | null,
): number {
  let bitrateKbps = settings.record.videoBitrateKbps;
  if (scene?.desktopAudioSourceId) {
    bitrateKbps += DESKTOP_AUDIO_BITRATE_KBPS;
  }
  if (scene?.microphoneAudioSourceId) {
    bitrateKbps += MICROPHONE_AUDIO_BITRATE_KBPS;
  }

  return Math.round((bitrateKbps * 1000 * HOUR_SECONDS) / 8);
}

export function formatStorageSize(bytes: number): string {
  if (bytes >= GIB_BYTES) {
    return `${formatNumber(bytes / GIB_BYTES)} GiB`;
  }

  return `${formatNumber(bytes / 1024 ** 2)} MiB`;
}

export function formatSceneDimensions(scene: RecorderScene | null): string {
  if (!scene) {
    return 'No saved scene';
  }

  return `${scene.canvasWidth} x ${scene.canvasHeight}`;
}

function formatNumber(value: number): string {
  if (Number.isInteger(value)) {
    return value.toString();
  }

  return value.toFixed(2).replace(/\.?0+$/, '');
}
