import { useCallback, useState } from 'react';
import type { LibraryDaysPage as LibraryDaysPageData } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { listLibraryDays } from '@/features/library/day-studio/day-client';
import { RecallReadState } from '../components/recall-read-state';
import { recallTimezone } from '../recall-format';
import { useRecallRead } from '../use-recall-read';
import { LibraryDayCard } from './library-day-card';

export interface LibraryDaysPageProps {
  page: LibraryDaysPageData;
}

/** Each mounted page owns its cursor, refresh and retry. Loading an older page
 * never rereads the pages already displayed. Privacy changes unmount the chain. */
export function LibraryDaysPage({ page }: LibraryDaysPageProps) {
  const [openedCursor, setOpenedCursor] = useState<string | null>(null);
  const cursor = page.nextBeforeDayId;
  const loadOlder = useCallback(() => setOpenedCursor(cursor), [cursor]);
  return (
    <>
      {page.days.map((day) => (
        <LibraryDayCard key={day.dayId} day={day} status={page} />
      ))}
      {cursor !== null ? (
        openedCursor === cursor ? (
          <OlderLibraryDays
            key={cursor}
            beforeDayId={cursor}
            privacyRevision={page.privacyRevision}
          />
        ) : (
          <Button onClick={loadOlder} variant="outline">
            Load older days
          </Button>
        )
      ) : null}
    </>
  );
}

function OlderLibraryDays({
  beforeDayId,
  privacyRevision,
}: {
  beforeDayId: string;
  privacyRevision: number;
}) {
  const read = useCallback(async () => {
    const page = await listLibraryDays({
      beforeDayId,
      limit: 30,
      timezone: recallTimezone,
    });
    if (
      page.privacyRevision !== privacyRevision ||
      page.days.some((day) => day.dayId >= beforeDayId)
    ) {
      throw new Error('Library page changed during listing.');
    }
    return page;
  }, [beforeDayId, privacyRevision]);
  const { state, refresh } = useRecallRead(read);
  return state.status === 'ready' ? (
    <LibraryDaysPage page={state.data} />
  ) : (
    <RecallReadState
      status={state.status}
      onRetry={refresh}
      loadingLabel="Loading days…"
      errorLabel="Older days could not be loaded."
    />
  );
}
