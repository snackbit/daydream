import { DEFAULT_TRACK_VIEWPORT_WIDTH_PX } from './track-timeline-math';

export function horizontalWheelDeltaPx(event: WheelEvent): number {
  const rawDelta = event.deltaX !== 0 ? event.deltaX : event.deltaY;
  if (event.deltaMode === 1) {
    return rawDelta * 16;
  }
  if (event.deltaMode === 2) {
    return rawDelta * DEFAULT_TRACK_VIEWPORT_WIDTH_PX;
  }

  return rawDelta;
}
