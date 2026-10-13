import { useCallback, useState } from 'react';
import { Button } from '@/components/ui/button';
import { RecallReadState } from '@/features/recall/components/recall-read-state';
import { DayCoverage } from '@/features/recall/day/day-coverage';
import { DaySession } from '@/features/recall/day/day-session';
import { DaySuggestion } from '@/features/recall/day/day-suggestion';
import { DayTask } from '@/features/recall/day/day-task';
import {
  getRecallDay,
  getRecallStatus,
  setRecallDayReviewed,
} from '@/features/recall/recall-client';
import { recallTimezone } from '@/features/recall/recall-format';
import { useRecallRead } from '@/features/recall/use-recall-read';
import { DaySummaryAction } from './day-summary-action';

export interface DayReviewPanelProps {
  date: string;
}
export function DayReviewPanel({ date }: DayReviewPanelProps) {
  return <DayReviewContent key={date} date={date} />;
}
function DayReviewContent({ date }: DayReviewPanelProps) {
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
    <div className="grid gap-6 p-5">
      {state.status !== 'ready' ? (
        <RecallReadState status={state.status} onRetry={refresh} />
      ) : (
        <>
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
                          ? 'Recorded content is not available for a summary yet.'
                          : 'No summary yet. Generate one from your recorded content.'))}
            </p>
            <p className="text-muted-foreground mt-4 text-xs">
              Summaries use eligible recorded content.
            </p>
            <DaySummaryAction
              date={date}
              status={state.data.status}
              evidenceCount={state.data.day.day.evidenceCount}
              refresh={refresh}
            >
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
            </DaySummaryAction>
          </section>
          <DayCoverage day={state.data.day} />
          <div className="grid gap-8">
            {state.data.day.sessions.length > 0 ? (
              <section>
                <h2 className="text-lg">Sessions</h2>
                {withheld ? (
                  <p className="text-muted-foreground mt-5 text-sm">
                    Sessions are withheld until current sources are reprocessed.
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
    </div>
  );
}
