import { Link } from '@tanstack/react-router';
import { ArrowUpRight, ShieldCheck } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { listLibraryDays } from '@/features/library/day-studio/day-client';
import { RecallReadState } from '../components/recall-read-state';
import { recallTimezone } from '../recall-format';
import { useLocalDayId } from '../use-local-day-id';
import { useRecallRead } from '../use-recall-read';
import { LibraryDaysPage } from './library-days-page';

function readDays() {
  return listLibraryDays({
    beforeDayId: null,
    limit: 30,
    timezone: recallTimezone,
  });
}

export function LibraryDays() {
  const { state, refresh } = useRecallRead(readDays);
  const today = useLocalDayId();
  if (state.status !== 'ready')
    return (
      <RecallReadState
        status={state.status}
        onRetry={refresh}
        loadingLabel="Loading days…"
        errorLabel="Library days could not be loaded."
      />
    );
  const days = state.data.days;
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
      <LibraryDaysPage key={state.data.privacyRevision} page={state.data} />
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
