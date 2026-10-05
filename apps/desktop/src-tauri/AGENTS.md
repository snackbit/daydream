# Native instructions

Applies to maintained Rust/Tauri code and tests here. Root privacy and compatibility
rules still apply. This app's Cargo manifest is not a root Cargo workspace.
The package declares Rust 1.95 minimum; check dependency MSRVs before upgrades.

## Boundaries and lifecycle

- Commands validate input, call focused domain/storage/service owners and map errors.
  Keep SQL row mapping, filesystem work and background jobs out of thin handlers.
- SQLite owns durable relational metadata, explicit embedded migrations and job
  state. Test migrations and preserve schema/command/event/payload compatibility
  unless an approved task changes the contract.
- Keep Linux portal/PipeWire/GStreamer, input/window providers, media writing and
  unsupported-platform adapters behind typed interfaces. Do not assume frontend
  tests prove native permission, device, session or media behavior.
- Recording state is user-controlled. Native worker/provider failures must have
  explicit lifecycle ownership and visible errors; do not report recording while
  its active capture/writer resources have failed.
- Keep callbacks and capture loops bounded. Do not block callbacks on processing,
  model/network calls or expensive allocations. Use focused deterministic helpers
  for chunk planning, timestamp alignment, eligibility and metadata mapping.
- Do not hold synchronous locks across async work. Trace existing lock/request
  ownership before modifying lifecycle, cancellation, background workers or IPC.
- Follow [protected-window rules](../../../docs/protected-window-recording-rules.md)
  for continuous matches, user pause precedence, unknown metadata and durable
  protection. Failed protection persistence must not enable unsafe processing.

## Processing, storage and privacy

- Every processing/retry/resume/read path must respect current redactions and
  protection eligibility. Recheck after async boundaries when privacy state can
  change. Cancellation and restart recovery cannot strand jobs or expose old results.
- Keep desktop and microphone audio distinguishable and aligned to source timeline.
  OCR processes retained frames unless the task defines another policy; visual/model
  observations, transcripts and summaries retain provenance and bounded inputs.
- Validate remote/model output at runtime before storing/displaying structured facts.
  Optional remote calls require explicit configuration and labeled user control.
  Do not put sensitive content or provider bodies in ordinary errors/logs.
- Use stable source IDs across playback lanes, redaction targets, processing and
  result filters. Changing privacy state invalidates cached/derived results and
  excludes hidden media from subsequent local or remote work.
- Preserve reversible metadata redaction where required. Physical deletion and
  retention remove media, derived data and processing temp artifacts, including
  recovery leftovers. Filesystem and SQL failures need an explicit recovery policy.
- Isolate vector/index adapters if introduced; deletion/invalidation and timestamp
  provenance must survive index choice. Do not implement roadmap features implicitly.

## Native tools, generated files and checks

- `build.rs` builds vendored `vendor/whisper.cpp` with CMake/bindgen. Treat vendor,
  model, binary and GPU-feature changes as separate reviewed integrations, including
  license/checksum/packaging and toolchain requirements. Never format/rewrite vendor
  sources during ordinary app changes.
- Development FFmpeg comes from `packages/ffmpeg`; distribution binaries must be
  app-managed with documented packaging, not an undeclared user installation.
- Tauri config/capabilities and generated schemas are part of native wiring. Use
  the supported generator; inspect generated changes and preserve collaborators'
  edits. Permission changes must be scoped and documented.
- Cargo dependencies use compatible updates and generated locks. Use `--locked`
  for checks. Do not reset caches, change target directories or build release just
  to validate source changes.
- Heavy native checks share `CARGO_BUILD_JOBS=2`; coordinate one native check at a
  time. Start with the relevant module/tests, then run the final full native gate
  once changes settle. Some transcription/playback tests need loopback binding.
- Capture coverage includes multi-screen metadata, audio separation, permission
  failures and lifecycle. Redaction covers visibility/invalidation and processing
  exclusion; processing covers chunk/timestamp provenance, resume and partial failure.
  Deletion covers derived/temp artifacts; privacy covers default no-logging/upload.
- Use synthetic fixtures. Tests mock processes, native devices, network/model and
  filesystem boundaries; deterministic production rules remain real.

Commands and host prerequisites: [validation map](../../../docs/agent-validation.md).
