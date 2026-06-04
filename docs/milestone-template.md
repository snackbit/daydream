# Milestone Template

This document is the canonical recipe for optional milestone artifacts in this
repository.

Every new milestone SHOULD follow this format unless there is a concrete reason
to deviate. If a section does not apply, omit it cleanly instead of inventing a
replacement shape.

## Purpose

The milestone pair exists to do two different jobs:

- The milestone doc is the contract.
- The scratchpad is the live notebook.

The milestone doc should make scope, constraints, gates, validation, and
completion bar obvious to a new reader. The scratchpad should capture decisions,
evidence, open questions, implementation notes, validation logs, and follow-up
items without turning the main doc into a log dump.

Milestones are optional. Use them only when the user explicitly asks for
milestone-scoped work or approves a milestone-style plan.

## Artifact Pair

Every milestone MUST have both files:

- `docs/milestones/m<nn>-<slug>.md`
- `docs/milestones/m<nn>-<slug>-scratchpad.md`

Recommended naming:

- `m1-capture-foundation.md`
- `m1-capture-foundation-scratchpad.md`

## Readiness Bar

A milestone doc is an implementation-ready contract, not a mixed
planning/execution artifact.

Before a milestone is approved for execution:

- all material product, privacy, interface, storage, processing, native
  permission, scope, and boundary decisions MUST already be locked in the
  milestone body
- unresolved design or validation questions MUST be handled during planning or
  called out as blockers before approval
- gates MUST describe deliverables and validation work, not meta work about
  figuring out what to build

## Gold Standard Rules

- Start with `Goal`, `Summary`, `Locked Decisions`, `Scope`, and `Non-Goals`.
- Add domain contract sections only when they materially reduce ambiguity.
- Use gates with checkbox tasks. Gates are the execution plan.
- Give every gate a clear outcome-oriented title.
- Add `Exit Criteria:` under a gate when the gate is broad or easy to interpret
  loosely.
- Keep tasks concrete, testable, and reviewable. Avoid vague checklist items
  like "finish backend" or "improve UX".
- Milestones MUST be decision-complete before execution starts. If something
  still needs to be defined, confirmed, or decided, that is a planning blocker,
  not a gate task.
- Gate tasks SHOULD read like issue deliverables: add, implement, update,
  remove, generate, validate, verify.
- Gate tasks MUST NOT use planning/meta verbs such as `define`, `confirm`,
  `decide`, `investigate`, or `figure out` unless the documentation itself is a
  shipped deliverable.
- Separate implementation work from final validation. Use a dedicated
  `Testing and Validation` section and/or final validation gate.
- Keep rollout, runbook, migration, native permission, model/tool, or operator
  guidance in its own section when it matters.
- Put evolving notes, dead ends, benchmark findings, and open questions in the
  scratchpad, not in the main gate checklist.
- Reconcile completed gates against upcoming gates. If implementation changes
  downstream deliverables, contracts, stop boundaries, validation, success
  criteria, risks, or assumptions, update the milestone doc before calling the
  gate complete.
- Update checklist state as work completes. Do not leave completed work
  unticked.

## Standard Section Order

Use this order for the milestone doc:

1. `# Milestone M<nn>: <Title>`
2. `## Goal`
3. `## Summary`
4. `## Inputs` (optional)
5. `## Locked Decisions`
6. `## Success Criteria` (optional but recommended for large/product-facing work)
7. `## Scope`
8. `## Non-Goals`
9. Domain-specific contract sections as needed, for example:
   - `## Definitions and Ownership Rules`
   - `## Architectural Invariants`
   - `## Public API / Interface / Config Changes`
   - `## Native Permission Requirements`
   - `## Privacy and Deletion Contract`
   - `## Storage Format`
   - `## Processing Contract`
10. `## Gate A: ...` onward
11. `## Testing and Validation`
12. `## Dependencies and Risks` or `## Risks and Edge Cases`
13. `## Assumptions`
14. `## Expected Deliverables` or `## Operability Runbook` when useful

Not every milestone needs every optional section, but the order should stay
stable.

## Gate Writing Rules

- One gate should own one coherent stage of work.
- Keep gate names directional, not generic:
  - good: `Gate C: Rust Capture Command Contract`
  - bad: `Gate C: Backend`
- Checklists should describe observable outcomes.
- If a checklist item cannot be proven done by a code diff, generated artifact,
  or validation result, it probably belongs in planning or the scratchpad
  instead of the gate.
- Contract shapes, privacy rules, permission requirements, and behavior
  expectations belong in `Locked Decisions`, interface/invariant sections, or
  `Assumptions`, then the gate implements and validates them.
- Nest sub-checklists only when they sharpen the contract.
- If a gate has a special stop boundary, say it explicitly as an execution
  boundary, not as a meta reminder.
- If a gate requires manual review, say what must be verified.
- If a gate changes contracts, permissions, storage, deletion, processing
  eligibility, or state transitions, it should include at least one explicit
  testing task somewhere in the milestone.
- Completed gate checklists SHOULD NOT be rewritten except for checkbox state or
  narrowly needed corrections. Downstream changes belong in upcoming gates,
  contract sections, risks, validation, or assumptions.

## Gate Feedback Rules

Gate implementation can materially change later gates. Treat downstream
correction as part of gate implementation, not optional cleanup.

- At gate closeout, compare implementation decisions, new constraints, and
  discovered facts against upcoming gates, contract sections, risks, validation
  requirements, success criteria, and assumptions.
- Update the milestone doc when a scratchpad finding changes future
  implementation requirements.
- Keep evidence and rationale in the scratchpad, but keep contract-impacting
  changes in the milestone doc.
- If no downstream changes are needed, record that result in the scratchpad.

## Stop Boundaries

Use explicit stop-boundary wording when a milestone crosses a known handoff
line:

- Database migrations:
  backend schema edits MAY proceed first, but the milestone MUST say whether
  migration files are expected in the same gate and which command validates
  them.
- Generated artifacts:
  if code generation is required, the milestone MUST name the source artifact,
  generated output, command, and whether generation is expected to happen
  automatically or manually.
- Native permissions or packaged binaries:
  if OS permissions or bundled tools such as FFmpeg are involved, the milestone
  MUST name the expected platform behavior and validation boundary before
  implementation starts.

## Scratchpad Rules

The scratchpad is not a second milestone doc. It should capture live planning
and execution context that would otherwise clutter the main artifact.

Good scratchpad content:

- locked decisions copied or refined from the milestone
- research inputs and reference links
- baseline evidence and inventories
- terminology decisions
- open questions and risk watchlists
- gate notes and implementation findings
- gate reconciliation notes
- validation log
- deferred follow-up candidates

Bad scratchpad content:

- duplicated full gate checklists
- long changelog-style file inventories unless needed for a decision
- generic narration with no future value
- meta reminders that should have been encoded as milestone decisions, gate
  deliverables, or explicit stop boundaries

## Milestone Doc Template

```md
# Milestone M<nn>: <Title>

## Goal

<One paragraph. State the end result and why it matters.>

## Summary

- <What this milestone changes>
- <What it does not try to solve indirectly>
- <What makes this the right milestone boundary>

## Inputs

- <research doc / prior milestone / source-of-truth file>
- <relevant implementation surfaces>

## Locked Decisions

- Milestone artifacts:
  - `docs/milestones/m<nn>-<slug>.md`
  - `docs/milestones/m<nn>-<slug>-scratchpad.md`
- <decision that should not drift during implementation>
- <explicit constraint or tradeoff>

## Success Criteria

The milestone is successful when:
- <reader/operator/user-visible outcome>
- <technical outcome>
- <validation/completion outcome>

## Scope

- <in-scope area>
- <in-scope area>
- <in-scope area>

## Non-Goals

- <explicitly out of scope>
- <explicitly out of scope>
- <explicitly out of scope>

## <Domain Contract Section>

- <definitions, invariants, interface changes, target architecture, etc.>

## Gate A: <Outcome-Oriented Name>

- [ ] <implementation deliverable based on a locked contract>
- [ ] <implementation deliverable based on a locked contract>
- [ ] <validation or migration/generation task when applicable>

Exit Criteria:
- <what must be true to call this gate done>
- <what must be true to call this gate done>

## Gate B: <Outcome-Oriented Name>

- [ ] <implementation deliverable>
- [ ] <implementation deliverable>
- [ ] <implementation deliverable or explicit stop-boundary step>

Exit Criteria:
- <what must be true to call this gate done>

## Gate <Final>: Validation and Launch Readiness

- [ ] <targeted lint/test/build/manual validation task>
- [ ] <manual verification task>
- [ ] <scratchpad and follow-up note update>

Exit Criteria:
- <validation passed in a real run>
- <residual risks are documented>

## Testing and Validation

Required validation:
- `COREPACK_HOME=/tmp/corepack corepack yarn lint`
- `COREPACK_HOME=/tmp/corepack corepack yarn typecheck`
- `COREPACK_HOME=/tmp/corepack corepack yarn test`
- `cargo test --manifest-path apps/desktop/src-tauri/Cargo.toml` when Rust/Tauri changes are in scope and local prerequisites are installed
- manual verification:
  - <critical path>
  - <negative path>

If the implementation changes contracts, permissions, storage, deletion, or
state transitions, add or update the relevant tests in the correct layer.

## Dependencies and Risks

- <dependency or ordering risk>
- <external input risk>
- <privacy or native permission risk>

## Assumptions

- <assumption>

## Expected Deliverables

- <artifact>
- <artifact>
```

## Scratchpad Template

```md
# Milestone M<nn>: <Title> Scratchpad

## Current Status

- <planning / in progress / blocked / validating / complete>

## Locked Decisions

- <decision copied or refined from the milestone>

## Findings

- <evidence, references, inventories, or implementation notes>

## Gate Reconciliation Log

- Gate <X>: <downstream sections reviewed; milestone doc changes made, or no
  downstream changes needed>

## Validation Log

- `<command>`: <result>

## Follow-Up Candidates

- <deferred item with reason>
```
