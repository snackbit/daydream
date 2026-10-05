# Master review: 2026-10-05

Immutable baseline: clean main `6527f5fb14435f52f2bd880dde0e9635eaa50506`.
Scope: entire current repository, divided into native recording, processing/storage/
privacy and frontend workflows. Planned day-index work was excluded. Read-only
review ended before implementation, dependency installs or native/browser checks.

The baseline has 14 confirmed findings: seven High and seven Medium. Original
evidence remains in each partition report; fixes and checks are recorded separately.

| ID | Priority | Category | Finding | Baseline report | Fix status |
| --- | --- | --- | --- | --- | --- |
| N1 | High | Security | Protection persistence/materialization fails open. | [Native](master-native.md) | Verified |
| N2 | High | Security | Unknown window metadata looks like a reliable nonmatch. | [Native](master-native.md) | Verified |
| N3 | High | Logical flaws/bugs | Active capture/writer failure is not supervised. | [Native](master-native.md) | Verified |
| N4 | Medium | Documentation/scope alignment | Changing rule IDs interrupts continuous protection. | [Native](master-native.md) | Verified |
| N5 | Medium | Documentation/scope alignment | User lifecycle controls retain automatic resume authority. | [Native](master-native.md) | Verified |
| P1 | High | Security | Playback/redaction source IDs differ from derived visibility IDs. | [Processing](master-processing.md) | Verified |
| P2 | High | Security | Changed redactions are not rechecked by queued/active processing. | [Processing](master-processing.md) | Verified |
| P3 | High | Security | VLM context reads newly redacted raw text. | [Processing](master-processing.md) | Verified |
| P4 | Medium | Logical flaws/bugs | Restart during pause/cancel strands running chunks. | [Processing](master-processing.md) | Verified |
| P5 | Medium | Security | Recording deletion leaves extracted processing artifacts. | [Processing](master-processing.md) | Verified |
| F1 | High | Security | Inspector retains cached private text after redaction. | [Frontend](master-frontend.md) | Verified |
| F2 | Medium | Logical flaws/bugs | Capture route returns with stale automatic recording state. | [Frontend](master-frontend.md) | Verified |
| F3 | Medium | Logical flaws/bugs | Async saves overwrite newer drafts and overlap ownership. | [Frontend](master-frontend.md) | Verified |
| F4 | Medium | Code quality/structure | Playback construction repeatedly sorts the entire frame set. | [Frontend](master-frontend.md) | Verified |

## Verification and limits

Baseline lint/typecheck passed; desktop unit project passed 18 files/151 tests.
Baseline browser/native tests, real capture, packaged artifacts and dependency
audit execution were not run. Partition reports describe inspected and limited areas.

Frontend fixes passed 50 focused unit and 95 focused browser tests, then the final
JavaScript gate (290 tests, lint/typecheck) and production build. Native/processing
ownership checks passed. Final native verification passed 349 tests, formatting and
strict all-target Clippy. The final bounded cancellation/Clippy refinements additionally
passed 94 focused AI/transcription tests (71/23). Processing fix evidence is in
[the separate fix report](master-processing-fixes.md).

Dependency security assessment is separate from passing tests: the Yarn graph has
zero reported security advisories and an ESLint 9 deprecation hold. Native OSV still
reports GLib 0.18 VariantStrIter unsoundness and proc-macro-error maintenance debt;
[research](../agent-tooling-research.md) gives precise constraints and limits.

Completion requires owning regression scenarios and coordinated relevant gates,
with failures/manual limits stated. Do not rewrite original findings as though
they were absent at the baseline.

[Active cleanup](../refactoring-tasks.md) records ownership splits.
[Validation](../agent-validation.md) defines commands and budgets.
[Agent-tooling research](../agent-tooling-research.md) records instruction/dependency
improvements and primary-source provenance.
