# Daydream

Daydream is a local-first desktop recall assistant. It helps you return to what
happened on your computer: the work you did, the conversation you want to revisit,
the information you saw, and the next step you meant to take.

Its starting question is simple: **What did I do today?**

Today, Daydream records selected screens, desktop and microphone audio, and
activity context into a private local library. You can replay a recording, inspect
transcripts and recognized screen text, and redact sensitive parts. The next step
is to turn that evidence into a searchable day index, with a useful overview of
your day, meaningful sessions, and follow-up suggestions. The longer-term vision
adds questions and answers that cite the moments they came from.

The ambition is to make your digital day easier to remember and pick up again.
You should be able to find something even when you forgot to bookmark it, write
it down, or remember which application contained it.

## The problem

A working day leaves traces across browser tabs, documents, editors, meetings,
messages, and terminals. You might remember discussing a decision without
remembering the wording. You might remember reading a useful explanation without
remembering the page. At the end of a busy day, it can be difficult to explain
what you worked on, where your time went, or what still needs attention.

Recovering that context often means checking several histories, searching
messages, reopening files, or relying on notes you may never have taken.

Daydream brings those traces into one time-based record that you control. Screen
and audio provide the original evidence. Transcription, screen text, and activity
context help you navigate it. The planned intelligence layer will organize that
evidence into a day you can review without watching hours of playback.

The value is practical: find the passage, revisit the decision, recover the
context, and continue your work.

## Who it is for

The intended audience is people whose work happens across many applications and
whose useful context is easy to lose between them. Developers, researchers,
designers, writers, and independent workers are natural examples. Someone who
alternates between focused work, research, and calls should recognize the problem
immediately.

The early audience also needs to be comfortable with a Linux developer alpha.
Daydream currently focuses on Linux, especially KDE Plasma on Wayland. It is open
source under AGPL-3.0-or-later, and there are no stable packaged releases yet.

The product should speak to the person reviewing their own day. Its measures of
activity should help that person understand context, with visible gaps and
uncertainty. The landing page should frame this as personal recall and user
ownership; productivity scoring and employee oversight are outside this brief.

## What exists today

Daydream already has the capture and evidence layer that the broader vision
depends on.

You choose monitor sources, crop or scale their output, and explicitly start,
pause, resume, and stop recording. New recordings keep each monitor in its own
video stream, alongside separate desktop and microphone audio tracks. Recording
state and controls remain visible while you move through the app.

The local Library includes playback, thumbnails, waveforms, and a studio with
time-aligned tracks. You can select a screen source or review synchronized
sources together. Transcripts, recognized screen text, visual observations, and
activity markers retain their source timestamps so you can inspect the moment
behind a result.

Processing can run during recording, after stopping, or manually. Its dashboard
shows progress and failures, with controls to pause, resume, cancel, and retry
work. Processing is bounded and resumable; an ordinary failure in one section
does not discard successful results from the others.

Local speech transcription uses whisper.cpp. Local screen-text recognition uses
Tesseract by default. Optional configured vision processing can recognize screen
text or produce visual observations. OpenAI-compatible remote transcription and
vision endpoints are available through explicit configuration.

Activity context includes keyboard and mouse timing, screen changes, and
supported window metadata. The activity collector does not record key values,
typed text, pointer coordinates, or clicked content. Screen recordings can still
contain whatever is visible on the selected screens, including text you type;
that distinction matters when explaining privacy.

Timeline redactions exclude or invalidate overlapping derived results.
Protected-window rules can pause capture or create automatic redactions.

These capabilities are implemented in a developer alpha. Real-device behavior,
permissions, desktop compatibility, and packaged-app testing still need work.
The landing page must keep that maturity visible.

## What Daydream is becoming next

The next planned product step is a **private day index**. A day becomes the main
unit of review, with recordings available as its supporting evidence.

### A useful overview of your day

Opening an indexed day should show what happened, how recorded time was spent,
and where the record is incomplete. The planned overview includes a day summary,
session timeline, time by category, processing status, and unknown coverage.

The purpose is to help you orient yourself quickly. You should be able to see
that the morning contained research and a planning call, while the afternoon
contained a substantial coding session, then open the part you care about.

Unknown time needs to remain visible. Missing permissions, incomplete processing,
or conflicting evidence should not produce confident activity labels.

### Meaningful sessions

Daydream plans to combine transcripts, screen text, visual observations, and
activity context into time-bounded sessions. A session might describe a meeting,
research on a topic, work in a repository, a conversation, or a break.

Those interpretations need supporting source links and visible uncertainty.
Users will be able to correct session labels, categories, and notes, with those
corrections preserved when generated results are rebuilt.

The Library will offer different ways to browse the same local index: days,
recordings, sessions, and individual segments. Daily review should be easy while
source inspection remains available.

### Search by wording and meaning

The planned search combines exact text matching with semantic retrieval. You
should be able to search for a phrase you remember or describe the subject when
you cannot recall the words.

Results should identify the evidence they matched and let you jump to the
relevant recording and timestamp. An example query might be “database migration
discussion” or “the article about keyboard accessibility.”

The first day-index milestone delivers search results. Generated conversational
answers with citations are a later step.

### Follow-ups you can review

Daydream plans to identify possible action items, promised replies, and other
follow-ups from recorded evidence. Each suggestion should explain its source so
you can decide whether it belongs on your list.

The near-term scope keeps these suggestions inside Daydream, where you can
accept or dismiss them. External task, calendar, and reminder integrations are
later possibilities and would require explicit user actions.

### A local index with model choice

The index is planned to live on your machine. Local text and vector retrieval
will support searching the derived evidence. Planned provider settings will let
users assign compatible models to specific jobs instead of tying the experience
to one hosted AI service.

For this next milestone, local language models and embeddings mean a
user-configured local endpoint. Daydream is not planning to bundle and manage a
complete local language-model runtime in that milestone. Remote providers remain
an explicit choice, with clear disclosure before private content is sent.

## The longer-term vision

Once the day index is dependable, Daydream can offer a conversational way to
retrieve context. You could ask what you worked on after lunch, which follow-ups
came out of a meeting, or where a particular decision was discussed.

An answer should cite its sources and open the relevant evidence. If a recording
only supports part of an answer, the response should say so. Users need a way to
check a claim without trusting a fluent summary on its own.

Later possibilities include memory across multiple days, visual similarity
search, and explicit export or task-creation workflows. Broader platform support
is also future work. These are directions for exploration, with no release dates
or availability promises implied by this document.

The desired experience is continuity: return to a piece of work with enough
context to understand what happened and decide what to do next.

## Privacy and trust shape the product

Daydream handles unusually sensitive material. Screens and microphones can
capture personal messages, confidential documents, and conversations. Derived
summaries, window titles, and search results can be sensitive too.

The default architecture keeps captured content on the local machine. Local
transcription and local OCR are available. Remote AI capabilities require
explicit configuration and can receive private content when enabled. “Local
first” must be explained through those actual choices.

Recording stays visible and under the user's control. Privacy controls belong in
the core experience, close to capture and evidence review.

Redactions affect processing and derived results. As the index expands, that
same requirement must cover summaries, sessions, follow-ups, and search. A
sensitive moment should not reappear through an old generated result.

Reversible redactions currently mask and exclude spans; they do not physically
erase the original media. Physical deletion and downstream cleanup have their
own requirements and remaining work. Landing-page copy must preserve that
distinction.

Source links are another part of trust. A useful summary should let you inspect
the recording behind it. A proposed follow-up should show the evidence that led
to it. User corrections should survive regeneration.

## The feeling the product should convey

Daydream should feel personal, calm, and capable. You are returning to your own
context, at your own pace. The emotional promise is relief from having to keep
every useful detail in your head.

The name gives room for a softer visual identity, with warmth and a sense of
time passing. The interface still needs precision: readable timestamps, clear
source links, distinct recording states, and useful evidence views.

A landing page can be more expressive than the desktop interface. It should
make the product recognizable through a convincing example of a day and the
ability to revisit a moment within it.

## Brief for the landing-page design agent

Design a landing page for Daydream using this document as product context. Lead
with the human benefit of remembering and resuming your work. Explain the
capture and evidence layer, then show the private day index the product is
moving toward. Preserve the difference between implemented alpha capabilities,
the next planned milestone, and longer-term possibilities.

Use these as copy seeds, not mandatory final headlines:

- “Return to the moments that matter.”
- “A private memory of your working day.”
- “Find what you saw. Remember what you meant to do.”

A possible supporting line is: “Daydream records the screens and audio you
choose, keeps them on your machine by default, and is building a searchable view
of your day with links back to the original moments.”

Build the visual story around one believable synthetic working day. For example,
show a research session, a planning call, and an afternoon coding session. Let a
visitor see how a session or search result connects to a timestamp, transcript
excerpt, or frame in the evidence studio. Any dashboard, semantic-search, or
follow-up mockup needs a visible label identifying it as a planned experience.

The page should answer these questions in a clear reading order:

1. What will Daydream help me remember or recover?
2. What does it record, and how do I control recording?
3. What can I inspect in the alpha today?
4. What will the private day index add?
5. Where does my data go, and what happens if I enable remote AI?
6. How can I explore or contribute to the project now?

Show privacy through concrete behavior: local storage, optional remote
providers, visible recording controls, protected-window rules, and
redaction-aware processing. Give source-linked evidence enough visual prominence
that visitors understand how they can verify a generated interpretation.

Choose typography, color, composition, and motion that suit the name and support
the explanation. Keep the page readable on mobile, preserve contrast, and respect
reduced-motion preferences. Technical details can sit below the main product
story, where interested early adopters can find them.

Use a repository link or developer getting-started action as the current call to
action. The project lives at <https://github.com/snackbit/daydream>. State
“Linux developer alpha” clearly. A stable-download action needs an actual
packaged release before it can be presented as available.

Use invented names, documents, and conversations in design fixtures. Keep
customer counts, testimonials, benchmarks, pricing, launch dates, and integration
claims out of the page unless separately supplied and verified. Broad claims such
as “records everything,” “perfect memory,” or “nothing ever leaves your device”
would misrepresent the product's controls, uncertainty, or optional remote
capabilities.

The central relationship to communicate is this: you choose what to record,
Daydream helps you find and understand it, and the original evidence remains
within reach.

## Repository basis

This brief reflects the repository as reviewed on 2026-10-06. It describes product
direction and provides design context; it does not authorize implementation scope
or commit the project to a release schedule.

- [README: implemented capabilities and current limits](README.md)
- [Recall behavior and architecture](docs/recall-redesign-implementation.md)
- [Processing privacy boundaries](docs/processing-privacy-boundaries.md)
