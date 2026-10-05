import { Link } from '@tanstack/react-router';
import { ArrowUpRight, LockKeyhole } from 'lucide-react';
import { PageHeader, PageLayout } from '@/app/PageLayout';
import { Button } from '@/components/ui/button';
import { RecentRecordingsSection } from '@/features/home/components/recent-recordings-section';
import { useHomeRecordingsController } from '@/features/home/use-home-recordings-controller';

export function HomeScreen() {
  const recordings = useHomeRecordingsController();

  return (
    <PageLayout className="gap-5">
      <PageHeader
        title="Home"
        description="Start a visible capture when you need one. Review finished sessions in your local Library."
        actions={
          <Button asChild className="gap-1.5">
            <Link to="/capture">
              Prepare capture
              <ArrowUpRight aria-hidden="true" className="size-3.5" />
            </Link>
          </Button>
        }
      />

      <div className="grid gap-6 lg:grid-cols-[minmax(0,1fr)_16rem] lg:gap-8">
        <RecentRecordingsSection
          actions={recordings.actions}
          model={recordings.model}
        />

        <aside className="border-t pt-4 lg:border-t-0 lg:border-l lg:pt-0 lg:pl-5">
          <h2 className="flex items-center gap-2 text-[13px] font-semibold tracking-tight">
            <LockKeyhole aria-hidden="true" className="size-3.5" />
            Data stays local
          </h2>
          <p className="text-muted-foreground mt-1.5 text-xs leading-5">
            Recording files stay on this device. Settings identifies optional
            remote AI providers and the data they can receive.
          </p>
          <Button asChild className="mt-2 h-7 px-0" size="sm" variant="link">
            <Link to="/settings">Open Settings</Link>
          </Button>
        </aside>
      </div>
    </PageLayout>
  );
}
