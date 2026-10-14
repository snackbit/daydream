import type {
  ProcessingLaneActivity,
  ProcessingWaitingReason,
  RecordingProcessingLane,
} from '@daydream/shared';
import { taskStatusLabel } from './task-format';

const waitingLabels: Record<ProcessingWaitingReason, string> = {
  ocr: 'Waiting for OCR in this section',
  'previous-source': 'Waiting for the previous section on this source',
  retry: 'Waiting for automatic retry',
  'live-input': 'Waiting for the next finalized audio section',
  protection: 'Waiting for recording protection',
  capacity: 'Waiting for an available worker',
  paused: 'Paused',
  unknown: 'Queued · waiting to be scheduled',
};
export function processingActivityLabel(
  activity: ProcessingLaneActivity,
): string {
  if (activity.waitingReason) return waitingLabels[activity.waitingReason];
  if (activity.workerActive === false && activity.status === 'running')
    return 'Worker stopped · reconciling state';
  switch (activity.phase) {
    case 'starting':
      return 'Preparing source';
    case 'recognizing-text':
      return 'Recognizing screen text';
    case 'detecting-speech':
      return 'Checking for speech';
    case 'transcribing':
      return 'Transcribing speech';
    case 'observing-screen':
      return 'Describing screen changes';
    case 'waiting-for-model':
      return 'Waiting for the model';
    case 'waiting-for-worker':
      return 'Waiting for an available worker';
    default:
      return activity.status === 'running' ? 'Processing source' : 'Queued';
  }
}
export function processingLaneLabel(lane: RecordingProcessingLane): string {
  switch (lane.applicability) {
    case 'disabled':
      return 'Disabled in settings';
    case 'no-source':
      return 'No source recorded';
    case 'protected':
      return 'Waiting for recording protection';
    case 'no-eligible-input':
      return 'No eligible source sections';
    case 'waiting':
      return 'Waiting for finalized recording segments';
    case 'available': {
      if (
        lane.inputOpen &&
        lane.completed + lane.skipped === lane.total &&
        !lane.failed &&
        !lane.cancelled &&
        lane.status !== 'paused'
      )
        return 'Caught up · live recording';
      const current =
        lane.activity?.find((item) => item.status === 'running') ??
        lane.activity?.[0];
      if (current) return processingActivityLabel(current);
      return lane.status ? taskStatusLabel(lane.status) : 'Not queued';
    }
  }
}
