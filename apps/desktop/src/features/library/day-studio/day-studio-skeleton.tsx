import { Skeleton } from '@/components/ui/skeleton';
import { recallDateLabel } from '@/features/recall/recall-format';
import { RecordingStudioLayout } from '../components/studio/layout/recording-studio-layout';

export function DayStudioSkeleton({ date }: { date: string }) {
  return (
    <section
      aria-label="Loading day studio"
      aria-busy="true"
      className="flex min-h-0 flex-1 flex-col"
    >
      <p className="sr-only" role="status">
        Loading day studio…
      </p>
      <div className="flex shrink-0 flex-col gap-6 px-5 py-6 lg:flex-row lg:items-center lg:justify-between lg:px-8 lg:py-5">
        <div className="min-w-0">
          <h1 className="text-[32px] font-light tracking-tight lg:text-4xl">
            {recallDateLabel(date)}
          </h1>
          <div aria-hidden="true" className="mt-3 flex gap-2">
            <Skeleton className="h-9 w-32" />
            <Skeleton className="h-9 w-28" />
          </div>
          <Skeleton aria-hidden="true" className="mt-3 h-4 w-56 max-w-full" />
          <div className="mt-2 h-4" />
        </div>
        <div aria-hidden="true" className="flex flex-wrap gap-2">
          <Skeleton className="h-9 w-40" />
          <Skeleton className="h-9 w-24" />
          <Skeleton className="h-9 w-24" />
        </div>
      </div>
      <div
        aria-hidden="true"
        inert
        className="relative flex min-h-[36rem] w-full flex-1 flex-col gap-6 px-5 pb-6 lg:px-8"
      >
        <RecordingStudioLayout
          initiallyOpen={false}
          disabled
          player={<PlayerSkeleton />}
          timeline={<TimelineSkeleton />}
          inspector={<InspectorSkeleton />}
        />
      </div>
    </section>
  );
}

function PlayerSkeleton() {
  return (
    <div className="bg-muted flex h-full min-h-[24rem] flex-col min-[1200px]:min-h-0">
      <div className="flex items-center justify-between border-b px-5 py-3">
        <Skeleton className="bg-background/60 h-4 w-36" />
        <Skeleton className="bg-background/60 h-7 w-28" />
      </div>
      <div className="flex min-h-0 flex-1 items-center justify-center p-6">
        <Skeleton className="bg-background/60 size-16 rounded-full" />
      </div>
      <div className="flex items-center gap-4 border-t p-4">
        <Skeleton className="bg-background/60 size-9 rounded-full" />
        <Skeleton className="bg-background/60 h-3 min-w-0 flex-1" />
        <Skeleton className="bg-background/60 h-4 w-24" />
        <Skeleton className="bg-background/60 h-8 w-12" />
      </div>
    </div>
  );
}

function TimelineSkeleton() {
  return (
    <div className="bg-background flex h-full flex-col overflow-hidden">
      <div className="flex items-center justify-between gap-4 border-b p-4">
        <Skeleton className="h-7 w-36" />
        <Skeleton className="h-7 w-32" />
      </div>
      <div className="flex gap-4 border-b px-4 py-3">
        <div className="w-28 shrink-0" />
        {[0, 1, 2, 3].map((tick) => (
          <Skeleton key={tick} className="h-3 min-w-0 flex-1" />
        ))}
      </div>
      {[0, 1, 2].map((lane) => (
        <div
          key={lane}
          className="flex min-h-12 flex-1 items-center gap-4 border-b px-4 py-3 last:border-b-0"
        >
          <Skeleton className="h-4 w-28 shrink-0" />
          <Skeleton className="h-6 min-w-0 flex-1" />
        </div>
      ))}
    </div>
  );
}

function InspectorSkeleton() {
  return (
    <div className="bg-muted rounded-panel flex h-full min-h-0 flex-col overflow-hidden">
      <div className="flex gap-4 border-b px-6 py-5">
        <Skeleton className="bg-background/60 h-6 flex-1" />
        <Skeleton className="bg-background/60 h-6 flex-1" />
      </div>
      <div className="grid gap-6 overflow-hidden p-6">
        {[0, 1, 2].map((item) => (
          <div key={item} className="grid gap-3">
            <Skeleton className="bg-background/60 h-4 w-24" />
            <Skeleton className="bg-background/60 h-20 w-full" />
            <Skeleton className="bg-background/60 h-3 w-3/4" />
            <Skeleton className="bg-background/60 h-3 w-1/2" />
          </div>
        ))}
      </div>
    </div>
  );
}
