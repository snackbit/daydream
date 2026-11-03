import type {
  LibraryRecordingDetail,
  OcrHistoryCursor,
  OcrTimelineFrame,
  RecordingAiResultInput,
  RecordingVisualObservations,
} from '@daydream/shared';
import {
  getRecordingActivity,
  getRecordingAiProcessingState,
  getRecordingVisualObservations,
} from '@/features/ai/ai-client';
import { getOcrTimelineFrames } from '@/features/ai/ocr/ocr-client';
import { visibleOcrTimelineFrames } from '@/features/ai/ocr/ocr-timeline-visibility';
import {
  visibleActivity,
  visibleAiProcessingState,
  visibleVisualObservations,
} from '@/features/ai/results/derived-result-visibility';
import { buildStudioVisualFrames } from '../../components/studio/observations/studio-visual-observations';
import type { StudioResultRange } from '../../components/studio/results/studio-result-window-store';
import type { DayRecordingResults } from '../day-results';

/** Each source publishes independently; OCR pagination cannot gate speech. */
export async function readDayResultWindow(
  recording: LibraryRecordingDetail,
  input: RecordingAiResultInput,
  request: StudioResultRange,
  current: () => boolean,
  publish: (data: DayRecordingResults) => void,
  previous?: DayRecordingResults,
) {
  let data: DayRecordingResults = {
    request,
    ai: previous?.ai ?? null,
    activity: previous?.activity ?? null,
    frames: previous?.frames ?? [],
    pending: true,
  };
  let visual: RecordingVisualObservations | null = null;
  const frames: OcrTimelineFrame[] = [];
  let visualReady = false;
  let framesReady = Boolean(input.bucketMs);
  function update(patch: Partial<DayRecordingResults>) {
    if (!current()) return;
    data = { ...data, ...patch };
    publish(data);
  }
  function updateFrames() {
    // Refresh the combined visual source atomically so partial pages or a failed
    // companion read cannot erase previously readable observations.
    if (previous && (!visualReady || !framesReady)) return;
    update({
      frames: buildStudioVisualFrames(
        visibleVisualObservations(recording, visual),
        recording,
        visibleOcrTimelineFrames(recording, frames),
      ),
    });
  }
  async function readFrames() {
    if (input.bucketMs) return;
    let cursor: OcrHistoryCursor | null = null;
    const cursors = new Set<string>();
    do {
      if (!current()) return;
      const page = await getOcrTimelineFrames({
        ...input,
        cursor,
        limit: 250,
        startTimestampMs: input.startOffsetMs ?? null,
        endTimestampMs: input.endOffsetMs ?? null,
      });
      if (!current()) return;
      frames.push(...page.frames);
      updateFrames();
      cursor = page.nextCursor;
      if (cursor) {
        const key = JSON.stringify(cursor);
        if (cursors.has(key)) throw new Error('Repeated OCR cursor');
        cursors.add(key);
      }
    } while (cursor);
    framesReady = true;
    updateFrames();
  }
  const settled = await Promise.allSettled([
    getRecordingAiProcessingState(input).then((ai) =>
      update({
        ai: visibleAiProcessingState(recording, ai),
      }),
    ),
    getRecordingActivity(input).then((activity) =>
      update({
        activity: visibleActivity(recording, activity),
      }),
    ),
    getRecordingVisualObservations({ ...input, includeOcrRegions: false }).then(
      (value) => {
        visual = value;
        visualReady = true;
        updateFrames();
      },
    ),
    readFrames(),
  ]);
  update({ pending: false });
  return {
    data,
    failed: settled.some((result) => result.status === 'rejected'),
  };
}
