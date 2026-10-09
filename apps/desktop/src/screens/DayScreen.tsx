import { useCallback, useState } from 'react';
import { Link } from '@tanstack/react-router';
import { ArrowLeft, RefreshCw } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { EvidenceSourceLink } from '@/features/recall/components/evidence-source-link';
import { RecallReadState } from '@/features/recall/components/recall-read-state';
import { DayCoverage } from '@/features/recall/day/day-coverage';
import { DayRecordings } from '@/features/recall/day/day-recordings';
import { DaySession } from '@/features/recall/day/day-session';
import { DaySuggestion } from '@/features/recall/day/day-suggestion';
import { DayTask } from '@/features/recall/day/day-task';
import {
  getRecallDay,
  getRecallStatus,
  regenerateRecallDay,
  setRecallDayReviewed,
} from '@/features/recall/recall-client';
import {
  recallDateLabel,
  recallTimezone,
  recallDayStatusLabel,
} from '@/features/recall/recall-format';
import { useRecallRead } from '@/features/recall/use-recall-read';

export interface DayScreenProps {
  date: string;
}
export function DayScreen({ date }: DayScreenProps) {
  const read = useCallback(async () => {
    const [day, status] = await Promise.all([
      getRecallDay({ dayId: date, timezone: recallTimezone }),
      getRecallStatus(),
    ]);
    if (day.privacyRevision !== status.privacyRevision)
      throw new Error('Recall eligibility changed during reading.');
    return { day, status, privacyRevision: day.privacyRevision };
  }, [date]);
  const { state, refresh } = useRecallRead(read);
  const withheld =
    state.status === 'ready' &&
    (state.data.day.day.status === 'stale' ||
      state.data.day.day.status === 'invalidated');
  const [busy, setBusy] = useState(false);
  const [actionError, setActionError] = useState(false);
  const perform = useCallback(
    (action: () => Promise<unknown>) => {
      setBusy(true);
      setActionError(false);
      void action()
        .then(refresh)
        .catch(() => {
          setActionError(true);
        })
        .finally(() => {
          setBusy(false);
        });
    },
    [refresh],
  );
  function regenerate() {
    perform(() =>
      regenerateRecallDay({ dayId: date, timezone: recallTimezone }),
    );
  }
  function markReviewed() {
    if (state.status !== 'ready') return;
    const { day } = state.data.day;
    perform(() =>
      setRecallDayReviewed({
        dayId: date,
        reviewed: !day.reviewed,
        inputFingerprint: day.inputFingerprint,
      }),
    );
  }
  return (
    <PageLayout>
      <Link
        to="/library"
        className="text-muted-foreground flex w-fit items-center gap-2 text-xs"
      >
        <ArrowLeft aria-hidden="true" className="size-4" />
        Library
      </Link>
      <PageHeader
        title={recallDateLabel(date)}
        description="Recordings, notes and summaries for this day."
        actions={
          state.status === 'ready' ? (
            <Badge variant="outline">
              {recallDayStatusLabel[state.data.day.day.status]}
            </Badge>
          ) : null
        }
      />
      {state.status !== 'ready' ? (
        <RecallReadState status={state.status} onRetry={refresh} />
      ) : (
        <>
          <DayRecordings recordings={state.data.day.recordings} />
          {actionError ? (
            <p role="alert" className="text-destructive text-sm">
              The change could not be saved. Please try again.
            </p>
          ) : null}
          <section
            aria-label="Day overview"
            className="bg-muted rounded-[20px] p-6 sm:p-8"
          >
            <p className="text-muted-foreground mb-3 text-xs tracking-widest uppercase">
              Summary
            </p>
            <p className="max-w-3xl text-lg leading-8 break-words">
              {withheld
                ? 'This interpretation needs an update. Previous source-derived text is withheld.'
                : (state.data.day.day.summary ??
                  (state.data.status.generationUnavailableReason ===
                  'recall_generation_disabled'
                    ? 'Summaries off. Recordings are available above.'
                    : !state.data.status.generationReady
                      ? 'Summary setup incomplete. Configure a summary provider in Settings.'
                      : state.data.day.day.status === 'failed'
                        ? 'Summary failed. Check the provider settings and try again.'
                        : state.data.day.day.evidenceCount === 0
                          ? 'Text and transcripts are not available yet.'
                          : 'No summary yet. Generate one from your text and transcripts.'))}
            </p>
            <p className="text-muted-foreground mt-4 text-xs">
              Summaries use available text and transcripts.
            </p>
            <div className="mt-5 flex flex-wrap gap-3">
              {state.data.status.generationReady &&
              state.data.day.day.evidenceCount > 0 ? (
                <Button
                  disabled={
                    busy ||
                    !state.data.status.generationReady ||
                    state.data.day.day.evidenceCount === 0
                  }
                  onClick={regenerate}
                  variant="outline"
                >
                  <RefreshCw aria-hidden="true" className="size-4" />
                  Generate summary
                </Button>
              ) : null}
              {state.data.day.day.summary !== null ? (
                <Button
                  disabled={
                    busy ||
                    (state.data.day.day.status !== 'ready' &&
                      state.data.day.day.status !== 'partial')
                  }
                  onClick={markReviewed}
                  variant="outline"
                >
                  {state.data.day.day.reviewed
                    ? 'Mark unreviewed'
                    : 'Mark reviewed'}
                </Button>
              ) : null}
            </div>
            {!state.data.status.generationReady ? (
              <p className="text-muted-foreground mt-3 text-xs">
                <Link
                  to="/settings"
                  search={{ tab: 'ai', focus: 'recall-generation' }}
                  className="underline"
                >
                  Summary settings
                </Link>
              </p>
            ) : null}
          </section>
          <DayCoverage day={state.data.day} />
          <div className="grid gap-8 lg:grid-cols-[1.35fr_1fr]">
            {state.data.day.sessions.length > 0 ||
            (state.data.day.evidence?.length ?? 0) > 0 ? (
              <section>
                <h2 className="text-lg">Text and transcripts</h2>
                {state.data.day.evidence?.length ? (
                  <SharedDisclosure label="Recorded text and transcripts">
                    <div className="grid gap-4 pt-4">
                      {state.data.day.evidence.map((evidence) => (
                        <article key={evidence.evidenceId}>
                          <EvidenceSourceLink citation={evidence} />
                          <p className="mt-2 text-sm whitespace-pre-wrap">
                            {evidence.text}
                          </p>
                        </article>
                      ))}
                    </div>
                  </SharedDisclosure>
                ) : null}
                {withheld ? (
                  <p className="text-muted-foreground mt-5 text-sm">
                    Sessions are withheld until current sources are reprocessed.
                  </p>
                ) : state.data.day.sessions.length === 0 ? (
                  <p className="text-muted-foreground mt-5 text-sm">
                    No interpreted sessions yet.
                  </p>
                ) : (
                  state.data.day.sessions.map((session) => (
                    <DaySession
                      key={`${session.id}-${state.data.privacyRevision}`}
                      session={session}
                      dayId={date}
                      busy={busy}
                      onAction={perform}
                    />
                  ))
                )}
              </section>
            ) : null}
            {state.data.day.tasks.length > 0 ||
            (!withheld &&
              state.data.day.suggestions.some(
                (item) => !item.dismissed && item.acceptedTaskId === null,
              )) ? (
              <aside className="bg-muted h-fit rounded-[20px] p-6">
                <h2 className="text-lg">Tasks</h2>
                <p className="text-muted-foreground mt-3 mb-5 text-xs leading-5">
                  Suggestions are yours to review. Accepted tasks stay on this
                  device and never send messages or create external actions.
                </p>
                {!withheld
                  ? state.data.day.suggestions
                      .filter(
                        (suggestion) =>
                          suggestion.acceptedTaskId === null &&
                          !suggestion.dismissed,
                      )
                      .map((suggestion) => (
                        <DaySuggestion
                          key={suggestion.id}
                          suggestion={suggestion}
                          dayId={date}
                          busy={busy}
                          onAction={perform}
                        />
                      ))
                  : null}
                {state.data.day.tasks.map((task) => (
                  <DayTask
                    key={`${task.id}-${state.data.privacyRevision}`}
                    task={withheld ? { ...task, sourceWithheld: true } : task}
                    busy={busy}
                    onAction={perform}
                  />
                ))}
                {state.data.day.suggestions.length === 0 &&
                state.data.day.tasks.length === 0 ? (
                  <p className="text-muted-foreground text-sm">
                    No suggested or saved tasks for this day.
                  </p>
                ) : null}
              </aside>
            ) : null}
          </div>
        </>
      )}
    </PageLayout>
  );
}
