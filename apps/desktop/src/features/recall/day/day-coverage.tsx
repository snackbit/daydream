import { useMemo } from 'react';
import type { RecallDay } from '@daydream/shared';
import { SharedDisclosure } from '@/components/shared-disclosure';
import { recallTime, recordedDuration } from '../recall-format';
import {
  buildDayCoverageSpans,
  coverageSourceLabel,
  coverageSpanKind,
} from './day-coverage-model';

export interface DayCoverageProps {
  day: RecallDay;
}
export function DayCoverage({ day }: DayCoverageProps) {
  const coverage = useMemo(
    () => buildDayCoverageSpans(day.coverage),
    [day.coverage],
  );
  let start = day.day.endMs;
  let end = day.day.startMs;
  for (const interval of day.coverage) {
    if (interval.kind === 'gap') continue;
    start = Math.min(start, interval.startMs);
    end = Math.max(end, interval.endMs);
  }
  const span = Math.max(1, end - start);
  return (
    <section className="grid gap-5">
      <div className="grid grid-cols-2 gap-6 border-b pb-6 sm:grid-cols-3">
        <div>
          <p className="text-3xl font-light">
            {recordedDuration(
              day.recordings.some((recording) => recording.durationMs === null)
                ? null
                : day.day.capturedDurationMs,
            )}
          </p>
          <p className="text-muted-foreground mt-2 text-xs">Recorded time</p>
        </div>
        <div>
          <p className="text-3xl font-light">{day.sessions.length}</p>
          <p className="text-muted-foreground mt-2 text-xs">Summary sessions</p>
        </div>
        <div>
          <p className="text-3xl font-light">{day.day.redactionCount}</p>
          <p className="text-muted-foreground mt-2 text-xs">Privacy masks</p>
        </div>
      </div>
      <h2 className="text-lg">Recording timeline</h2>
      {end < start ? (
        <p className="text-muted-foreground text-sm">
          No capture coverage is available for this day.
        </p>
      ) : (
        <>
          <div
            className="grid gap-2"
            role="img"
            aria-label="Capture coverage and protected source intervals"
          >
            <div className="text-muted-foreground flex justify-between text-xs">
              <span>{recallTime(start)}</span>
              <span>{recallTime(end)}</span>
            </div>
            <div className="bg-muted relative h-3 overflow-hidden rounded-full">
              {coverage
                .filter(
                  (interval) =>
                    interval.endMs > start && interval.startMs < end,
                )
                .map((interval, index) => (
                  <span
                    key={`${interval.startMs}-${index}`}
                    data-kind={coverageSpanKind(interval)}
                    title={`${recallTime(interval.startMs)}–${recallTime(interval.endMs)}`}
                    className="bg-primary/25 data-[kind=gap]:bg-muted data-[kind=missing]:bg-destructive/40 data-[kind=uninterpreted]:bg-muted-foreground/30 absolute inset-y-0"
                    style={{
                      left: `${((Math.max(start, interval.startMs) - start) / span) * 100}%`,
                      width: `${((Math.min(end, interval.endMs) - Math.max(start, interval.startMs)) / span) * 100}%`,
                    }}
                  />
                ))}
            </div>
            {coverage.some((interval) =>
              interval.sources.some((source) => source.kind === 'redacted'),
            ) ? (
              <>
                <div className="bg-muted relative h-2 overflow-hidden rounded-full">
                  {coverage
                    .filter((interval) =>
                      interval.sources.some(
                        (source) => source.kind === 'redacted',
                      ),
                    )
                    .map((interval, index) => (
                      <span
                        key={`${interval.startMs}-${index}`}
                        className="bg-foreground/60 absolute inset-y-0"
                        style={{
                          left: `${((interval.startMs - start) / span) * 100}%`,
                          width: `${((interval.endMs - interval.startMs) / span) * 100}%`,
                        }}
                      />
                    ))}
                </div>
                <p className="text-muted-foreground text-xs">
                  Protected sources shown separately; other sources may remain
                  eligible.
                </p>
              </>
            ) : null}
          </div>
          <SharedDisclosure label="Inspect coverage intervals">
            <ul className="mt-4 grid gap-2">
              {coverage.map((interval) => (
                <li
                  key={interval.startMs}
                  className="bg-muted flex flex-wrap justify-between gap-3 rounded-lg px-4 py-3 text-xs"
                >
                  <span>
                    {recallTime(interval.startMs)}–{recallTime(interval.endMs)}
                  </span>
                  <span>
                    {interval.sources
                      .map((source) => {
                        const recording = day.recordings.find(
                          (item) => item.recordingId === source.recordingId,
                        );
                        const label = coverageSourceLabel(source);
                        return day.recordings.length > 1 && source.recordingId
                          ? `${recording?.title ?? source.recordingId} · ${label}`
                          : label;
                      })
                      .join(' · ')}
                  </span>
                </li>
              ))}
            </ul>
          </SharedDisclosure>
        </>
      )}
    </section>
  );
}
