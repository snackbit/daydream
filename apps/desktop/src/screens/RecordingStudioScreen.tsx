import { useCallback, useState } from 'react';
import { useNavigate, useParams } from '@tanstack/react-router';
import { AlertTriangle } from 'lucide-react';
import { FullHeightLayout } from '@/app/PageLayout';
import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { RecordingStudio } from '@/features/library/components/studio/recording-studio';
import { RecordingDebugDialog } from '@/features/library/recording-studio-route/recording-debug-dialog';
import { RecordingDeleteDialog } from '@/features/library/recording-studio-route/recording-delete-dialog';
import { RecordingRenameDialog } from '@/features/library/recording-studio-route/recording-rename-dialog';
import { RecordingStudioToolbar } from '@/features/library/recording-studio-route/recording-studio-toolbar';
import { useRecordingStudioController } from '@/features/library/recording-studio-route/use-recording-studio-controller';
import { useDebugEnabled } from '@/features/settings/use-debug-enabled';

export function RecordingStudioScreen() {
  const { recordingId } = useParams({ strict: false }) as {
    recordingId: string;
  };
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

  return (
    <FullHeightLayout className="h-full min-h-0">
      <RecordingStudioToolbar
        aiProcessing={state.aiProcessing}
        debugEnabled={debugEnabled}
        detail={state.detail}
        loadStatus={state.status}
        processingStartBusy={state.processingStartBusy}
        onDebug={() => setDebugDialogOpen(true)}
        onDelete={() => setDeleteDialogOpen(true)}
        onRefresh={() => void actions.load()}
        onRename={() => setRenameDialogOpen(true)}
        onStartAiProcessing={() => void actions.startAiProcessing()}
      />

      {state.error ? (
        <div className="p-4">
          <Alert variant="destructive">
            <AlertTriangle />
            <AlertTitle>Recording failed to load</AlertTitle>
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
          className="text-muted-foreground px-4 py-1 text-[10px]"
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
        />
      ) : null}

      <RecordingDeleteDialog
        open={deleteDialogOpen}
        onConfirm={() => void actions.confirmDelete()}
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
