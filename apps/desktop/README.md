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

## Linux prerequisites

On Debian or Ubuntu, the native development packages include:

```bash
sudo apt install \
  libwebkit2gtk-4.1-dev \
  build-essential \
  curl \
  wget \
  file \
  libxdo-dev \
  libssl-dev \
  libayatana-appindicator3-dev \
  librsvg2-dev \
  libpipewire-0.3-dev \
  libpulse-dev \
  libgstreamer1.0-dev \
  libgstreamer-plugins-base1.0-dev \
  libgstreamer-plugins-bad1.0-dev \
  gstreamer1.0-plugins-base \
  gstreamer1.0-plugins-good \
  gstreamer1.0-plugins-bad \
  gstreamer1.0-plugins-ugly \
  gstreamer1.0-libav \
  gstreamer1.0-tools \
  gstreamer1.0-pulseaudio \
  cmake \
  meson \
  ninja-build \
  libabsl-dev \
  clang \
  libclang-dev \
  tesseract-ocr
```

Package names vary by distribution.

On Arch Linux or CachyOS, install the echo-cancellation build prerequisites with
`sudo pacman -S --needed meson ninja abseil-cpp`. They must be on `PATH` before
running `yarn dev`; the launcher checks Meson and Ninja before starting the build.
