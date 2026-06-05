import tailwindcss from '@tailwindcss/vite';
import react from '@vitejs/plugin-react';
import { playwright } from '@vitest/browser-playwright';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

const dirname = path.dirname(fileURLToPath(import.meta.url));

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  plugins: [react(), ...tailwindcss()],
  resolve: {
    alias: {
      '@': path.resolve(dirname, 'src'),
    },
  },
  optimizeDeps: {
    include: ['react-resizable-panels'],
  },

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    ...(host
      ? {
          hmr: {
            protocol: 'ws' as const,
            host,
            port: 1421,
          },
        }
      : {}),
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    coverage: {
      all: true,
      include: [
        'src/features/recorder/scene-layout.ts',
        'src/features/recorder/recorder-client.ts',
        'src/features/recorder/recorder-workspace-store.ts',
      ],
      provider: 'v8' as const,
      reporter: ['text', 'json-summary'],
      thresholds: {
        branches: 82,
        functions: 94,
        lines: 92,
        statements: 92,
      },
    },
    projects: [
      {
        extends: true as const,
        test: {
          exclude: ['dist/**', 'src/**/*.browser.test.tsx'],
          include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
          name: 'unit',
        },
      },
      {
        extends: true as const,
        test: {
          browser: {
            enabled: true,
            headless: true,
            instances: [{ browser: 'chromium' as const }],
            provider: playwright(),
          },
          exclude: ['dist/**'],
          include: ['src/**/*.browser.test.tsx'],
          name: 'browser',
        },
      },
    ],
  },
}));
