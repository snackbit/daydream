# Agent Runbooks

Runbooks are optional repo-specific recipes for recurring agent scenarios. Use
this directory before broad repo analysis when the task explicitly matches one
of these workflows.

Runbook-driven work should end with concrete completion proof, not only a code
summary. Name the relevant tests, manual checks, contract/doc updates, and
residual risks. Omit categories that do not apply.

When host-native diagnostic commands fail in ways that look sandbox-related,
agents should prefer one safe elevated retry before treating the output as
evidence about the host.

## Index

### [Validation map](../../docs/agent-validation.md)

Use for exact scoped commands, native/browser prerequisites, dependency checks
and shared resource limits. Keep command details in that canonical map.

### [Planning](./planning.md)

Use for explicitly requested implementation plans.

### [UI Quality and Scenario Testing](./ui-quality-and-scenario-testing.md)

Use when adding or reviewing user-facing UI, stateful workflows, or fixes for
bugs found during manual verification.

## Authoring New Runbooks

Prefer updating an existing runbook when the scenario already exists. Add a new
runbook only when the workflow is distinct enough to deserve its own entry.
Keep runbooks concrete, repo-specific, and free of stale product assumptions.
