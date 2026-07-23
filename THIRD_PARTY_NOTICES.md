# Third-party notices

Daydream is licensed under `AGPL-3.0-or-later`. Third-party software, model
files, and system components keep their own licenses.

This document highlights code and binaries that Daydream vendors, resolves, or
expects directly. The JavaScript and Rust manifests and lockfiles contain the
complete dependency lists.

## whisper.cpp

Daydream vendors `whisper.cpp` source under
`apps/desktop/src-tauri/vendor/whisper.cpp`.

That source is licensed under the MIT License. Its copyright and license text
are preserved in
`apps/desktop/src-tauri/vendor/whisper.cpp/LICENSE`.

Whisper model files are downloaded separately and remain subject to their
upstream terms.

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
`apps/desktop/src/components/ui`.

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

Daydream also uses third-party Rust crates, JavaScript packages, native system
libraries, and optional user-configured model services. Their license metadata
is recorded in `Cargo.lock`, `yarn.lock`, and the corresponding package
manifests.

Publishing the source repository does not replace the license review required
before distributing desktop binaries.
