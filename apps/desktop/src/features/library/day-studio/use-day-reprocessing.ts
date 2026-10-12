import { useCallback, useEffect, useState } from 'react';
import type { ReprocessingTarget } from '@daydream/shared';
import { useStudioReprocessing } from '../components/studio/reprocessing/use-studio-reprocessing';
import { splitDayRanges, type DayTimelineModel } from './day-timeline';

/** Each recording retains its own reviewed native reprocessing plan. */
export function useDayReprocessing(model: DayTimelineModel) {
  const [queue, setQueue] = useState<
    Array<{ recordingId: string; targets: ReprocessingTarget[] }>
  >([]);
  const head = queue[0];
  const controller = useStudioReprocessing(head?.recordingId ?? '');
  const open = useCallback(
    (targets: ReprocessingTarget[]) => {
      const groups = splitDayRanges(
        model,
        targets.flatMap((t) =>
          (
            t.ranges ?? [
              {
                startOffsetMs: 0,
                endOffsetMs:
                  (model.view.endedAtMs ?? model.originMs) - model.originMs,
              },
            ]
          ).map((r) => ({ ...r, trackIds: [t.trackId] })),
        ),
      );
      setQueue(
        [...groups].map(([recordingId, ranges]) => {
          const tracks = new Map<
            string,
            Array<{ startOffsetMs: number; endOffsetMs: number }>
          >();
          for (const range of ranges)
            for (const trackId of range.trackIds)
              tracks.set(trackId, [
                ...(tracks.get(trackId) ?? []),
                {
                  startOffsetMs: range.startOffsetMs,
                  endOffsetMs: range.endOffsetMs,
                },
              ]);
          return {
            recordingId,
            targets: [...tracks].map(([trackId, ranges]) => ({
              trackId,
              ranges,
            })),
          };
        }),
      );
    },
    [model],
  );
  const show = controller.open;
  useEffect(() => {
    if (head) show(head.targets);
  }, [head, show]);
  const close = useCallback(() => {
    if (controller.pending) return;
    controller.close();
    setQueue([]);
  }, [controller]);
  const start = useCallback(async () => {
    await controller.start();
  }, [controller]);
  useEffect(() => {
    if (controller.queued) setQueue((q) => q.slice(1));
  }, [controller.queued]);
  return { ...controller, open, close, start };
}
