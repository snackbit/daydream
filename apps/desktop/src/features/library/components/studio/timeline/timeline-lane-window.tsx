import type { ReactNode } from 'react';

export interface TimelineLaneWindowProps {
  children: ReactNode;
  className: string;
  trackId: string;
  width: number;
  duration: number;
  height: number;
  range?: { startTimelineMs: number; endTimelineMs: number } | undefined;
}
/** Bound painted/clipped surfaces even when the scroll spacer spans many hours.
 * Children retain global timeline coordinates inside the translated window. */
export function TimelineLaneWindow({
  children,
  className,
  trackId,
  width,
  duration,
  height,
  range,
}: TimelineLaneWindowProps) {
  const left = ((range?.startTimelineMs ?? 0) / duration) * width;
  const end = ((range?.endTimelineMs ?? duration) / duration) * width;
  return (
    <div
      className={className}
      data-lane-content
      data-track-id={trackId}
      style={{
        marginLeft: left,
        width: Math.max(1, end - left),
        height,
        contain: 'layout paint',
      }}
    >
      <div className="absolute inset-y-0" style={{ left: -left, width }}>
        {children}
      </div>
    </div>
  );
}
