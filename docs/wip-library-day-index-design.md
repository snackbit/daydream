# WIP Design: Library as a Private Day Index

## Status

This is a WIP product direction document. It is ground truth for the direction
Daydream is stepping toward, but it is not an implementation milestone and does
not override milestone task scope by itself.

## Product Thesis

Daydream should not primarily feel like a recording library. It should feel like
a private day index: a local, source-linked memory of what happened during a
working day.

Raw playback is still essential, but it is not the core product experience. The
playback studio is the receipt: the place where users inspect evidence, verify
AI output, redact sensitive spans, and debug processing. The default user value
is the intelligence layer that answers:

- What happened today?
- What did I spend time on?
- What needs follow-up?
- Where is the evidence?

The north star:

> Daydream turns a private working-day recording into a source-linked memory
> index. It shows how time was spent, identifies meaningful work sessions,
> extracts follow-ups, and lets the user ask questions about the day while
> always being able to jump back to exact evidence.

## Library Model

The Library should use one underlying local index, but allow multiple user
projections over that index. Users should be able to choose the organization
that matches how they think about their work.

Supported projections:

- **Days:** the recommended default for daily review.
- **Recordings:** source/media management and lower-level inspection.
- **Sessions:** AI-derived blocks such as meetings, coding, chat, research,
  entertainment, AFK, or admin work.
- **Segments:** low-level recording chunks for inspection, processing status,
  redaction, and debugging.

This means "Library" should eventually become less about files and more about
browsing indexed time. Recordings remain the durable source assets behind the
experience.

Library list previews should be private by default. They should show metadata,
coarse status, and high-level labels without exposing raw transcript, OCR, or
private snippets until the user opens a detail view. A later setting may control
preview density.

## Default Detail View

Opening a day or recording should default to a summary dashboard, not raw
playback.

Recommended top-level detail structure:

1. **Overview**
   - Total recorded time.
   - Active, idle, AFK, and unknown time.
   - Time by inferred category, such as focused work, meetings, chatting,
     research, browsing, entertainment, and admin.
   - Processing status, redaction status, and confidence/unknown coverage.

2. **Timeline of Sessions**
   - AI-derived blocks such as:
     - `08:30-08:45 Chat with Julian about his Turkey flight`
     - `11:00-11:30 Meeting: roadmap planning`
     - `13:10-14:25 Coding: Library activity lane`
   - Each session should include source links, involved apps/windows, supporting
     transcript/OCR snippets, confidence, and gaps or uncertainty.

3. **Follow-Ups**
   - Suggested todos.
   - Promised replies.
   - Meeting action items.
   - Calendar or reminder candidates.
   - Creation actions must be explicit user actions. Daydream should suggest
     with evidence, not silently create tasks or calendar entries.

4. **Evidence**
   - Playback studio.
   - Transcript.
   - OCR and frame summaries.
   - Activity lanes.
   - Redactions.

5. **Ask**
   - Chat/search over the indexed day.
   - Answers should cite source spans.
   - Follow-up creation should be available from cited answers.

## Playback and Evidence Role

The playback studio should remain the source viewer and editing surface. It is
where users verify and correct the AI-prepared view.

Evidence should be rendered as time-aligned lanes rather than one giant text
document:

- Video/source media.
- Desktop audio and microphone audio.
- Transcript per audio source.
- OCR text.
- VLM/frame observations.
- Activity.
- Window/app focus.
- Redactions.

Transcript, OCR, and visual observations are evidence lanes. They should not be
the default product summary. The summary dashboard and session timeline should
consume those lanes and link back to them.

## Activities Lane

The Activities lane should show attention and interaction clues. It should not
capture sensitive content beyond what is explicitly needed.

Initial activity sources:

- Keyboard activity timing only. Do not capture key values or typed text.
- Mouse movement, click, and scroll timing only. Keep it coarse.
- Frame-delta activity.
- Merged active, idle, AFK, and unknown intervals.

Future event sources to consider:

- Focused app/window changes.
- Focused window title changes, treated as sensitive metadata.
- Open window inventory snapshots.
- Screen lock/unlock.
- Sleep/wake.
- Display attach/detach or monitor topology changes.
- Audio speech/silence by source.
- Meeting or call state where detectable.
- Browser page title or URL domain only if explicitly enabled.
- Media playback indicators where detectable.
- Manual user markers such as private, break, meeting, deep work, or ignore.
- Calendar context if the user connects a calendar later.

Clipboard events should not be captured by default. If ever added, the default
should be event timing only, not clipboard content.

Activities are clues, not the final UX. The user-facing output should be
derived sessions, summaries, follow-ups, and answerable questions. The raw
activity lane exists for trust, correction, redaction, and debugging.

## Derived Sessions

A derived session is a time-bounded interpretation of user activity. It should
combine multiple evidence sources:

- Transcript segments.
- OCR regions.
- VLM/frame observations.
- Activity intervals.
- Focused app/window metadata.
- Recording segment boundaries.
- User redactions and manual labels.

Examples:

- Meeting with notes and follow-ups.
- Chat with a person about a topic.
- Focused coding session in a repository.
- Research session across browser/document windows.
- Entertainment or break.
- AFK/idle span.
- Unknown or low-confidence span.

Every derived session must keep provenance. It must be possible to jump from a
session label, summary, or follow-up back to the source spans that support it.
Low-confidence or mixed sessions should be shown as uncertain instead of
overstated.

## Assistant Behavior

Daydream should be assertive enough to be useful, but not autonomous by default.

Default posture:

- Suggest labels, summaries, todos, reminders, and calendar entries.
- Show source evidence for generated claims.
- Ask for explicit confirmation before creating or exporting anything.
- Preserve uncertainty when evidence is incomplete or conflicting.
- Allow users to correct labels so later summaries can improve.

The chat interface should answer questions over the local index, not over raw
files directly. Answers should cite source spans and expose actions only when
the supporting evidence is available.

Example questions:

- "Are there follow-ups from the roadmap meeting?"
- "What did I work on after lunch?"
- "When did I talk with Julian about his flight?"
- "Show me the source for this todo."
- "What did I spend the most focus time on today?"

## Privacy Rules

The day index contains highly sensitive derived data. Library UX must preserve
privacy by default.

Required rules:

- Do not show raw transcript, OCR text, window titles, prompts, or model output
  in Library list cards by default.
- Do not upload recordings, transcripts, OCR, embeddings, or summaries unless a
  remote provider path is explicitly configured and labeled.
- Do not log raw transcript, OCR, prompts, window titles, or model responses by
  default.
- Redaction must invalidate or remove overlapping derived sessions, transcript,
  OCR, VLM observations, embeddings, summaries, and search results.
- Summaries and follow-ups must not become orphaned private content after their
  source spans are cut or hidden.
- Unknown or unsupported activity capture should render as unknown, not fake
  idle or active time.

## UI Direction

Near-term UI should move toward this hierarchy:

- Library overview: configurable projection selector and private previews.
- Detail default: summary dashboard for the selected day/recording/session.
- Evidence tab: current playback studio with timeline lanes and inspector.
- Ask tab: chat/search over the selected indexed scope.
- Redaction remains globally important and must be reachable from evidence and
  source-linked generated items.

The summary dashboard should avoid marketing-style cards and decorative
visuals. It should feel like an operational review surface: dense, scannable,
source-linked, and quiet.

Useful dashboard elements:

- Time allocation chart with unknown/low-confidence time visible.
- Session list with timestamps, labels, evidence count, confidence, and source
  jump actions.
- Follow-up list with source span, suggested action type, and confirmation
  controls.
- Processing coverage indicator for transcript, OCR, activity, VLM, summary,
  and index readiness.
- Redaction impact indicator when generated items overlap hidden or cut spans.

## Open Product Questions

- Should the default Library projection be a user setting, a per-device setting,
  or a remembered local UI preference?
- Should a "day" span midnight-to-midnight, user-configured workday hours, or a
  recording cluster?
- How should user corrections to AI-derived session labels feed future local
  classification?
- What confidence model is simple enough to show without pretending precision?
- Which external actions should ship first: todos, calendar entries, reminders,
  or exportable meeting notes?

## Current Implementation Fit

The current app already has a recording-centric Library and a playback studio.
That should not be thrown away. It should become the Evidence surface behind the
larger day-index product model.

Near-term implementation should continue improving playback, transcript,
activity, OCR, redaction, and source provenance. The product direction above
explains why those evidence layers matter: they are inputs to the day index,
not the final user experience by themselves.
