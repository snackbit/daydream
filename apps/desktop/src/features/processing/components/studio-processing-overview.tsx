import { useMemo, useState } from 'react';
import { Link } from '@tanstack/react-router';
import { Workflow } from 'lucide-react';
import { Button } from '@/components/ui/button';
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from '@/components/ui/sheet';
import { createRecordingProcessingStore } from '../recording-processing-store';
import { ProcessingOverview } from './processing-overview';

export interface StudioProcessingOverviewProps {
  recordingIds: readonly string[];
}
export function StudioProcessingOverview({
  recordingIds,
}: StudioProcessingOverviewProps) {
  const [open, setOpen] = useState(false);
  const scope = [...new Set(recordingIds)].sort().join(',');
  const store = useMemo(
    () => createRecordingProcessingStore(scope ? scope.split(',') : [], 'all'),
    [scope],
  );
  return (
    <Sheet open={open} onOpenChange={setOpen}>
      <SheetTrigger asChild>
        <Button
          variant="outline"
          disabled={!scope}
          data-studio-preserve-selection
        >
          <Workflow className="size-3.5" />
          Processing overview
        </Button>
      </SheetTrigger>
      <SheetContent className="overflow-y-auto data-[side=right]:w-full data-[side=right]:sm:max-w-2xl">
        <SheetHeader className="pr-12">
          <SheetTitle>Processing overview</SheetTitle>
          <SheetDescription>
            Current and queued work for{' '}
            {recordingIds.length === 1 ? 'this recording' : 'this day'}.
          </SheetDescription>
        </SheetHeader>
        <div className="min-w-0 space-y-5 px-5 pb-6">
          {open && scope ? (
            <ProcessingOverview key={scope} store={store} />
          ) : null}
          <Button asChild variant="link" className="px-0">
            <Link to="/processing" search={{ recordings: scope }}>
              Open full Processing page
            </Link>
          </Button>
        </div>
      </SheetContent>
    </Sheet>
  );
}
