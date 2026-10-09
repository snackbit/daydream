import { useCallback, useState } from 'react';
import { useNavigate, useParams, useSearch } from '@tanstack/react-router';
import { AlertTriangle } from 'lucide-react';
import { FullHeightLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { RecordingStudio } from '@/features/library/components/studio/recording-studio';
import { useStudioRecallEvidence } from '@/features/library/components/studio/use-studio-recall-evidence';
import { RecordingDebugDialog } from '@/features/library/recording-studio-route/recording-debug-dialog';
import { RecordingDeleteDialog } from '@/features/library/recording-studio-route/recording-delete-dialog';
import { RecordingRenameDialog } from '@/features/library/recording-studio-route/recording-rename-dialog';
import { RecordingStudioToolbar } from '@/features/library/recording-studio-route/recording-studio-toolbar';
import { useRecordingStudioController } from '@/features/library/recording-studio-route/use-recording-studio-controller';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';

export function RecordingStudioScreen() {
  const search = useSearch({ from: '/library/$recordingId' });
  const { recordingId } = useParams({ from: '/library/$recordingId' });
  const navigate = useNavigate();
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [renameDialogOpen, setRenameDialogOpen] = useState(false);
  const [debugDialogOpen, setDebugDialogOpen] = useState(false);
  const debugEnabled = useDebugEnabled();
  const onDeleted = useCallback(async () => {
    await navigate({ to: '/library' });
  }, [navigate]);
  const { actions, state } = useRecordingStudioController({
    onDeleted,
    recordingId,
  });
  const recall = useStudioRecallEvidence(
    search,
    recordingId,
    JSON.stringify(state.detail?.redactions ?? []),
  );
  const openDebug = useCallback(() => setDebugDialogOpen(true), []);
  const openDelete = useCallback(() => setDeleteDialogOpen(true), []);
  const openRename = useCallback(() => setRenameDialogOpen(true), []);
  const refresh = useCallback(() => void actions.load(), [actions]);
  const startProcessing = useCallback(
    () => void actions.startAiProcessing(),
    [actions],
  );
  const confirmDelete = useCallback(() => actions.confirmDelete(), [actions]);

  return (
    <FullHeightLayout className="bg-background text-foreground h-full min-h-0">
      <RecordingStudioToolbar
        aiProcessing={state.aiProcessing}
        debugEnabled={debugEnabled}
        detail={state.detail}
        loadStatus={state.status}
        processingStartBusy={state.processingStartBusy}
        onDebug={openDebug}
        onDelete={openDelete}
        onRefresh={refresh}
        onRename={openRename}
        onStartAiProcessing={startProcessing}
      />
      {recall.state?.status === 'loading' ? (
        <p
          role="status"
          className="text-muted-foreground px-5 pb-6 text-sm lg:px-8"
        >
          Checking recall source…
        </p>
      ) : recall.state?.status === 'unavailable' ? (
        <div className="rounded-panel mx-5 mb-6 flex flex-wrap items-center gap-4 border p-6 lg:mx-8">
          <p role="status" className="text-sm">
            This recall source is unavailable. It may have been hidden, deleted
            or replaced.
          </p>
          <Button variant="outline" onClick={recall.retry}>
            Check again
          </Button>
        </div>
      ) : null}

      {state.error ? (
        <div className="p-4">
          <Alert variant="destructive">
            <AlertTriangle />
            <AlertTitle>
              {state.status === 'error'
                ? 'Recording failed to load'
                : 'Recording action failed'}
            </AlertTitle>
            <AlertDescription>{state.error}</AlertDescription>
          </Alert>
        </div>
      ) : null}

      {state.status === 'loading' ? (
        <div className="text-muted-foreground p-4 text-sm">
          Loading recording...
        </div>
      ) : null}
      {state.ocrTimelineError ? (
        <div className="px-4 py-2">
          <Alert variant="destructive">
            <AlertTitle>OCR observations unavailable</AlertTitle>
            <AlertDescription>
              {state.ocrTimelineError} Refresh the studio to try again.
            </AlertDescription>
          </Alert>
        </div>
      ) : state.ocrTimelineStatus === 'loading' ? (
        <p
          className="text-muted-foreground px-5 py-3 text-xs lg:px-8"
          role="status"
        >
          Loading OCR observation index…
        </p>
      ) : null}

      {state.detail ? (
        <RecordingStudio
          debugEnabled={debugEnabled}
          activity={state.activity.state}
          aiProcessingState={state.aiProcessing.state}
          detail={state.detail}
          onRefreshPlayback={actions.load}
          onCreateRedactions={actions.createRedactions}
          onDeleteRedaction={actions.removeRedaction}
          onUnredactRanges={actions.unredactRanges}
          playbackAssets={state.playbackAssets}
          thumbnailStatus={state.thumbnailStatus}
          visualObservations={state.visualObservations.state}
          ocrFrames={state.ocrFrames}
          initialEvidence={recall.state?.evidence ?? null}
          initialSeek={search.evidenceId === null ? search.seek : null}
        />
      ) : null}

      <RecordingDeleteDialog
        open={deleteDialogOpen}
        onConfirm={confirmDelete}
        onOpenChange={setDeleteDialogOpen}
      />
      {renameDialogOpen ? (
        <RecordingRenameDialog
          detail={state.detail}
          open={renameDialogOpen}
          onOpenChange={setRenameDialogOpen}
          onSave={actions.confirmRename}
        />
      ) : null}
      {debugDialogOpen ? (
        <RecordingDebugDialog
          open={debugDialogOpen}
          recordingId={recordingId}
          onOpenChange={setDebugDialogOpen}
        />
      ) : null}
    </FullHeightLayout>
  );
}
