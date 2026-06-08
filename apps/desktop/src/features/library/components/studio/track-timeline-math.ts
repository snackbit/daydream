export const INITIAL_TRACK_WINDOW_MS = 180_000;
export const MIN_TRACK_WINDOW_MS = 10_000;
export const TRACK_LABEL_WIDTH_PX = 160;
export const TRACK_RULER_HEIGHT_PX = 32;
export const TRACK_LANE_HEIGHT_PX = 56;
export const TRACK_ZOOM_STEP = 1.25;
export const TRACK_FOLLOW_EDGE_PX = 48;
export const DEFAULT_TRACK_VIEWPORT_WIDTH_PX = 800;
export const RULER_TARGET_TICK_PX = 90;

export interface RulerTick {
  leftPx: number;
  major: boolean;
  timeMs: number;
}

export function clampTimelineMs(
  timelineMs: number,
  totalDurationMs: number,
): number {
  return Math.max(0, Math.min(totalDurationMs, timelineMs));
}

export function minVisibleWindowMs(totalDurationMs: number): number {
  return Math.min(Math.max(1, totalDurationMs), MIN_TRACK_WINDOW_MS);
}

export function clampVisibleWindowMs(
  visibleWindowMs: number,
  totalDurationMs: number,
): number {
  const minimum = minVisibleWindowMs(totalDurationMs);
  return Math.min(
    Math.max(minimum, visibleWindowMs),
    Math.max(minimum, totalDurationMs),
  );
}

export function clampScrollLeft(
  scrollLeft: number,
  viewport: HTMLDivElement,
): number {
  return Math.max(
    0,
    Math.min(
      scrollLeft,
      Math.max(0, viewport.scrollWidth - viewport.clientWidth),
    ),
  );
}

export function buildRulerTicks({
  scrollLeft,
  timelineViewportWidth,
  timelineWidthPx,
  totalDurationMs,
  visibleWindowMs,
}: {
  scrollLeft: number;
  timelineViewportWidth: number;
  timelineWidthPx: number;
  totalDurationMs: number;
  visibleWindowMs: number;
}): RulerTick[] {
  const majorIntervalMs = chooseRulerIntervalMs(
    (visibleWindowMs / timelineViewportWidth) * RULER_TARGET_TICK_PX,
  );
  const minorIntervalMs = Math.max(1, majorIntervalMs / 4);
  const visibleStartMs = clampTimelineMs(
    (scrollLeft / timelineWidthPx) * totalDurationMs,
    totalDurationMs,
  );
  const visibleEndMs = clampTimelineMs(
    ((scrollLeft + timelineViewportWidth) / timelineWidthPx) * totalDurationMs,
    totalDurationMs,
  );
  const firstTickMs =
    Math.floor(
      Math.max(0, visibleStartMs - majorIntervalMs) / minorIntervalMs,
    ) * minorIntervalMs;
  const lastTickMs = Math.min(totalDurationMs, visibleEndMs + majorIntervalMs);
  const ticks: RulerTick[] = [];

  for (
    let timeMs = firstTickMs;
    timeMs <= lastTickMs;
    timeMs += minorIntervalMs
  ) {
    const major = Math.round(timeMs / minorIntervalMs) % 4 === 0;
    ticks.push({
      leftPx: (timeMs / totalDurationMs) * timelineWidthPx,
      major,
      timeMs,
    });
  }

  return ticks;
}

function chooseRulerIntervalMs(targetIntervalMs: number): number {
  const intervals = [
    1_000, 2_000, 5_000, 10_000, 15_000, 30_000, 60_000, 120_000, 300_000,
    600_000, 900_000, 1_800_000, 3_600_000,
  ];

  return (
    intervals.find((interval) => interval >= targetIntervalMs) ??
    intervals[intervals.length - 1]!
  );
}
