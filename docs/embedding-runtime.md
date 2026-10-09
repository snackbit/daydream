# Managed embedding runtime

Daydream builds an independent `daydream-embedding-worker` executable. Its
llama.cpp/ggml objects never link into the desktop library, whose existing ggml
belongs to Whisper. The worker is packaged under `embedding/` and communicates
only through private child-process pipes. There is no HTTP listener or Python
runtime in the app. Production builds disable upstream diagnostic output.
Build and install outputs are isolated too: Whisper uses `OUT_DIR/whisper`,
the CPU worker `OUT_DIR/embedding-cpu`, and Vulkan `OUT_DIR/embedding-vulkan`.
Only Whisper's library directory is emitted as a Rust native link search path.
The combined native gate caught a shared-install-directory collision during
development; the corrected archive paths and rebuilt executable were checked,
then Whisper VAD and embedding process tests passed together.
The isolation regression passed six real synthetic Whisper VAD tests and the
embedding process/API suite. Installed archive source paths stay with their
owner, and the rebuilt Rust test executable contains no llama.cpp GGML source
paths. The Linux worker disables core dumps and exits when its parent dies,
including while inference is active.

Resource staging compares bytes before copying workers or notices, preserving
unchanged destination timestamps and executable permissions. Generated
`resources/embedding/` is excluded from Tauri's dev watcher through `.taurignore`;
the worker, vendored runtime and build-support source remain watched. This prevents
a build from triggering its own restart. `yarn test:dev-watch` exercises the real
CLI watcher with a synthetic Cargo runner: unignored generated writes reproduce a
restart, ignored writes do not, and Rust source edits still restart normally.

The default is multilingual E5 Small Q8_0 on CPU. Auto uses a compiled Vulkan
backend only when an actual Vulkan GPU is available, then falls back to CPU if
model/context allocation fails. Explicit Vulkan reports an unavailable backend
or loading failure. Build Vulkan support with the `embedding-vulkan` Cargo
feature and the platform's Vulkan development tools. That feature additionally
packages `daydream-embedding-worker-vulkan`; the CPU executable is always
included. Auto can therefore fall back even when the Vulkan loader is missing
and the optional process cannot start. CPU builds remove stale optional binaries
before packaging. GPU quality/performance
must be verified separately on supported hardware.

The verified development toolchain uses glslc `2026.3` / Vulkan `1.4.357` and
official SPIRV-Headers revision `86f980c731e62ae4eaf383d320449d71687936bf`
(archive SHA256 `d11ef011d0788ed002d7cc96224d298f43b126cabe887cd4d46199cb19ecf386`).
Headers are a compile-time prerequisite; their license is included with the
worker notices. The validation setup installs them only into a `/tmp` prefix.
The worker CMake boundary links the SPIRV-Headers include interface explicitly;
the pinned upstream finds the package without attaching that interface.
The upstream shader generator can launch sixteen compilers independently of
CMake's job count. The validation build supplies a glslc wrapper using `flock`
to serialize shader compilers so they plus the other CMake job fit the two-job
native budget.

## Pinned sources and licenses

- [Official llama.cpp](https://github.com/ggml-org/llama.cpp/tree/abeada335e2e78bd3fe63febafab7e900ce75810):
  revision `abeada335e2e78bd3fe63febafab7e900ce75810`, MIT. Source archive SHA256
  `9d5bfc27b3c3c3fc9fef962587381f071c44c4b216e2c32a12b69b7d20165b32`.
  The vendored subset contains unchanged core, GGML, CMake and upstream vendor
  files; upstream licenses remain beside their sources.
- [Multilingual E5 conversion](https://huggingface.co/cstr/multilingual-e5-small-GGUF/tree/274299fdb785d87e8d907a24af76016cb1a8a1d8):
  `multilingual-e5-small-q8_0.gguf`, 131,624,960 bytes, SHA256
  `0a34067a40f25d3149b36885faa62bee0e5284d0f9edc102acfc00e115d953e8`.
  [Official upstream weights](https://huggingface.co/intfloat/multilingual-e5-small/tree/614241f622f53c4eeff9890bdc4f31cfecc418b3)
  are MIT licensed. This older third-party conversion requires the tested
  compatibility adapter below; the unmodified file is incompatible with the
  pinned runtime.
- [Official ggml-org BGE conversion](https://huggingface.co/ggml-org/bge-small-en-v1.5-Q8_0-GGUF/tree/97abb509f3e9e1b4ea4287427aec0c4b78a1e468):
  `bge-small-en-v1.5-q8_0.gguf`, 36,685,152 bytes, MIT, SHA256
  `f046db1dc724cf4f6f0a0c5917e922823b73eb1d27b8f9a9c2797f7866974804`.
  Its [official reference weights](https://huggingface.co/BAAI/bge-small-en-v1.5/tree/5c38ec7c405ec4b44b94cc5a9bb96e735b38267a)
  are pinned to `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a` for tokenizer/CLS proof.

Model binaries are downloaded to the managed model cache and verified before
loading. They are not tracked in Git. Successful downloads run real synthetic
CPU embedding tests before reporting the self-test as passed.

## E5 compatibility and preprocessing

The worker repairs only metadata in an anonymous temporary GGUF file; original
Q8 tensors remain byte-identical. It declares the official two token-type rows,
uses the official Unigram tokenizer (`t5` in GGUF), and supplies official
whitespace and normalization conventions. Its tokenizer adapter preserves HF's
literal special-token boundaries and trailing normalized whitespace pieces,
which the runtime's generic Unigram normalization otherwise removes.
The normalization table is extracted
from the pinned official tokenizer JSON with `extract-normalizer.py`; its SHA256
is `0942789e0111452f1cc446f70036e26d24458c3c7858c53cc12ca4d87531279b`.
The worker closes the anonymous file on exit; no persistent adapted artifact is
needed. The embedding-space fingerprint includes model checksum, runtime,
adapter version, normalization table and indexing preprocessing.

E5 uses `query: ` / `passage: ` and mean pooling. BGE uses its official retrieval
instruction for queries and CLS pooling. Both return normalized 384-dimensional
vectors. Canonical evidence splits long documents at word/sentence boundaries
into at most 480-UTF8-byte fragments with 96-byte overlap. Each fragment
receives its own complete passage embedding; source, original document and time
provenance remain attached. Inputs exceeding 512 actual tokens fail explicitly.
No text is truncated or averaged into a whole-page vector.

Each request contains at most 32 texts, 8 KiB each, with a 300 KB serialized
request and 1 MiB reply limit. The worker uses two inference threads. Cancellation
is polled every 25 ms and kills the child, as do timeout/protocol/crash failures.
The next request may restart it. An idle child unloads after 60 seconds. The
app retains at most one loaded worker across request snapshots, with one shared
inference slot. Calls have a 120-second deadline.

Remote embedding calls require their own enabled profile and explicit `api-key`
or `none` authentication. Redirects are disabled. Response bytes, counts,
indices, dimensions, finite/nonzero values and cancellations are checked before
indexing. API model identity, dimensions, endpoint and input conventions identify
the vector space; the full settings fingerprint also guards configuration races.
No remote body, key, prompt or captured text is included in ordinary errors.

## Actual runtime proof (2026-10-06–07)

`verify-reference.py` compares the native default against pinned official weights
using development-only torch `2.11.0+cpu`, transformers `5.5.0`, sentencepiece
`0.2.1`, numpy `2.4.3`, protobuf `7.34.1` and safetensors `0.8.0` in `/tmp`.
English, German, umlauts, decomposed accents, ligatures, full-width letters,
literal special tokens, unknown characters, emoji, Chinese and Arabic have exact
matching tokenizer IDs. All nine normalized vectors have cosine similarity at
least **0.999831** with the official mean-pooled reference.
English/German travel-budget paraphrases outrank the unrelated garden fixture.
The optional BGE model has exact matching token IDs on the same nine fixtures
and normalized CLS-pooling reference cosine at least **0.999758**. Its English
travel fixture scored 0.756 against its paraphrase and 0.357 against the unrelated
garden fixture.

`verify-retrieval.py` exercises the staged production worker with English/German
expense-spreadsheet paraphrases. A relevant paragraph is embedded intact within
a bounded passage among twenty unrelated garden paragraphs. It outranks every
unrelated passage in each language (English 0.791 vs at most 0.748; German 0.800
vs at most 0.791). All passages fit the actual token limit. Production stderr is
empty and no token IDs are emitted. Canonical splitting, source identity and
privacy are separately covered by native storage/search scenarios. These
synthetic fixtures do not establish general retrieval quality.

The separate CPU/Vulkan executable pair was staged under the exact packaged
filenames and tested on an NVIDIA GeForce RTX 4090 with driver `615.71.09`.
CPU/Vulkan normalized-vector cosine is at least **0.999877** for E5 and
**0.999883** for BGE. Auto selected the real Vulkan device and matched explicit
Vulkan within float rounding. With Vulkan devices hidden, Auto ran the same
model on CPU and matched the CPU baseline; explicit Vulkan returned
`vulkan_unavailable`. Every production-worker probe emitted zero stderr bytes.
The native process-boundary suite separately covers a broken optional GPU
executable falling back to the always-packaged CPU executable.

A persistent release CPU worker measured 20 synthetic English/German queries,
including pipe serialization and JSON parsing: **4.36 ms p50**, **5.76 ms p95**,
**5.82 ms maximum**, with **461 ms** cold model startup. This is separate from
the native full-search database benchmark; frontend latency was not measured
by this probe. These measurements do not establish other hardware performance.
Cross-platform packaging remains a separate validation requirement.

Reproduce with pinned official files in a development cache:

```sh
cmake -S apps/desktop/src-tauri/embedding-worker -B /tmp/daydream-embedding-build \
  -DGGML_VULKAN=OFF -DDAYDREAM_MODEL_VALIDATION=ON
cmake --build /tmp/daydream-embedding-build --target daydream-embedding-worker -j2
python apps/desktop/src-tauri/embedding-worker/verify-reference.py \
  /tmp/daydream-official-e5 /tmp/daydream-embedding-build/daydream-embedding-worker \
  /tmp/daydream-e5.gguf
python apps/desktop/src-tauri/embedding-worker/verify-reference.py \
  /tmp/daydream-official-bge /tmp/daydream-embedding-build/daydream-embedding-worker \
  /tmp/daydream-bge.gguf bge-small-en-v1.5-q8_0
python apps/desktop/src-tauri/embedding-worker/verify-retrieval.py \
  apps/desktop/src-tauri/resources/embedding/daydream-embedding-worker \
  /tmp/daydream-e5.gguf
python apps/desktop/src-tauri/embedding-worker/verify-backends.py \
  /tmp/daydream-embedding-package/embedding/daydream-embedding-worker \
  /tmp/daydream-embedding-package/embedding/daydream-embedding-worker-vulkan \
  /tmp/daydream-e5.gguf multilingual-e5-small-q8_0
python apps/desktop/src-tauri/embedding-worker/measure-query.py \
  /tmp/daydream-embedding-package/embedding/daydream-embedding-worker \
  /tmp/daydream-e5.gguf multilingual-e5-small-q8_0 cpu
```

The validation flag exposes synthetic token IDs and diagnostic output for this
offline developer proof. Cargo distribution builds force this flag off.

For a feature-enabled Cargo package, supply the staged header prefix and the
same bounded shader compiler wrapper:

```sh
CMAKE_PREFIX_PATH=/tmp/daydream-spirv-headers-install \
DAYDREAM_VULKAN_GLSLC=/tmp/daydream-limited-glslc \
CARGO_BUILD_JOBS=2 cargo build --manifest-path apps/desktop/src-tauri/Cargo.toml \
  --locked --features embedding-vulkan
```

The two temporary paths refer to development prerequisites, not runtime files.
The app packages both executables; CPU-only packages include only the CPU worker.
