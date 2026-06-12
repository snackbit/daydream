import { PageHeader, PageLayout } from '@/app/PageLayout';
import { LibraryOverviewContent } from '@/features/library/components/overview/library-overview-content';
import { LibraryOverviewDialogs } from '@/features/library/components/overview/library-overview-dialogs';
import { LibraryOverviewToolbar } from '@/features/library/components/overview/library-overview-toolbar';
import { useLibraryOverviewController } from '@/features/library/components/overview/use-library-overview-controller';

export function LibraryScreen() {
  const overview = useLibraryOverviewController();

  return (
    <PageLayout>
      <PageHeader
        title="Library"
        description="Local recordings stored on this machine."
        actions={
          <LibraryOverviewToolbar
            actions={overview.actions}
            model={overview.model}
          />
        }
      />
      <LibraryOverviewContent
        actions={overview.actions}
        model={overview.model}
      />
      <LibraryOverviewDialogs
        actions={overview.actions}
        model={overview.model}
      />
    </PageLayout>
  );
}
