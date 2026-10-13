import {
  startTransition,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from 'react';
import type { OcrHistoryCursor, OcrTimelineFrame } from '@daydream/shared';
import {
  getRecordingActivity,
  getRecordingAiProcessingState,
  getRecordingVisualObservations,
  listenToAiProcessingChanged,
} from '@/features/ai/ai-client';
import { getOcrTimelineFrames } from '@/features/ai/ocr/ocr-client';
import { visibleOcrTimelineFrames } from '@/features/ai/ocr/ocr-timeline-visibility';
import {
  visibleActivity,
  visibleAiProcessingState,
  visibleVisualObservations,
} from '@/features/ai/results/derived-result-visibility';
import { useStudioResultRange } from '../components/studio/studio-result-window-store';
import { buildStudioVisualFrames } from '../components/studio/studio-visual-observations';
import type { DayRecordingResults } from './day-results';
import type { DayTimelineModel } from './day-timeline';

export function useDayResults(model: DayTimelineModel) {
  const resultRange = useStudioResultRange(model.view.id);
  const [revision, setRevision] = useState(0);
  const [failedIds, setFailedIds] = useState<ReadonlySet<string>>(new Set());
  const reload = useCallback(() => setRevision((value) => value + 1), []);
  const cachedReads = useMemo(
    () => new Map<string, { recordingId: string; data: DayRecordingResults }>(),
    [model, revision],
  );
  const [results, setResults] = useState(
      new Map<string, DayRecordingResults>(),
    ),
    [error, setError] = useState(false);
  useEffect(() => {
    let active = true,
      timer = 0,
      unlisten: (() => void) | undefined;
    let eventsUnavailable = false;
    const generations = new Map<string, number>();
    const pending = new Set<string>();
    const retryReads = new Set<string>();
    async function load(ids: ReadonlySet<string>) {
      let index = 0;
      const recordings = model.manifest.recordings.filter((recording) =>
        ids.has(recording.id),
      );
      async function worker() {
        while (active) {
          const recording = recordings[index++];
          if (!recording) return;
          const spans = model.spans.filter(
            (span) =>
              span.recordingId === recording.id &&
              ((span.startMs - model.originMs < resultRange.endOffsetMs &&
                span.startMs - model.originMs + span.durationMs >
                  resultRange.startOffsetMs) ||
                (span.startMs - model.originMs <
                  (resultRange.reviewEndOffsetMs ?? 0) &&
                  span.startMs - model.originMs + span.durationMs >
                    (resultRange.reviewStartOffsetMs ?? 0))),
          );
          if (!spans.length) continue;
          function sourceRanges(from: number, to: number) {
            return spans.flatMap((span) => {
              const start = span.startMs - model.originMs;
              const left = Math.max(start, from);
              const right = Math.min(start + span.durationMs, to);
              return right > left
                ? [
                    {
                      from: span.sourceOffsetMs + left - start,
                      to: span.sourceOffsetMs + right - start,
                    },
                  ]
                : [];
            });
          }
          const timelineRanges = sourceRanges(
            resultRange.startOffsetMs,
            resultRange.endOffsetMs,
          );
          const reviewRanges = sourceRanges(
            resultRange.reviewStartOffsetMs ?? 0,
            resultRange.reviewEndOffsetMs ?? 0,
          );
          const ranges = timelineRanges.length ? timelineRanges : reviewRanges;
          if (!ranges.length) continue;
          const startOffsetMs = Math.floor(
            Math.min(...ranges.map((range) => range.from)),
          );
          const endOffsetMs = Math.ceil(
            Math.max(...ranges.map((range) => range.to)),
          );
          const reviewStartOffsetMs = reviewRanges.length
            ? Math.floor(Math.min(...reviewRanges.map((range) => range.from)))
            : startOffsetMs;
          const reviewEndOffsetMs = reviewRanges.length
            ? Math.ceil(Math.max(...reviewRanges.map((range) => range.to)))
            : Math.min(endOffsetMs, startOffsetMs + 360_000);
          const request = (generations.get(recording.id) ?? 0) + 1;
          generations.set(recording.id, request);
          const current = () =>
            active && generations.get(recording.id) === request;
          try {
            const input = {
              recordingId: recording.id,
              startOffsetMs,
              endOffsetMs,
              reviewStartOffsetMs,
              reviewEndOffsetMs,
              ...(resultRange.bucketMs
                ? { bucketMs: resultRange.bucketMs }
                : {}),
            };
            const key = JSON.stringify(input);
            const cached = cachedReads.get(key);
            if (cached) {
              if (current())
                startTransition(() =>
                  setResults((values) =>
                    !current() || values.get(recording.id) === cached.data
                      ? values
                      : new Map(values).set(recording.id, cached.data),
                  ),
                );
              continue;
            }
            const [ai, activity, visual] = await Promise.all([
              getRecordingAiProcessingState(input),
              getRecordingActivity(input),
              getRecordingVisualObservations({
                ...input,
                includeOcrRegions: false,
              }),
            ]);
            const frames: OcrTimelineFrame[] = [];
            let cursor: OcrHistoryCursor | null = null;
            const cursors = new Set<string>();
            if (!resultRange.bucketMs)
              do {
                const page = await getOcrTimelineFrames({
                  ...input,
                  cursor,
                  limit: 1000,
                  startTimestampMs: startOffsetMs,
                  endTimestampMs: endOffsetMs,
                });
                if (!current()) break;
                frames.push(...page.frames);
                cursor = page.nextCursor;
                if (cursor) {
                  const key = JSON.stringify(cursor);
                  if (cursors.has(key)) throw new Error('Repeated OCR cursor');
                  cursors.add(key);
                }
              } while (cursor);
            if (!current()) continue;
            setFailedIds((ids) => {
              const next = new Set(ids);
              next.delete(recording.id);
              return next;
            });
            const next = {
              request: resultRange,
              ai: visibleAiProcessingState(recording, ai),
              activity: visibleActivity(recording, activity),
              frames: buildStudioVisualFrames(
                visibleVisualObservations(recording, visual),
                recording,
                visibleOcrTimelineFrames(recording, frames),
              ),
            };
            cachedReads.set(key, { recordingId: recording.id, data: next });
            retryReads.delete(recording.id);
            if (!retryReads.size && !eventsUnavailable) setError(false);
            while (cachedReads.size > 32) {
              const first = cachedReads.keys().next().value;
              if (first === undefined) break;
              cachedReads.delete(first);
            }
            startTransition(() =>
              setResults((values) =>
                current() ? new Map(values).set(recording.id, next) : values,
              ),
            );
          } catch {
            if (current()) {
              retryReads.add(recording.id);
              setError(true);
              setFailedIds((ids) => new Set(ids).add(recording.id));
            }
          }
        }
      }
      await Promise.all([worker(), worker()]);
    }
    setError(false);
    setFailedIds(new Set());
    void load(
      new Set(model.manifest.recordings.map((recording) => recording.id)),
    );
    function queueRead(id: string) {
      generations.set(id, (generations.get(id) ?? 0) + 1);
      for (const [key, read] of cachedReads)
        if (read.recordingId === id) cachedReads.delete(key);
      pending.add(id);
      if (timer) return;
      timer = window.setTimeout(() => {
        timer = 0;
        const ids = new Set(pending);
        pending.clear();
        void load(ids);
      }, 250);
    }
    function reconcile() {
      const ids = new Set(retryReads);
      if (eventsUnavailable)
        for (const recording of model.manifest.recordings)
          ids.add(recording.id);
      for (const read of cachedReads.values()) {
        if (
          read.data.ai?.job?.status === 'queued' ||
          read.data.ai?.job?.status === 'running' ||
          read.data.ai?.chunks.some(
            (chunk) => chunk.status === 'queued' || chunk.status === 'running',
          )
        )
          ids.add(read.recordingId);
      }
      for (const id of ids) queueRead(id);
    }
    const fallback = window.setInterval(reconcile, 5000);
    const focus = () => {
      for (const recording of model.manifest.recordings)
        queueRead(recording.id);
    };
    window.addEventListener('focus', focus);
    void listenToAiProcessingChanged((event) => {
      if (!active) return;
      if ('sourceKind' in event && event.sourceKind === 'waveform') return;
      // A shared task/model event does not invalidate this day's recordings.
      const id = event.recordingId;
      if (
        !id ||
        !model.manifest.recordings.some((recording) => recording.id === id)
      )
        return;
      queueRead(id);
    })
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => {
        if (active) {
          eventsUnavailable = true;
          setError(true);
          for (const recording of model.manifest.recordings)
            retryReads.add(recording.id);
        }
      });
    return () => {
      active = false;
      clearTimeout(timer);
      window.clearInterval(fallback);
      window.removeEventListener('focus', focus);
      unlisten?.();
    };
  }, [model, revision, resultRange, cachedReads]);
  const visible = useMemo(() => {
    const next = new Map<string, DayRecordingResults>();
    for (const recording of model.manifest.recordings) {
      const result = results.get(recording.id);
      if (!result) continue;
      if (!recording.redactions.length) {
        next.set(recording.id, result);
        continue;
      }
      next.set(recording.id, {
        ...result,
        ai: visibleAiProcessingState(recording, result.ai),
        activity: visibleActivity(recording, result.activity),
        frames: result.frames.filter((frame) => {
          const supportStart = frame.vlmObservations.reduce(
            (start, observation) =>
              Math.min(
                start,
                observation.contextStartMs ?? observation.frameTimestampMs,
              ),
            frame.frameTimestampMs,
          );
          return !recording.redactions.some(
            (range) =>
              (range.trackIds.length === 0 ||
                !frame.sourceTrackId ||
                range.trackIds.includes(frame.sourceTrackId)) &&
              supportStart < range.endOffsetMs &&
              frame.endTimestampMs > range.startOffsetMs,
          );
        }),
      });
    }
    return next;
  }, [model, results]);
  return { results: visible, error, reload, resultRange, failedIds };
}
