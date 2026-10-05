# Desktop development

Tauri 2 hosts the React/Vite app. TanStack Router owns navigation and Zustand owns
shared UI state; Rust owns capture, playback, processing, settings and SQLite.
Read the [root README](../../README.md) for setup, capabilities and native limits.

Run commands from the repository root:

```sh
COREPACK_HOME=/tmp/corepack corepack yarn dev
```

The launcher uses scripts/tauri-dev.mjs and defaults WEBKIT_DISABLE_DMABUF_RENDERER=1
on Linux Wayland for the known WebKitGTK DMA-BUF issue.

For frontend-only iteration without rebuilding Rust:

```sh
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop dev
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test:unit <file>
COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop test:browser <file>
```

The ordinary Vite page has no native Tauri runtime. Browser Mode tests use synthetic
IPC fixtures to run user workflows; the Vite page is not a simulated recorder.

The [validation map](../../docs/agent-validation.md) defines scoped/full
checks, budgets and Playwright/loopback prerequisites. Browser tests prove behavior
at the IPC boundary. Real portal consent, window/input access, audio devices, media
writing and packaged behavior require native/manual checks.

Ownership: [frontend](src/AGENTS.md), [native](src-tauri/AGENTS.md).
