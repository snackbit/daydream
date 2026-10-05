# Frontend instructions

Applies to React/TypeScript source and tests here. Root privacy and compatibility
rules still apply. Read nearby owners and tests before choosing a pattern.

## UI and state ownership

- Follow the user's studio direction: dense, precise controls, quiet neutral
  surfaces, restrained color and minimal radius/borders. Shadcn primitives do not
  require oversized cards or default rounded styling. Keep layout changes coherent
  with that direction; check current screenshots rather than inventing new tokens.
  Light and dark modes need equal quality and readable critical states. Provide a
  persistent, accessible appearance control that users can reach easily.

- MUST use actual shadcn components in `components/ui` when an eligible component
  exists. Before building a surface, inspect those components and official shadcn
  docs. Use `../components.json` when adding official registry source; do not
  hand-roll equivalent controls. The generator CLI is not a runtime dependency.
  `shadcn-variants.css` keeps unchanged upstream state helpers locally. Verify
  generator version/advisories before a one-off Corepack Yarn generation command,
  and inspect generated source, CSS and dependency changes before adopting them.
- Navigation MUST go through TanStack Router. Screen/route files own params,
  top-level loading/error/delete/navigation and feature composition. Detailed
  playback, editors, toolbars, dialogs, panels and debug flows belong under
  `features/<feature>/`.
- Use component-local state for local concerns, focused Zustand stores for shared
  state, and narrow subscriptions. Sibling orchestration belongs to a named hook or
  controller, not mutable command refs. Refs model imperative boundaries such as
  media elements, canvas, timers, observers and scroll containers.
- Stores compose focused action groups; split lifecycle, persistence, preview,
  selection and deterministic reducers when ownership grows. Group broad component
  props by responsibility or use slots.
- Keep effect dependencies honest. Effects synchronize external systems; pure
  derivations stay outside effects. Do not hide values in refs or silence dependency
  warnings to avoid reruns.
- Preserve draft edits across async saves and distinguish current request/session
  ownership. Event subscriptions and in-flight IPC reads need cleanup and stale-result
  protection across navigation, recording changes and privacy invalidation.
- Typed clients own Tauri IPC. UI MUST NOT query SQLite or invoke FFmpeg/native/model
  runtimes directly. Treat external input as unknown until validated; TypeScript
  assertions do not validate JSON or model output.
- Keep shared contracts grouped by domain under `packages/shared`; compatibility
  barrels may preserve exports. Check frontend and Rust producers/consumers together.

## Performance and naming

- Compute playback/timeline view models once per stable input when repeated sorting,
  filtering or allocations are meaningful. Use indexes/maps/sets for repeated lookups
  in hot paths. Do not add `useMemo` without a real cost or stability boundary.
- Names describe responsibility: `*Screen` for routes, `*Layout` for layout-only
  composition, `*Panel` for side panels, `*Controls` for actions and
  `use*Controller` for orchestration.
- Split by ownership before adding behavior to a complex file; line count alone is
  not the decision. Keep React hooks focused instead of mixing loading, timers,
  media refs, derived state, mutations and error recovery in one hook.

## Visible behavior and tests

- Before UI work, identify the primary workflow and visible result, plus relevant
  loading, empty, error, disabled and success states. Cover meaningful combinations:
  saved/dirty, active lifecycle, unavailable permissions/tools and retry/recovery.
- Interactive changes MUST include Browser Mode or equivalent user-level coverage
  of a critical workflow, or document why that layer is impractical. Use actual
  clicks, typing, selection, navigation and visible assertions. Internal store
  assertions support workflow proof; they do not replace it.
- Critical controls need accessible names and keyboard reachability. Pointer-only
  canvas handles are an exception. Disabled primary actions expose the blocking
  condition nearby.
- Check layout changes at desktop and narrow viewports; preserve critical labels
  and clear primary actions. Browser tests prove mocked IPC workflows; real portal,
  WebKit, media devices and packaged behavior need separate native/manual evidence.
- Turn established/manual bugs into focused regression coverage before or alongside
  the fix. If automation is impractical, state the trigger, manual check and residual
  risk. Mock only real external boundaries, use synthetic private data and prefer
  accessible queries over implementation details.
- Unit tests own deterministic math, mapping, reducers, validation and transitions.
  Split large tests by workflow. Do not snapshot incidental classes or authored copy.

Exact commands and resource limits: [validation map](../../../docs/agent-validation.md).
Workflow checklist: [UI runbook](../../../.agents/runbooks/ui-quality-and-scenario-testing.md).
