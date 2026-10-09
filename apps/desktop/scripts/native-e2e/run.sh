#!/bin/sh
# Run from repository root. Reuse is explicit so stale assets cannot pass silently.
set -eu
DAYDREAM_E2E_SCRIPT="$(CDPATH= cd "$(dirname "$0")" && pwd)/run.sh"
CDPATH= cd "$(dirname "$DAYDREAM_E2E_SCRIPT")/../../../.."
DAYDREAM_E2E_WEBDRIVER_BIN="${DAYDREAM_WEBKIT_WEBDRIVER:-$(command -v WebKitWebDriver || true)}"
case " ${DAYDREAM_E2E_PHASES:-initial restart keyboard} " in
  *" keyboard "*)
    if [ ! -x "$DAYDREAM_E2E_WEBDRIVER_BIN" ]; then
      printf '%s\n' 'Matching WebKitWebDriver is required: install the distro driver or set DAYDREAM_WEBKIT_WEBDRIVER to its executable path.' >&2
      exit 1
    fi
    ;;
esac
export DAYDREAM_WEBKIT_WEBDRIVER="$DAYDREAM_E2E_WEBDRIVER_BIN"
if [ "${DAYDREAM_E2E_PRIVATE_BUS:-}" != 1 ] && [ "${1:-}" != --reuse-validated-build ]; then
  COREPACK_HOME=/tmp/corepack corepack yarn workspace @daydream/desktop build
  CARGO_HOME="${CARGO_HOME:-/tmp/daydream-cargo-home}" CARGO_BUILD_JOBS=2 cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --features tauri/custom-protocol
fi
if [ "${DAYDREAM_E2E_PRIVATE_BUS:-}" != 1 ]; then
  exec dbus-run-session -- env DAYDREAM_E2E_PRIVATE_BUS=1 sh "$DAYDREAM_E2E_SCRIPT" --reuse-validated-build
fi
ulimit -c 0
export PLAYWRIGHT_BROWSERS_PATH="${PLAYWRIGHT_BROWSERS_PATH:-${XDG_CACHE_HOME:-$HOME/.cache}/ms-playwright}"
DAYDREAM_E2E_DIR=$(mktemp -d /tmp/daydream-native-e2e.XXXXXX)
export DAYDREAM_E2E_DIR
export XDG_DATA_HOME="$DAYDREAM_E2E_DIR/data" XDG_CONFIG_HOME="$DAYDREAM_E2E_DIR/config"
export XDG_CACHE_HOME="$DAYDREAM_E2E_DIR/cache" XDG_STATE_HOME="$DAYDREAM_E2E_DIR/state"
export XDG_RUNTIME_DIR="$DAYDREAM_E2E_DIR/runtime"
export GDK_BACKEND=broadway BROADWAY_DISPLAY=:188 TZ=UTC GDK_SCALE=1 GDK_DPI_SCALE=1
export GTK_IM_MODULE=gtk-im-context-simple LC_ALL=C.UTF-8
export WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:39288
export XDG_CURRENT_DESKTOP=synthetic XDG_SESSION_TYPE=broadway DESKTOP_SESSION=synthetic
export DISPLAY= WAYLAND_DISPLAY=
export DAYDREAM_RECORDER_BACKEND=fake COREPACK_HOME=/tmp/corepack
DAYDREAM_FFMPEG_PATH=$(corepack yarn workspace @daydream/desktop exec node --input-type=module -e 'import ffmpeg from "ffmpeg-static"; process.stdout.write(ffmpeg)')
export DAYDREAM_FFMPEG_PATH
mkdir -m 700 -p "$XDG_DATA_HOME" "$XDG_CONFIG_HOME" "$XDG_CACHE_HOME" "$XDG_STATE_HOME" "$XDG_RUNTIME_DIR" "$DAYDREAM_E2E_DIR/package/embedding" "$DAYDREAM_E2E_DIR/package/vad"
cc -shared -fPIC apps/desktop/scripts/native-e2e/broadway-dpi.c -ldl -o "$DAYDREAM_E2E_DIR/broadway-dpi.so"
cp apps/desktop/src-tauri/target/debug/daydream-desktop "$DAYDREAM_E2E_DIR/package/"
cp apps/desktop/src-tauri/resources/embedding/daydream-embedding-worker "$DAYDREAM_E2E_DIR/package/embedding/"
cp apps/desktop/src-tauri/resources/vad/ggml-silero-v6.2.0.bin "$DAYDREAM_E2E_DIR/package/vad/"
printf 'Native E2E artifacts: %s\n' "$DAYDREAM_E2E_DIR"
DAYDREAM_E2E_BROADWAY_PID=
DAYDREAM_E2E_WEBDRIVER_PID=
DAYDREAM_E2E_APP_PID=
cleanup() {
  if [ -n "$DAYDREAM_E2E_WEBDRIVER_PID" ]; then
    kill "$DAYDREAM_E2E_WEBDRIVER_PID" 2>/dev/null || true
    wait "$DAYDREAM_E2E_WEBDRIVER_PID" 2>/dev/null || true
  fi
  if [ -n "$DAYDREAM_E2E_APP_PID" ]; then
    kill "$DAYDREAM_E2E_APP_PID" 2>/dev/null || true
    wait "$DAYDREAM_E2E_APP_PID" 2>/dev/null || true
  fi
  if [ -n "$DAYDREAM_E2E_BROADWAY_PID" ]; then
    kill "$DAYDREAM_E2E_BROADWAY_PID" 2>/dev/null || true
    wait "$DAYDREAM_E2E_BROADWAY_PID" 2>/dev/null || true
  fi
}
trap cleanup EXIT INT TERM
for DAYDREAM_E2E_PHASE in ${DAYDREAM_E2E_PHASES:-initial restart keyboard}; do
  export XDG_RUNTIME_DIR="$DAYDREAM_E2E_DIR/runtime/$DAYDREAM_E2E_PHASE"
  mkdir -m 700 -p "$XDG_RUNTIME_DIR"
  broadwayd -a 127.0.0.1 -p 39188 :188 > "$DAYDREAM_E2E_DIR/broadway-$DAYDREAM_E2E_PHASE.log" 2>&1 &
  DAYDREAM_E2E_BROADWAY_PID=$!
  if [ "$DAYDREAM_E2E_PHASE" = keyboard ]; then
    unset WEBKIT_INSPECTOR_HTTP_SERVER
    export TAURI_WEBVIEW_AUTOMATION=true WEBKIT_INSPECTOR_SERVER=127.0.0.1:39289
    "$DAYDREAM_WEBKIT_WEBDRIVER" --host=127.0.0.1 --port=39290 --target=127.0.0.1:39289 > "$DAYDREAM_E2E_DIR/webdriver.log" 2>&1 &
    DAYDREAM_E2E_WEBDRIVER_PID=$!
  else
    unset TAURI_WEBVIEW_AUTOMATION WEBKIT_INSPECTOR_SERVER
    export WEBKIT_INSPECTOR_HTTP_SERVER=127.0.0.1:39288
  fi
  LD_PRELOAD="$DAYDREAM_E2E_DIR/broadway-dpi.so" "$DAYDREAM_E2E_DIR/package/daydream-desktop" > "$DAYDREAM_E2E_DIR/app-$DAYDREAM_E2E_PHASE.log" 2>&1 &
  DAYDREAM_E2E_APP_PID=$!
  corepack yarn workspace @daydream/desktop exec node scripts/native-e2e/journeys.mjs "$DAYDREAM_E2E_PHASE"
  kill "$DAYDREAM_E2E_APP_PID"
  wait "$DAYDREAM_E2E_APP_PID" || true
  DAYDREAM_E2E_APP_PID=
  kill "$DAYDREAM_E2E_BROADWAY_PID" 2>/dev/null || true
  wait "$DAYDREAM_E2E_BROADWAY_PID" 2>/dev/null || true
  DAYDREAM_E2E_BROADWAY_PID=
  if [ -n "$DAYDREAM_E2E_WEBDRIVER_PID" ]; then
    kill "$DAYDREAM_E2E_WEBDRIVER_PID" 2>/dev/null || true
    wait "$DAYDREAM_E2E_WEBDRIVER_PID" 2>/dev/null || true
    DAYDREAM_E2E_WEBDRIVER_PID=
  fi
done
