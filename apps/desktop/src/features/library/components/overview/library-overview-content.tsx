import { AlertTriangle } from 'lucide-react';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { RecordingLibraryItem } from './recording-library-item';
import type {
  LibraryOverviewActions,
  LibraryOverviewModel,
} from './use-library-overview-controller';

interface LibraryOverviewContentProps {
  actions: Pick<
    LibraryOverviewActions,
    | 'openRecording'
    | 'requestRecordingDelete'
    | 'requestRecordingRename'
    | 'selectRecording'
  >;
  model: Pick<
    LibraryOverviewModel,
    | 'error'
    | 'selectedIdSet'
    | 'sortedRecordings'
    | 'status'
    | 'thumbnailPaths'
    | 'viewMode'
  >;
}

export function LibraryOverviewContent({
  actions,
  model,
}: LibraryOverviewContentProps) {
  return (
    <>
      {model.error ? (
        <Alert variant="destructive">
          <AlertTriangle />
          <AlertTitle>Library failed</AlertTitle>
          <AlertDescription>{model.error}</AlertDescription>
        </Alert>
      ) : null}

      {model.status === 'loading' ? (
        <div className="text-muted-foreground border p-4 text-sm">
          Loading recordings...
        </div>
      ) : null}

      {model.status !== 'loading' && model.sortedRecordings.length === 0 ? (
        <div className="text-muted-foreground border p-4 text-sm">
          No recordings yet.
        </div>
      ) : null}

      {model.sortedRecordings.length > 0 ? (
        <div
          className={
            model.viewMode === 'grid'
              ? 'grid gap-3 sm:grid-cols-2 lg:grid-cols-4'
              : 'flex flex-col gap-2'
          }
        >
          {model.sortedRecordings.map((recording) => (
            <RecordingLibraryItem
              key={recording.id}
              isSelected={model.selectedIdSet.has(recording.id)}
              recording={recording}
              thumbnailPath={model.thumbnailPaths[recording.id] ?? null}
              viewMode={model.viewMode}
              onDelete={() => actions.requestRecordingDelete(recording)}
              onOpen={() => actions.openRecording(recording)}
              onRename={() => actions.requestRecordingRename(recording)}
              onSelect={(event) => actions.selectRecording(recording, event)}
            />
          ))}
        </div>
      ) : null}
    </>
  );
}
