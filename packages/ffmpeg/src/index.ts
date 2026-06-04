import { accessSync, constants } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const ffmpegStaticPath = require('ffmpeg-static') as unknown;

export interface FfmpegResolution {
  path: string;
  source: 'ffmpeg-static';
}

export function resolveFfmpegBinaryPath(): string {
  if (typeof ffmpegStaticPath !== 'string' || ffmpegStaticPath.length === 0) {
    throw new Error('ffmpeg-static did not resolve an FFmpeg binary path.');
  }

  accessSync(ffmpegStaticPath, constants.X_OK);

  return ffmpegStaticPath;
}

export function resolveFfmpeg(): FfmpegResolution {
  return {
    path: resolveFfmpegBinaryPath(),
    source: 'ffmpeg-static',
  };
}

export function createFfmpegEnv(): NodeJS.ProcessEnv {
  return {
    ...process.env,
    DAYDREAM_FFMPEG_PATH: resolveFfmpegBinaryPath(),
  };
}
