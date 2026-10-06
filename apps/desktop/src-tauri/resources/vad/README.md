# Bundled speech detector

Silero VAD v6.2.0, GGML conversion distributed by ggml-org.

- Source: https://huggingface.co/ggml-org/whisper-vad/resolve/c5c26827b67dfd053856f92e824e14fdcc123daf/ggml-silero-v6.2.0.bin
- SHA-256: `2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987`
- License: MIT, reproduced in `LICENSE`.

The application bundles this model. Runtime speech detection is CPU-only and
offline, and never downloads a replacement implicitly. The adapter verifies the
model before loading it.
