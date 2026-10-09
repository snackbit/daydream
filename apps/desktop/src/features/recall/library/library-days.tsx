import { useCallback, useState } from 'react';
import { Link } from '@tanstack/react-router';
import { ArrowUpRight, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { RecallReadState } from '../components/recall-read-state';
import { getRecallStatus, listRecallDays } from '../recall-client';
import { recallTimezone } from '../recall-format';
import { useLocalDayId } from '../use-local-day-id';
import { useRecallRead } from '../use-recall-read';
import { LibraryDayCard } from './library-day-card';

export function LibraryDays() {
  const [pages, setPages] = useState(1);
  const read = useCallback(async () => {
    const status = await getRecallStatus();
    let beforeDayId: string | null = null;
    const days = [];
    let hasMore = false;
    for (let page = 0; page < pages; page += 1) {
      const batch = await listRecallDays({
        beforeDayId,
        limit: 30,
        timezone: recallTimezone,
      });
      days.push(...batch);
      hasMore = batch.length === 30;
      beforeDayId = batch.at(-1)?.dayId ?? null;
      if (!hasMore) break;
    }
    if (days.some((day) => day.privacyRevision !== status.privacyRevision))
      throw new Error('Recall eligibility changed during listing.');
    return { days, status, hasMore, privacyRevision: status.privacyRevision };
  }, [pages]);
  const { state, refresh } = useRecallRead(read);
  const today = useLocalDayId();
  if (state.status !== 'ready')
    return <RecallReadState status={state.status} onRetry={refresh} />;
  const days = state.data.days;
  function loadOlder() {
    setPages((value) => value + 1);
  }
  return (
    <div className="grid gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <h2 className="text-lg font-medium">Days</h2>
        <Button asChild variant="outline">
          <Link to="/day/$date" params={{ date: today }}>
            Today <ArrowUpRight aria-hidden="true" />
          </Link>
        </Button>
      </div>
      {days.map((day) => (
        <LibraryDayCard key={day.dayId} day={day} status={state.data.status} />
      ))}
      {state.data.hasMore ? (
        <Button onClick={loadOlder} variant="outline">
          Load older days
        </Button>
      ) : null}
      {days.length === 0 ? (
        <div className="bg-muted rounded-[20px] p-8">
          <h3 className="text-2xl font-light">
            Your next day starts with a choice.
          </h3>
          <p className="text-muted-foreground mt-3 text-sm">
            Choose your recording sources. Eligible screen text and transcripts
            will appear here as they are processed.
          </p>
          <Button asChild className="mt-6" variant="outline">
            <Link to="/capture">
              Choose sources <ArrowUpRight aria-hidden="true" />
            </Link>
          </Button>
        </div>
      ) : null}
      <div className="text-muted-foreground flex items-start gap-3 text-xs">
        <ShieldCheck aria-hidden="true" className="size-4 shrink-0" />
        <p>
          Previews respect protected sources and privacy masks. Open a day to
          inspect its eligible text and transcripts.{' '}
        </p>
      </div>
    </div>
  );
}
