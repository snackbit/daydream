import { useMemo } from 'react';
import { create } from 'zustand';

export const STUDIO_OVERVIEW_PIXELS_PER_MS = 0.01;
export const STUDIO_REVIEW_STEP_MS = 30_000;

export interface StudioResultRange {
  ready?: boolean;
  startOffsetMs: number;
  endOffsetMs: number;
  bucketMs?: number;
  reviewStartOffsetMs?: number;
  reviewEndOffsetMs?: number;
}
interface StudioResultWindowState {
  reviewActive: boolean;
  setReviewActive: (owner: string, active: boolean) => void;
  owner: string | null;
  range: StudioResultRange;
  reviewOwner: string | null;
  review: StudioResultRange;
  setReview: (owner: string, time: number) => void;
  setWindow: (
    owner: string,
    start: number,
    end: number,
    pixelsPerMs?: number,
  ) => void;
}
export const INITIAL_STUDIO_RESULT_RANGE: StudioResultRange = {
  startOffsetMs: 0,
  endOffsetMs: 30_000,
};
export const useStudioResultWindowStore = create<StudioResultWindowState>(
  (set, get) => ({
    reviewActive: false,
    setReviewActive(owner, active) {
      if (get().reviewOwner === owner && get().reviewActive === active) return;
      set({ reviewOwner: owner, reviewActive: active });
    },
    owner: null,
    range: INITIAL_STUDIO_RESULT_RANGE,
    reviewOwner: null,
    review: INITIAL_STUDIO_RESULT_RANGE,
    setReview(owner, time) {
      const from = Math.max(
        0,
        Math.floor(time / STUDIO_REVIEW_STEP_MS) * STUDIO_REVIEW_STEP_MS -
          STUDIO_REVIEW_STEP_MS / 2,
      );
      const to = from + STUDIO_REVIEW_STEP_MS * 2;
      if (
        get().reviewOwner === owner &&
        get().reviewActive &&
        get().review.startOffsetMs === from &&
        get().review.endOffsetMs === to
      )
        return;
      set({
        reviewOwner: owner,
        reviewActive: true,
        review: { startOffsetMs: from, endOffsetMs: to },
      });
    },
    setWindow(owner, start, end, pixelsPerMs = 1) {
      const bucket =
        pixelsPerMs <= STUDIO_OVERVIEW_PIXELS_PER_MS
          ? Math.max(5_000, Math.ceil(48 / pixelsPerMs / 5000) * 5000)
          : undefined;
      const from = Math.max(0, Math.floor(start / 30_000) * 30_000);
      const to = Math.max(from + 1, Math.ceil(end / 30_000) * 30_000);
      const current = get();
      if (
        current.owner === owner &&
        current.range.startOffsetMs === from &&
        current.range.endOffsetMs === to &&
        current.range.bucketMs === bucket
      )
        return;
      set({
        owner,
        range: {
          startOffsetMs: from,
          endOffsetMs: to,
          ...(bucket ? { bucketMs: bucket } : {}),
        },
      });
    },
  }),
);
export function useStudioResultRange(owner: string) {
  const range = useStudioResultWindowStore((state) =>
    state.owner === owner ? state.range : INITIAL_STUDIO_RESULT_RANGE,
  );
  const review = useStudioResultWindowStore((state) =>
    state.reviewOwner === owner ? state.review : INITIAL_STUDIO_RESULT_RANGE,
  );
  const ready = useStudioResultWindowStore((state) => state.owner === owner);
  const reviewActive = useStudioResultWindowStore(
    (state) => state.reviewOwner === owner && state.reviewActive,
  );
  return useMemo(
    () => ({
      ...range,
      ready,
      ...(reviewActive
        ? {
            reviewStartOffsetMs: review.startOffsetMs,
            reviewEndOffsetMs: review.endOffsetMs,
          }
        : {}),
    }),
    [range, review, ready, reviewActive],
  );
}
