import { useMemo } from 'react';
import { Link, useSearch } from '@tanstack/react-router';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { ProcessingOverview } from '@/features/processing/components/processing-overview';
import { SharedProcessingWork } from '@/features/processing/components/shared-processing-work';
import { validateProcessingLocation } from '@/features/processing/processing-location';
import {
  createRecordingProcessingStore,
  useRecordingProcessingStore,
} from '@/features/processing/recording-processing-store';

export function ProcessingScreen() {
  const search = useSearch({ strict: false });
  const scope = validateProcessingLocation(search).recordings;
  const store = useMemo(
    () =>
      scope
        ? createRecordingProcessingStore(scope.split(','), 'all')
        : useRecordingProcessingStore,
    [scope],
  );
  return (
    <PageLayout>
      <PageHeader
        title="Processing"
        description="See what is running, what is waiting, and what needs your attention."
        actions={
          scope ? (
            <Button asChild size="sm" variant="outline">
              <Link to="/processing" search={{}}>
                Show all recordings
              </Link>
            </Button>
          ) : undefined
        }
      />
      {scope ? (
        <p className="text-muted-foreground text-sm">
          Showing {scope.split(',').length} selected recordings.
        </p>
      ) : null}
      <ProcessingOverview store={store} />
      <SharedProcessingWork />
    </PageLayout>
  );
}
