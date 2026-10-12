import { spawn, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);
const localConfigPath = new URL('../tauri-dev.local', import.meta.url);

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

function normalizeFeatureList(value) {
  return value
    .split(/\r?\n/)
    .map((line) => line.replace(/#.*/, ''))
    .join(',')
    .split(/[,\s]+/)
    .map((feature) => feature.trim())
    .filter(Boolean);
}

function resolveTauriArgs() {
  const args = ['tauri', 'dev'];
  const featureSource =
    process.env.DAYDREAM_TAURI_FEATURES ??
    (existsSync(localConfigPath) ? readFileSync(localConfigPath, 'utf8') : '');
  const features = normalizeFeatureList(featureSource);

  if (features.length > 0) {
    args.push('--features', ...features);
    console.warn(`Tauri dev cargo features: ${features.join(', ')}`);
  }

  return args;
}

const missingTools = ['meson', 'ninja'].filter(
  (tool) => spawnSync(tool, ['--version'], { stdio: 'ignore' }).status !== 0,
);
if (missingTools.length > 0) {
  console.error(
    `Missing native build tools: ${missingTools.join(', ')}. Install Meson and Ninja using README.md → Development → Prerequisites, then rerun yarn dev.`,
  );
  process.exit(1);
}

const child = spawn('yarn', resolveTauriArgs(), {
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
