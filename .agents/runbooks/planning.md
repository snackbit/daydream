# Planning

Use this runbook when the user requests an implementation plan. Routine fixes
can proceed without a planning document.

Inspect the current code, callers, tests and repository instructions first.
Separate current behavior from proposed behavior. Resolve product tradeoffs with
the user when they affect scope or acceptance.

A plan should state the intended outcome, owning boundaries, implementation steps
and validation. Include privacy, cancellation, compatibility, storage migrations,
native permissions and packaging where the change affects them. Identify concrete
failure and recovery scenarios.

Keep decisions and working state in the conversation. Create a plan file only
when requested; no milestone template or scratchpad pair is required.

Use the [validation map](../../docs/agent-validation.md) for exact commands and
resource limits. Report checks actually run and remaining limitations. Remove
resolved maintenance entries rather than accumulating completion history.
