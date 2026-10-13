import { useEffect, useState } from 'react';
import type {
  GetRecordingTranscriptPreviewInput,
  RecordingTranscriptPreview,
  TranscriptPreviewEntry,
} from '@daydream/shared';
import { formatDuration } from '../../../../library-format';

export type TranscriptPreviewReader = (
  input: GetRecordingTranscriptPreviewInput,
) => Promise<RecordingTranscriptPreview>;
export interface TranscriptGroupPreviewProps {
  trackLabel: string;
  count: number;
  startMs: number;
  endMs: number;
  input?: GetRecordingTranscriptPreviewInput | undefined;
  readPreview?: TranscriptPreviewReader | undefined;
  entries?: readonly TranscriptPreviewEntry[] | undefined;
}

export function TranscriptGroupPreview({
  trackLabel,
  count,
  startMs,
  endMs,
  input,
  readPreview,
  entries,
}: TranscriptGroupPreviewProps) {
  const [result, setResult] = useState<
    | { entries: readonly TranscriptPreviewEntry[]; status: 'ready' }
    | { status: 'loading' | 'error' }
  >({ status: 'loading' });
  const recordingId = input?.recordingId,
    sourceRole = input?.sourceRole;
  useEffect(() => {
    if (!readPreview || recordingId === undefined || sourceRole === undefined)
      return;
    let active = true;
    void readPreview({
      recordingId,
      sourceRole,
      startOffsetMs: startMs,
      endOffsetMs: endMs,
    }).then(
      (data) => {
        if (active) setResult({ status: 'ready', entries: data.entries });
      },
      () => {
        if (active) setResult({ status: 'error' });
      },
    );
    return () => {
      active = false;
    };
  }, [readPreview, recordingId, sourceRole, startMs, endMs]);
  const shown = entries ?? (result.status === 'ready' ? result.entries : null);
  return (
    <div className="space-y-2">
      <p className="font-medium">
        {trackLabel} · {count} transcript {count === 1 ? 'entry' : 'entries'}
      </p>
      <p>
        {formatDuration(startMs)}–{formatDuration(endMs)}
      </p>
      {shown ? (
        shown.length ? (
          shown.slice(0, 3).map((entry) => (
            <p key={entry.id} className="break-words">
              <span className="opacity-70">
                {formatDuration(entry.startOffsetMs)} ·{' '}
              </span>
              {entry.text}
              {entry.truncated ? '…' : ''}
            </p>
          ))
        ) : (
          <p>No transcript preview available.</p>
        )
      ) : (
        <p role="status">
          {result.status === 'error'
            ? 'Preview could not load. Reopen to retry.'
            : 'Loading preview…'}
        </p>
      )}
      <p className="opacity-70">Click to zoom</p>
    </div>
  );
}
