import { useCallback, useState } from 'react';
import { Link } from '@tanstack/react-router';
import { FullHeightLayout, PageHeader } from '@/app/PageLayout';
import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { getLibraryDayTimeline } from '@/features/library/day-studio/day-client';
import {
  DayStudio,
  type DayStudioLocation,
} from '@/features/library/day-studio/day-studio';
import { buildDayTimeline } from '@/features/library/day-studio/day-timeline';
import { useDayMutations } from '@/features/library/day-studio/use-day-mutations';
import { RecallReadState } from '@/features/recall/components/recall-read-state';
import { DayReviewPanel } from '@/features/recall/day/day-review-panel';
import {
  recallTimezone,
  recallDateLabel,
} from '@/features/recall/recall-format';
import { useRecallRead } from '@/features/recall/use-recall-read';

const EMPTY_LOCATION: DayStudioLocation = {
  recordingId: null,
  seek: null,
  evidenceId: null,
  source: null,
};
export interface DayScreenProps {
  date: string;
  location?: DayStudioLocation;
}
export function DayScreen({ date, location = EMPTY_LOCATION }: DayScreenProps) {
  const read = useCallback(
    () => getLibraryDayTimeline({ dayId: date, timezone: recallTimezone }),
    [date],
  );
  const { state, refresh } = useRecallRead(read);
  const mutations = useDayMutations(date, refresh);
  const manifest = state.status === 'ready' ? state.data : null;
  const signature = manifest ? JSON.stringify(manifest) : null;
  const [cached, setCached] = useState(() => ({
    signature,
    model: manifest ? buildDayTimeline(manifest) : null,
  }));
  const current =
    cached.signature === signature
      ? cached
      : { signature, model: manifest ? buildDayTimeline(manifest) : null };
  if (current !== cached) setCached(current);
  const model = current.model;
  return (
    <FullHeightLayout className="bg-background text-foreground h-full min-h-0">
      {mutations.failedCount > 0 ? (
        <Alert variant="destructive">
          <AlertDescription>
            Some privacy changes could not be saved. Your remaining changes are
            ready to retry.
          </AlertDescription>
          <Button
            variant="outline"
            disabled={mutations.pending}
            onClick={mutations.retry}
          >
            Retry remaining changes
          </Button>
        </Alert>
      ) : null}
      {state.status !== 'ready' ? (
        <div className="p-6">
          <PageHeader title={recallDateLabel(date)} />
          <RecallReadState status={state.status} onRetry={refresh} />
        </div>
      ) : model && model.spans.length > 0 ? (
        <DayStudio
          key={`${date}:${state.data.privacyRevision}`}
          model={model}
          location={location}
          onRefresh={refresh}
          mutations={mutations}
        />
      ) : (
        <div className="min-h-0 flex-1 overflow-y-auto p-6">
          <PageHeader title={recallDateLabel(date)} />
          <p className="text-muted-foreground mt-6">
            No playable recordings for this day.
          </p>
          <div className="mt-6 flex gap-3">
            <Button asChild variant="outline">
              <Link to="/library">Library</Link>
            </Button>
            <Button asChild>
              <Link to="/capture">Choose sources</Link>
            </Button>
          </div>
          <DayReviewPanel date={date} />
        </div>
      )}
    </FullHeightLayout>
  );
}
