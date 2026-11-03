// One synthetic black H.264 frame. No recorded content or audio.
import { resolveFfmpegBinaryPath } from '@daydream/ffmpeg';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const output = new URL(
  '../src/features/startup/media/decoder-warmup.mp4',
  import.meta.url,
);
const result = spawnSync(
  resolveFfmpegBinaryPath(),
  [
    '-hide_banner',
    '-loglevel',
    'error',
    '-y',
    '-f',
    'lavfi',
    '-i',
    'color=black:size=640x360:rate=1:duration=1',
    '-frames:v',
    '1',
    '-c:v',
    'libx264',
    '-threads',
    '1',
    '-profile:v',
    'baseline',
    '-pix_fmt',
    'yuv420p',
    '-movflags',
    '+faststart',
    '-an',
    fileURLToPath(output),
  ],
  { stdio: 'inherit' },
);
if (result.status !== 0)
  throw new Error('Playback warmup asset generation failed.');
