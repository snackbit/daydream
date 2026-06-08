import { spawn } from 'node:child_process';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

function createLinuxWaylandEnv() {
  if (
    process.platform !== 'linux' ||
    process.env.XDG_SESSION_TYPE !== 'wayland'
  ) {
    return {};
  }

  return {
    WEBKIT_DISABLE_DMABUF_RENDERER:
      process.env.WEBKIT_DISABLE_DMABUF_RENDERER ?? '1',
  };
}

function resolveFfmpegEnv() {
  if (process.env.DAYDREAM_FFMPEG_PATH) {
    return {};
  }

  const ffmpegPath = require('ffmpeg-static');
  if (typeof ffmpegPath !== 'string' || ffmpegPath.length === 0) {
    return {};
  }

  return {
    DAYDREAM_FFMPEG_PATH: ffmpegPath,
  };
}

const child = spawn('yarn', ['tauri', 'dev'], {
  env: {
    ...process.env,
    ...resolveFfmpegEnv(),
    ...createLinuxWaylandEnv(),
  },
  stdio: 'inherit',
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }

  process.exit(code ?? 0);
});
