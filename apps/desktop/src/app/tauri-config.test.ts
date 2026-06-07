import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'vitest';

const dirname = path.dirname(fileURLToPath(import.meta.url));
const tauriConfigPath = path.resolve(
  dirname,
  '../../src-tauri/tauri.conf.json',
);

interface TauriConfig {
  bundle?: {
    linux?: {
      appimage?: {
        bundleMediaFramework?: boolean;
      };
      deb?: {
        depends?: string[];
      };
      rpm?: {
        depends?: string[];
      };
    };
  };
}

test('Linux bundles provision required GStreamer runtime plugins', () => {
  const config = JSON.parse(
    readFileSync(tauriConfigPath, 'utf8'),
  ) as TauriConfig;

  expect(config.bundle?.linux?.appimage?.bundleMediaFramework).toBe(true);
  expect(config.bundle?.linux?.deb?.depends).toEqual(
    expect.arrayContaining([
      'libgstreamer1.0-0',
      'gstreamer1.0-plugins-base',
      'gstreamer1.0-plugins-good',
      'gstreamer1.0-plugins-bad',
      'gstreamer1.0-plugins-ugly',
      'gstreamer1.0-pulseaudio',
    ]),
  );
  expect(config.bundle?.linux?.rpm?.depends).toEqual(
    expect.arrayContaining([
      'gstreamer1',
      'gstreamer1-plugins-base',
      'gstreamer1-plugins-good',
      'gstreamer1-plugins-bad-free',
      'gstreamer1-plugins-ugly',
    ]),
  );
});
