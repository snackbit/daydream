# UI Quality and Scenario Testing

## Purpose

Use this when adding, changing, or reviewing user-facing UI, workflow behavior,
stateful product flows, or fixes for bugs found during manual verification.

The goal is to make agents prove that real workflows survive normal use,
failure states, and meaningful combinations instead of only adding narrow green
unit tests.

## Use When

- Building or changing React UI in `apps/desktop`
- Changing capture, processing, search, digest, redaction, settings, or
  diagnostics workflows
- Adding stateful behavior that crosses stores, Tauri commands, SQLite, native
  capture, filesystem storage, model runners, or privacy-sensitive data
- Fixing a bug found by manual verification
- Reviewing whether a UI or workflow change is actually complete

## Sources of Truth

- `AGENTS.md`
- `apps/desktop/src/components/ui`
- `apps/desktop/components.json`
- Nearby route, screen, store, and client tests
- shadcn docs when adding or choosing components

## Required Steps

1. Inspect before changing UI.
   - Check existing shadcn components in `apps/desktop/src/components/ui`.
   - Check nearby screens, feature components, stores, and tests.
   - Identify route/data boundaries and where Tauri/native/model/filesystem
     calls enter the UI.

2. Write a small risk matrix before or during implementation.
   - Name the primary workflow.
   - Name likely failure modes.
   - Pick the test layer for each risk: Browser Mode, store/unit, Rust/Tauri,
     or manual-only.
   - Note privacy/logging risks when sensitive data, prompts, transcripts,
     OCR, window titles, audio, frames, embeddings, or summaries are involved.

3. Test scenarios before implementation is considered complete.
   - Use Browser Mode or equivalent user-level tests for critical React
     workflows.
   - Use unit tests for deterministic logic, validation, data mapping, layout
     math, reducers, and store transitions.
   - Use Rust tests for Tauri command contracts, SQLite behavior, native
     lifecycle state, storage deletion, and process orchestration.
   - Use manual checks only for surfaces that cannot reasonably be automated
     in the current repo setup.

4. Cover meaningful workflow combinations.
   - Empty, populated, and persisted data.
   - Loading, success, error, retry, and recovery states.
   - Saved versus dirty state.
   - Permission or tool unavailable states.
   - Active lifecycle states such as idle, recording, paused, processing,
     failed, and complete when relevant.
   - Desktop and narrow viewport behavior for layout-changing UI.

5. Keep mocks honest.
   - Mock process, native, filesystem, model, network, or Tauri IPC boundaries.
   - Do not mock away the component, store transition, or domain behavior that
     the test claims to validate.
   - Prefer visible UI assertions and accessible queries over implementation
     details. Internal state assertions may support a test but should not be
     the main proof of a UI workflow.

6. Turn manual failures into regression tests.
   - If the operator or agent finds a bug manually, add a failing regression
     test before or alongside the fix.
   - If the bug cannot be automated, document the exact manual verification and
     residual risk in the final handoff or milestone scratchpad.

7. Validate UX basics.
   - Critical controls have accessible names.
   - Keyboard use works for normal controls unless the control is genuinely
     pointer-only.
   - Disabled primary actions expose the blocking condition through visible
     status, copy, or error state.
   - Text does not overlap or hide critical labels on checked viewports.
   - Existing shadcn/Tailwind patterns are used instead of hand-rolled
     equivalents when available.

## Handoff Requirements

- Summarize the scenario coverage added or updated.
- List commands run and whether they passed.
- List manual checks performed or explicitly skipped.
- Call out important untested risks, especially native, privacy, packaging, or
  manual-only behavior.

## Pitfalls

- Adding only unit tests for a feature whose main risk is a broken user
  workflow.
- Asserting Zustand or component internals when the user-visible behavior is
  available to assert.
- Mocking the exact behavior the test claims to prove.
- Testing only the happy path while ignoring dirty state, unavailable tools,
  permission failures, empty data, and retry paths.
- Treating a manual bug fix as complete without a regression test or explicit
  manual-only note.
