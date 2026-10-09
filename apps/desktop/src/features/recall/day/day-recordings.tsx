import { Link } from '@tanstack/react-router';
import type { RecallDayRecording } from '@daydream/shared';
import { Button } from '@/components/ui/button';
import { recallTime, recordedDuration } from '../recall-format';

export interface DayRecordingsProps {
  recordings: RecallDayRecording[];
}
export function DayRecordings({ recordings }: DayRecordingsProps) {
  return (
    <section aria-label="Recordings" className="grid gap-3">
      <h2 className="text-lg">Recordings</h2>
      {recordings.length === 0 ? (
        <p className="text-muted-foreground text-sm">
          No recordings for this day.
        </p>
      ) : (
        recordings.map((recording) => (
          <article
            key={recording.recordingId}
            className="flex flex-wrap items-center justify-between gap-4 rounded-xl border p-4"
          >
            <div>
              <h3 className="font-medium">
                {recording.title ?? 'Recording'} ·{' '}
                {recallTime(recording.startMs)}
              </h3>
              <p className="text-muted-foreground mt-1 text-xs">
                {recordedDuration(recording.durationMs)} ·{' '}
                {recording.availability === 'hidden'
                  ? 'Privacy hidden'
                  : recording.availability === 'partial'
                    ? 'Some media unavailable'
                    : recording.availability === 'unavailable'
                      ? 'Media unavailable'
                      : 'Available'}{' '}
                · {recording.processingStatus ?? 'Not processed'}
              </p>
            </div>
            <Button asChild variant="outline" size="sm">
              <Link
                to="/library/$recordingId"
                params={{ recordingId: recording.recordingId }}
                search={{
                  seek: recording.startOffsetMs,
                  evidenceId: null,
                  source: null,
                }}
              >
                Open recording
              </Link>
            </Button>
          </article>
        ))
      )}
    </section>
  );
}
