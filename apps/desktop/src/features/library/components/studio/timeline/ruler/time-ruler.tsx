import { useMemo } from 'react';
import { formatDuration } from '../../../../library-format';
import {
  type RulerTick,
  TRACK_RULER_HEIGHT_PX,
} from '../interactions/track-timeline-math';

export interface TimeRulerProps {
  ticks: RulerTick[];
  timelineWidthPx: number;
  originMs?: number | undefined;
}

export function TimeRuler({
  ticks,
  timelineWidthPx,
  originMs,
}: TimeRulerProps) {
  const clockFormat = useMemo(
    () =>
      new Intl.DateTimeFormat(undefined, {
        hour: '2-digit',
        minute: '2-digit',
        second: '2-digit',
      }),
    [],
  );
  const leftPx = ticks[0]?.leftPx ?? 0;
  const widthPx = Math.max(
    1,
    Math.min(timelineWidthPx, (ticks.at(-1)?.leftPx ?? 0) + 120) - leftPx,
  );
  return (
    <div
      aria-label="Time ruler"
      className="bg-background relative sticky top-0 z-20 border-b"
      data-time-ruler
      style={{
        height: TRACK_RULER_HEIGHT_PX,
        marginLeft: leftPx,
        width: widthPx,
      }}
    >
      {ticks.map((tick) => (
        <div
          className="border-muted-foreground/40 absolute bottom-0 border-l"
          key={`${tick.timeMs}-${tick.major ? 'major' : 'minor'}`}
          style={{
            height: tick.major ? 22 : 10,
            left: tick.leftPx - leftPx,
          }}
        >
          {tick.major ? (
            <div className="text-muted-foreground mt-0.5 ml-1 text-xs tabular-nums">
              {originMs === undefined
                ? formatDuration(tick.timeMs)
                : clockFormat.format(originMs + tick.timeMs)}
            </div>
          ) : null}
        </div>
      ))}
    </div>
  );
}
