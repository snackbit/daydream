import { resolveFfmpegBinaryPath } from '@daydream/ffmpeg';
import { spawn } from 'node:child_process';

const ffmpegPath = resolveFfmpegBinaryPath();

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

const child = spawn('yarn', ['tauri', 'dev'], {
  env: {
    ...process.env,
    DAYDREAM_FFMPEG_PATH: ffmpegPath,
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
