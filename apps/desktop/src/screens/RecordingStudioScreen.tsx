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
    <FullHeightLayout>
      <RecordingStudioToolbar
        aiProcessing={state.aiProcessing}
        debugEnabled={debugEnabled}
        detail={state.detail}
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

      {state.detail ? (
        <RecordingStudio
          aiProcessingState={state.aiProcessing.state}
          detail={state.detail}
          onCreateRedactions={actions.createRedactions}
          onDeleteRedaction={actions.removeRedaction}
          onUnredactRanges={actions.unredactRanges}
          playbackAssets={state.playbackAssets}
          thumbnailStatus={state.thumbnailStatus}
          visualObservations={state.visualObservations.state}
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
