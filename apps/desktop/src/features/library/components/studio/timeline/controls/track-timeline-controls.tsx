import { Alert, AlertDescription } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { ReprocessingDropdown } from '../../reprocessing/reprocessing-menu';
import type { TrackTimelineProps } from '../track-timeline';
import type { TrackTimelineController } from '../use-track-timeline-controller';
import { TrackTimelineToolbar } from './track-timeline-toolbar';

export interface TrackTimelineControlsProps {
  controller: TrackTimelineController;
  onReprocess: TrackTimelineProps['onReprocess'];
}

export function renderTrackTimelineControls({
  controller,
  onReprocess,
}: TrackTimelineControlsProps) {
  const {
    redactionMutation,
    followPlayhead,
    viewModel,
    hasTimeRangeSelection,
    targets,
    follow,
    menuOpen,
    waveformStatus,
    waveformTiles,
    zoomSelection,
    zoomIn,
    zoomOut,
  } = controller;

  return (
    <>
      <TrackTimelineToolbar
        followAction={
          !followPlayhead ? (
            <Button size="sm" variant="outline" onClick={follow}>
              Follow playback
            </Button>
          ) : null
        }
        selectionActions={
          hasTimeRangeSelection ? (
            <>
              <Button size="sm" variant="outline" onClick={zoomSelection}>
                Zoom In
              </Button>
              {onReprocess ? (
                <ReprocessingDropdown
                  label="Selection"
                  targets={targets}
                  onReprocess={onReprocess}
                  onOpenChange={menuOpen}
                />
              ) : null}
            </>
          ) : null
        }
        canZoomIn={viewModel.canZoomIn}
        canZoomOut={viewModel.canZoomOut}
        hasTimeRangeSelection={hasTimeRangeSelection}
        selectedRangeRedactionCount={viewModel.selectedRangeRedactions.length}
        statusText={
          waveformTiles?.error ??
          (waveformStatus === 'error'
            ? 'Waveforms unavailable. Retry in Processing.'
            : waveformStatus === 'loading'
              ? 'Preparing waveforms…'
              : 'Video and audio timeline')
        }
        zoomPercent={viewModel.zoomPercent}
        onZoomIn={zoomIn}
        onZoomOut={zoomOut}
        onRedactSelection={redactionMutation.redact}
        onUnredactSelection={redactionMutation.unredact}
        redactionPending={redactionMutation.pending}
      />
      {redactionMutation.error ? (
        <Alert variant="destructive">
          <AlertDescription>{redactionMutation.error}</AlertDescription>
        </Alert>
      ) : null}
      {redactionMutation.pending ? (
        <p role="status" className="text-muted-foreground px-6 py-2 text-xs">
          Updating redactions…
        </p>
      ) : null}
    </>
  );
}
