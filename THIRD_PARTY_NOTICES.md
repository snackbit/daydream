# Third-party notices

Daydream is licensed under `AGPL-3.0-or-later`. Third-party software, model
files, and system components keep their own licenses.

This document highlights code and binaries that Daydream vendors, resolves, or
expects directly. The JavaScript and Rust manifests and lockfiles contain the
complete dependency lists.

## Teleprompter virtualization

The React teleprompter uses `@tanstack/react-virtual` 3.14.13 and
`@tanstack/virtual-core` 3.17.11 (MIT, copyright 2021-present Tanner Linsley).
Their MIT notice is included in the packaged license resources under
`apps/desktop/src-tauri/resources/licenses/teleprompter/`.

## Model connections

The native model adapter uses `eventsource-stream` 0.2.3 (MIT OR Apache-2.0,
Julian Popescu) for SSE framing; this version was already pinned through the SDK.
The native model adapter uses `async-openai` 0.42.1 (MIT, Himanshu Neema)
and `openidconnect` 4.0.1 (MIT, David Ramos). Their required Rust versions
are 1.75 and 1.65 respectively, within the repository's Rust 1.95 toolchain.
Their MIT notices are packaged under
`apps/desktop/src-tauri/resources/licenses/model-connections/`.
The generated Cargo lock pins registry checksums. The SDK uses reqwest 0.13
through a bounded transport adapter; existing integrations retain reqwest 0.12.
TLS and authentication dependencies remain listed in the Cargo lock for bundle
license inventory. No OAuth client secret, model executable or remote account
credential is bundled.

## whisper.cpp

Daydream vendors `whisper.cpp` source under
`apps/desktop/src-tauri/vendor/whisper.cpp`.

That source is licensed under the MIT License. Its copyright and license text
are preserved in
`apps/desktop/src-tauri/vendor/whisper.cpp/LICENSE`.

Whisper model files are downloaded separately and remain subject to their
upstream terms.

Daydream's vendored VAD compute loop has one maintained correction: failed
graph computation resets its scheduler and returns failure instead of reporting
success with incomplete speech probabilities. The upstream API is unchanged.

## Managed embeddings

The isolated embedding worker vendors llama.cpp/GGML at revision
`abeada335e2e78bd3fe63febafab7e900ce75810` under
`apps/desktop/src-tauri/vendor/llama.cpp`. Its MIT license and upstream vendor
notices remain beside the sources. The worker also uses nlohmann JSON 3.12.0;
its MIT notice is preserved in `embedding-worker/JSON-LICENSE.txt`.

Multilingual E5 and BGE model weights are downloaded separately with pinned
checksums. Their upstream model licenses are MIT. The bundled E5 normalization
table comes from the pinned upstream tokenizer and retains its notice in
`embedding-worker/E5-LICENSE.txt`. Optional Vulkan builds preserve the
SPIRV-Headers notices in `embedding-worker/SPIRV-LICENSE.txt`.

Worker builds stage these notices beside the packaged executable. Exact source,
model and toolchain pins are documented in [embedding runtime](docs/embedding-runtime.md).

## Silero VAD

Silero v6.2.0 is bundled offline under `apps/desktop/src-tauri/resources/vad`.
The GGML artifact comes from ggml-org/whisper-vad revision
`c5c26827b67dfd053856f92e824e14fdcc123daf`. SHA-256:
`2aa269b785eeb53a82983a20501ddf7c1d9c48e33ab63a41391ac6c9f7fb6987`.
Its MIT license and source are included with the resource. Speech detection
runs locally on CPU; installing or using Daydream does not upload audio to Silero.

## FFmpeg and ffmpeg-static

The `packages/ffmpeg` development helper resolves `ffmpeg-static@5.3.0`. The
package declares `GPL-3.0-or-later` and supplies platform FFmpeg binaries.

FFmpeg build licensing depends on the libraries and build options used. Anyone
distributing an FFmpeg binary with Daydream must preserve the applicable
notices, provide required source or offers, and verify codec and patent
requirements for the target jurisdiction.

The current repository uses `ffmpeg-static` for development. This notice does
not claim that a future packaged binary has completed its distribution audit.

## GStreamer

The Linux recorder uses GStreamer libraries and plugins. Development machines
provide these through system packages, while some application bundle formats
may include media framework components.

GStreamer components and plugins may have different effective licensing and
codec or patent constraints. Binary packagers must inventory the exact
libraries and plugins included in each artifact.

## Tesseract and tessdata

Daydream can use a system Tesseract installation and can download selected
language data from the upstream `tessdata_best` repository. Tesseract,
Leptonica, and language data remain under their upstream licenses.

## shadcn/ui

Daydream includes generated shadcn/ui source under
`apps/desktop/src/components/ui` and the unchanged custom-variant section from
`shadcn@4.21.1` under `apps/desktop/src/shadcn-variants.css`.
Upstream: [Tailwind helper source](https://github.com/shadcn-ui/ui/blob/main/packages/shadcn/src/tailwind.css).
The local helpers preserve generated component state/orientation styling without
depending on the shadcn generator CLI or its server/registry dependencies.

MIT License

Copyright (c) 2023 shadcn

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Other dependencies

Microphone speaker-bleed removal uses `webrtc-audio-processing`, its configuration
crate, and native wrapper, pinned to 2.1.0. The bundled WebRTC AudioProcessing code
uses the BSD 3-Clause license. The upstream COPYING notice is packaged under
`apps/desktop/src-tauri/resources/licenses/webrtc-audio-processing/`.
Upstream: https://github.com/tonarino/webrtc-audio-processing/tree/v2.1.0.
Its build uses Abseil (Apache-2.0), supplied by the build environment or the
checksum-verified Meson wrap. Packagers must include the exact Abseil libraries
and their notices; bundled WebRTC does not bundle an arbitrary system Abseil.
The WAV reader/writer is `hound` 3.5.1 (Apache-2.0).
Upstream: https://github.com/ruuda/hound.

The capture editor uses `jpeg-encoder` 0.7.1, copyright (c) 2020 Vincent Stroebel,
under MIT or Apache-2.0, with the IJG notice for its quantization tables.
The unchanged upstream license texts are included in
`apps/desktop/src-tauri/resources/licenses/jpeg-encoder/` and packaged with the app.
Upstream: https://github.com/vstroebel/jpeg-encoder/tree/v0.7.1.

Daydream also uses third-party Rust crates, JavaScript packages, native system
libraries, and optional user-configured model services. Their license metadata
is recorded in `Cargo.lock`, `yarn.lock`, and the corresponding package
manifests.

Publishing the source repository does not replace the license review required
before distributing desktop binaries.
