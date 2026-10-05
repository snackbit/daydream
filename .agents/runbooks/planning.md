# Planning

## Purpose

Use this for milestone-scoped work. It keeps milestone workflow rules in one
place and makes feature completeness include code, docs, privacy, storage,
native permission, packaging, and validation surfaces when they matter.

This workflow is optional. Do not use it for ordinary implementation work unless
the user explicitly mentions a milestone, gate, scratchpad, or approved
milestone-style plan.

## Use When

- Creating, refining, executing, reviewing, or closing a milestone
- Reviewing whether a milestone is implementation-ready
- Planning work that requires several coordinated gates, native permissions,
  storage/migration decisions, model/tool choices, packaging decisions, or
  privacy-sensitive deletion behavior

## Sources of Truth

- `docs/milestone-template.md`
- `docs/milestones/*.md`
- `docs/milestones/*-scratchpad.md`
- `AGENTS.md`
- `README.md`

## Required Steps

1. Choose the work mode up front.
   - If the developer requests normal implementation work without mentioning a
     milestone, treat it as ad-hoc work.
   - If the developer refers to a milestone, gate, scratchpad, or approved
     milestone-style plan, treat it as milestone-scoped work.

2. Make milestone plans implementation-ready before approval.
   - Lock product behavior, privacy rules, IPC/API contracts, storage formats,
     native permission expectations, scope boundaries, stop boundaries, and
     rollout assumptions in the milestone body.
   - If unresolved planning work remains, refine the milestone first.

3. Use the milestone artifact pair.
   - Every milestone must have `docs/milestones/<milestone-name>.md` and
     `docs/milestones/<milestone-name>-scratchpad.md`.
   - Use `docs/milestone-template.md` as the canonical template.

4. Write gates as deliverables, not planning reminders.
   - Gate checklist items must describe concrete implementation or validation
     outcomes.
   - Avoid meta tasks such as `define`, `confirm`, `decide`, `investigate`, or
     `figure out` unless documentation is the deliverable.

5. Keep the scratchpad as the live notebook.
   - Store decisions, evidence, findings, validation notes, blockers, and
     follow-up candidates there.
   - Do not duplicate the full milestone checklist.

6. Treat privacy and local-first behavior as milestone completeness.
   - For capture, processing, indexing, search, digest, deletion, or retention
     work, explicitly account for sensitive data handling and redaction
     eligibility before the milestone is implementation-ready.
   - Do not process, persist, log, upload, or summarize private captured data in
     ways that conflict with `AGENTS.md`.

7. Treat config and packaging completeness as milestone completeness when
   relevant.
   - Explicitly identify checked-in config, env vars, native binaries, OS
     permissions, Tauri capability/config changes, database migrations, and docs
     needed to actually ship.

8. Follow stop boundaries.
   - For database migrations, generated artifacts, native permissions, and
     packaged binaries, follow the stop-boundary wording in the active
     milestone.
   - If the active milestone lacks a required stop boundary, refine the
     milestone before crossing that boundary.

9. Reconcile downstream gates at gate closeout.
   - Before calling a gate complete, compare the gate's implementation
     decisions, new constraints, and discovered facts against all upcoming
     gates, milestone contract sections, risks, validation requirements,
     success criteria, and assumptions.
   - Update the milestone doc when future gate deliverables, contracts, stop
     boundaries, validation requirements, or success criteria are now
     inaccurate.
   - Keep detailed evidence and rationale in the scratchpad.
   - Do not hide contract-impacting changes only in scratchpad notes.
   - If no downstream changes are needed, record that explicitly in the
     scratchpad.

10. Validate at the correct layer.
   - Use focused tests for the behavior changed.
   - Use the [validation map](../../docs/agent-validation.md) for scoped commands,
     final gates and shared resource limits. Run full gates once after edits settle.
   - Include negative or failure-path validation for privacy-sensitive,
     stateful, native, or user-facing behavior.
   - Treat sandbox-sensitive native diagnostics carefully. If a command that
     should normally see host hardware, devices, sessions, windows, displays,
     services, or drivers fails with missing resources, permission errors,
     inaccessible devices, or a crash that looks sandbox-related, do not treat
     the first result as host truth. Rerun the same safe diagnostic command with
     elevated permissions before drawing conclusions.
   - Do not elevate destructive commands, commands that mutate project state,
     or commands that could expose private captured content. If elevated
     validation is unsafe, record the limitation explicitly.

11. State completion proof before closeout.
   - Before marking a gate or scoped task complete, briefly state what proves
     completion.
   - Include only relevant proof categories: automated tests, manual checks,
     contract or API compatibility, documentation, privacy or security review,
     migrations or config, and downstream impact.
   - If a category does not apply, omit it. Do not add boilerplate proof.

## Validation

- Confirm the milestone doc is decision-complete before execution starts.
- Confirm gate checklist items are implementation or validation outcomes.
- Confirm the scratchpad reflects current findings and validation evidence.
- Confirm completed gates reconciled implementation findings against downstream
  gates and updated the milestone doc where future work changed.
- Confirm required privacy, config, storage, packaging, and native-permission
  surfaces were reviewed when runtime assumptions change.
- Confirm sandbox-sensitive native diagnostics were retried with safe elevation
  before using failure output as evidence, or that the limitation was recorded.
- Confirm closeout notes identify concrete completion proof instead of only
  summarizing code changes.
- Confirm completion notes do not claim done while required wiring is still
  pending.

## Pitfalls

- Treating ordinary ad-hoc work as milestone work without an explicit user cue.
- Treating code-complete as ship-complete when config, migration, packaging, or
  native permission wiring is still missing.
- Starting milestone implementation before the milestone doc and scratchpad are
  current.
- Hiding important privacy, scope, storage, or permission rules in scratchpad
  notes instead of the milestone body.
- Completing a gate without correcting later gates after implementation changed
  their assumptions, contracts, or validation requirements.
- Reporting completion without naming the evidence that makes the claim true.
- Treating sandbox-limited hardware, display, session, service, or driver
  command failures as host facts without a safe elevated retry.
