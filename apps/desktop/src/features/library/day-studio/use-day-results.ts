import { useCallback, useEffect, useState } from 'react';
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
import { buildStudioVisualFrames } from '../components/studio/studio-visual-observations';
import type { DayRecordingResults } from './day-results';
import type { DayTimelineModel } from './day-timeline';

export function useDayResults(model: DayTimelineModel) {
  const [revision, setRevision] = useState(0);
  const reload = useCallback(() => setRevision((value) => value + 1), []);
  const [results, setResults] = useState(
      new Map<string, DayRecordingResults>(),
    ),
    [error, setError] = useState(false);
  useEffect(() => {
    let active = true,
      generation = 0,
      timer = 0,
      unlisten: (() => void) | undefined;
    async function load() {
      const request = ++generation;
      let index = 0;
      setError(false);
      async function worker() {
        while (active && request === generation) {
          const recording = model.manifest.recordings[index++];
          if (!recording) return;
          try {
            const input = { recordingId: recording.id };
            const [ai, activity, visual] = await Promise.all([
              getRecordingAiProcessingState(input),
              getRecordingActivity(input),
              getRecordingVisualObservations(input),
            ]);
            const frames: OcrTimelineFrame[] = [];
            let cursor: OcrHistoryCursor | null = null;
            const cursors = new Set<string>();
            do {
              const page = await getOcrTimelineFrames({
                ...input,
                cursor,
                limit: 1000,
              });
              if (!active || request !== generation) return;
              frames.push(...page.frames);
              cursor = page.nextCursor;
              if (cursor) {
                const key = JSON.stringify(cursor);
                if (cursors.has(key)) throw new Error('Repeated OCR cursor');
                cursors.add(key);
              }
            } while (cursor);
            if (!active || request !== generation) return;
            const next = {
              ai: visibleAiProcessingState(recording, ai),
              activity: visibleActivity(recording, activity),
              frames: buildStudioVisualFrames(
                visibleVisualObservations(recording, visual),
                recording,
                visibleOcrTimelineFrames(recording, frames),
              ),
            };
            setResults((current) => new Map(current).set(recording.id, next));
          } catch {
            if (active && request === generation) setError(true);
          }
        }
      }
      await Promise.all([worker(), worker()]);
    }
    void load();
    void listenToAiProcessingChanged(() => {
      if (!active) return;
      generation++;
      window.clearTimeout(timer);
      setResults(new Map());
      timer = window.setTimeout(() => void load(), 750);
    })
      .then((cleanup) => {
        if (active) unlisten = cleanup;
        else cleanup();
      })
      .catch(() => {
        if (active) {
          generation++;
          setResults(new Map());
          setError(true);
        }
      });
    return () => {
      active = false;
      generation++;
      clearTimeout(timer);
      unlisten?.();
    };
  }, [model, revision]);
  return { results, error, reload };
}
