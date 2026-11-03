# Managed embedding runtime

The desktop app starts a separate embedding worker over private child-process
pipes. Its llama.cpp/ggml objects never link into the desktop library, which uses
Whisper's ggml. Production workers suppress diagnostic output and disable Linux
core dumps; they exit when their parent dies.

Build outputs stay separate: Whisper uses `OUT_DIR/whisper`, the CPU worker
`OUT_DIR/embedding-cpu`, and Vulkan `OUT_DIR/embedding-vulkan`. Resource staging
preserves unchanged timestamps and permissions. Generated embedding resources are
excluded from the Tauri development watcher; their source remains watched.

CPU is the default. Auto selects compiled Vulkan support when a GPU is available
and can fall back to CPU if loading fails. Explicit Vulkan reports failure rather
than switching backend. Build it with the `embedding-vulkan` Cargo feature and
Vulkan development tools. The CPU worker is always packaged; CPU-only builds
remove stale optional binaries.

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

## Verification

Native tests cover worker protocol, cancellation, restart and privacy. Synthetic
model checks compare tokenization and pooling with pinned reference models.
Run the owning tests before changing model conversion or input conventions.
GPU performance and general retrieval quality require separate evaluation.

See [validation commands](agent-validation.md) and
[third-party notices](../THIRD_PARTY_NOTICES.md).
