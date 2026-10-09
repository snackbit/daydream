import { LibraryOverviewContent } from './library-overview-content';
import { LibraryOverviewDialogs } from './library-overview-dialogs';
import { LibraryOverviewToolbar } from './library-overview-toolbar';
import { useLibraryOverviewController } from './use-library-overview-controller';

/** Recording scans and preview preparation start when their tab is opened. */
export function LibraryRecordingsContent() {
  const overview = useLibraryOverviewController();
  return (
    <div className="grid gap-5">
      <LibraryOverviewToolbar
        actions={overview.actions}
        model={overview.model}
      />
      <LibraryOverviewContent
        actions={overview.actions}
        model={overview.model}
      />
      <LibraryOverviewDialogs
        actions={overview.actions}
        model={overview.model}
      />
    </div>
  );
}
